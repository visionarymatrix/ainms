package service

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/ollama"
	"github.com/ainms/gateway/internal/repository/clickhouse"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/google/uuid"
)

// ComplianceService orchestrates AI compliance analysis via Ollama Cloud.
type ComplianceService struct {
	alertRepo         *postgres.ComplianceAlertRepo
	screenshotRepo    *postgres.ScreenshotRepo
	screenshotService *ScreenshotService
	deviceRepo        *postgres.DeviceRepo
	employeeRepo      *postgres.EmployeeRepo
	eventRepo         *clickhouse.EventRepo
	analysisCacheRepo *postgres.ActivityAnalysisRepo
	ollamaClient      *ollama.Client
	uploadDir         string
}

// NewComplianceService creates a new compliance service.
func NewComplianceService(
	alertRepo *postgres.ComplianceAlertRepo,
	screenshotRepo *postgres.ScreenshotRepo,
	screenshotService *ScreenshotService,
	deviceRepo *postgres.DeviceRepo,
	employeeRepo *postgres.EmployeeRepo,
	eventRepo *clickhouse.EventRepo,
	analysisCacheRepo *postgres.ActivityAnalysisRepo,
	ollamaClient *ollama.Client,
	uploadDir string,
) *ComplianceService {
	return &ComplianceService{
		alertRepo:         alertRepo,
		screenshotRepo:    screenshotRepo,
		screenshotService: screenshotService,
		deviceRepo:        deviceRepo,
		employeeRepo:      employeeRepo,
		eventRepo:         eventRepo,
		analysisCacheRepo: analysisCacheRepo,
		ollamaClient:      ollamaClient,
		uploadDir:         uploadDir,
	}
}

const complianceSystemPrompt = `You are a workplace compliance monitor. You receive multiple screenshots taken at specific times during a monitoring window, along with app usage data. Decide if the user is doing productive work or violating company policy.

Rules:
- Personal shopping, entertainment, social media for non-work purposes = violation
- Coding, documentation, work tools, email = productive
- Context matters: judge based on what the user is actually viewing/doing, not just the app name

Context-aware rules:
- YouTube: if the video is a tutorial, tech talk, educational content, or work-related training → productive. If it's movies, music videos, gaming streams → violation
- Shopping sites: if browsing work equipment, software licenses, office supplies → productive. If personal shopping → violation
- Social media: if posting work-related content, reading industry news, professional networking → productive. If casual browsing, personal posts → violation
- News sites: if reading industry/tech news → productive. If entertainment/gossip → violation

Violation messages MUST:
- Reference the specific screenshot number and time when you observed the violation (e.g., "At 14:32 (Screenshot 3), you were watching Netflix")
- Be specific about WHAT the user was doing, not just the app name
- If multiple screenshots show violations, mention each one with its time

You must respond in exactly this format:

DECISION: violation OR productive
MESSAGE: a natural 1-2 sentence message for the user. Only include a message for violations — for productive decisions respond with MESSAGE: ok`

// AnalyzeScreenshot runs AI compliance analysis on a saved screenshot.
func (s *ComplianceService) AnalyzeScreenshot(ctx context.Context, screenshotID uuid.UUID, deviceID uuid.UUID, windowTitle, appName string) (*domain.ComplianceAlert, error) {
	// Load screenshot image from disk
	req, err := s.screenshotRepo.GetByID(ctx, screenshotID)
	if err != nil {
		return nil, fmt.Errorf("screenshot not found: %w", err)
	}
	if req.ImagePath == nil || *req.ImagePath == "" {
		return nil, fmt.Errorf("screenshot image not saved")
	}

	imagePath := filepath.Join(s.uploadDir, *req.ImagePath)
	imageData, err := os.ReadFile(imagePath)
	if err != nil {
		return nil, fmt.Errorf("read screenshot image: %w", err)
	}

	// Look up employee + role for context
	device, err := s.deviceRepo.GetByID(ctx, deviceID)
	if err != nil {
		return nil, fmt.Errorf("device not found: %w", err)
	}

	employee, err := s.employeeRepo.GetByID(ctx, device.EmployeeID)
	if err != nil {
		return nil, fmt.Errorf("employee not found: %w", err)
	}

	roleName := "employee"
	if employee.RoleID != nil {
		roleName = employee.RoleID.String()
	}

	userPrompt := fmt.Sprintf(
		"The user's current window title is: '%s'\nThe active application is: '%s'\nThe user's role is: %s\nCaptured at: %s\n\nAnalyze this screenshot and decide if this is productive work. If there is a violation, reference the time in your MESSAGE.",
		windowTitle, appName, roleName, time.Now().Format("15:04:05"),
	)

	// Call Ollama Cloud vision API
	chatCtx, cancel := context.WithTimeout(ctx, 60*time.Second)
	defer cancel()

	resp, err := s.ollamaClient.AnalyzeScreenshot(chatCtx, complianceSystemPrompt, userPrompt, imageData)
	if err != nil {
		return nil, fmt.Errorf("ollama analysis failed: %w", err)
	}

	decision, message := parseComplianceResponse(resp.Message.Content)

	alert := &domain.ComplianceAlert{
		ID:           uuid.New(),
		DeviceID:     deviceID,
		EmployeeID:   device.EmployeeID,
		ScreenshotID: &screenshotID,
		Decision:     decision,
		Message:      message,
		ModelUsed:    resp.Model,
		RawResponse:  stringPtr(resp.Message.Content),
		Status:       "pending",
		CreatedAt:    time.Now(),
	}

	if err := s.alertRepo.Create(ctx, alert); err != nil {
		return nil, fmt.Errorf("save alert: %w", err)
	}

	return alert, nil
}
type ScreenshotMeta struct {
	RequestID   uuid.UUID `json:"request_id"`
	WindowTitle string    `json:"window_title"`
	AppName     string    `json:"app_name"`
	CapturedAt  string    `json:"captured_at"`
}

// AppUsageEntry is app usage data from the agent's activity buffer window.
type AppUsageEntry struct {
	AppName      string  `json:"app_name"`
	DurationSecs float64 `json:"duration_secs"`
	SampleCount  int     `json:"sample_count"`
	WindowTitle  string  `json:"window_title"`
}

// AnalyzeScreenshotsBatch runs AI compliance analysis on multiple screenshots
// with app usage context from the activity buffer window.
func (s *ComplianceService) AnalyzeScreenshotsBatch(ctx context.Context, deviceID uuid.UUID, screenshotIDs []uuid.UUID, imageDataMap map[uuid.UUID][]byte, metas []ScreenshotMeta, appUsage []AppUsageEntry) (*domain.ComplianceAlert, error) {
	device, err := s.deviceRepo.GetByID(ctx, deviceID)
	if err != nil {
		return nil, fmt.Errorf("device not found: %w", err)
	}

	employee, err := s.employeeRepo.GetByID(ctx, device.EmployeeID)
	if err != nil {
		return nil, fmt.Errorf("employee not found: %w", err)
	}

	roleName := "employee"
	if employee.RoleID != nil {
		roleName = employee.RoleID.String()
	}

	// Save all screenshots to disk
	for _, id := range screenshotIDs {
		data, ok := imageDataMap[id]
		if !ok {
			continue
		}
		if err := os.MkdirAll(s.uploadDir, 0755); err != nil {
			return nil, fmt.Errorf("create upload directory: %w", err)
		}
		filename := fmt.Sprintf("%s.png", id.String())
		filePath := filepath.Join(s.uploadDir, filename)
		if err := os.WriteFile(filePath, data, 0644); err != nil {
			return nil, fmt.Errorf("write screenshot file: %w", err)
		}
		if err := s.screenshotRepo.UpdateStatus(ctx, id, "completed", &filename); err != nil {
			log.Printf("[compliance] UpdateStatus for %s failed: %v", id, err)
		}
	}

	// Build app usage summary text
	var appUsageText strings.Builder
	appUsageText.WriteString("APP USAGE DURING MONITORING WINDOW:\n")
	if len(appUsage) == 0 {
		appUsageText.WriteString("  No desktop apps detected.\n")
	} else {
		var totalDur float64
		for _, a := range appUsage {
			totalDur += a.DurationSecs
		}
		for i, a := range appUsage {
			pct := 0.0
			if totalDur > 0 {
				pct = (a.DurationSecs / totalDur) * 100.0
			}
			title := a.WindowTitle
			if len(title) > 60 {
				title = title[:57] + "..."
			}
			appUsageText.WriteString(fmt.Sprintf("%d. %s — %.1fmin (%.0f%%) [last title: %q]\n",
				i+1, a.AppName, a.DurationSecs/60.0, pct, title))
		}
	}

	// Build per-screenshot metadata text
	var metaText strings.Builder
	metaText.WriteString("SCREENSHOT CAPTURES (in chronological order):\n")
	windowStart := ""
	for i, m := range metas {
		capturedAt := m.CapturedAt
		if capturedAt != "" {
			t, err := time.Parse(time.RFC3339, capturedAt)
			if err == nil {
				if windowStart == "" {
					windowStart = t.Format("15:04")
				}
				capturedAt = t.Format("15:04:05")
			}
		}
		if capturedAt == "" {
			metaText.WriteString(fmt.Sprintf("  Screenshot %d: app=%q, window=%q\n", i+1, m.AppName, m.WindowTitle))
		} else {
			metaText.WriteString(fmt.Sprintf("  Screenshot %d [captured at %s]: app=%q, window=%q\n", i+1, capturedAt, m.AppName, m.WindowTitle))
		}
	}

	windowLabel := ""
	if windowStart != "" {
		windowLabel = fmt.Sprintf(" (monitoring window starting at %s)", windowStart)
	}

	userPrompt := fmt.Sprintf(
		"The user's role is: %s%s\n\n%s\n%s\nAnalyze these screenshots combined with the app usage data and decide if this is productive work or a policy violation. If there is a violation, you MUST reference the specific screenshot number and the time it was captured in your MESSAGE.",
		roleName, windowLabel, appUsageText.String(), metaText.String(),
	)

	// Collect all image data in order
	imagesData := make([][]byte, 0, len(screenshotIDs))
	for _, id := range screenshotIDs {
		if data, ok := imageDataMap[id]; ok {
			imagesData = append(imagesData, data)
		}
	}

	chatCtx, cancel := context.WithTimeout(ctx, 120*time.Second)
	defer cancel()

	resp, err := s.ollamaClient.AnalyzeScreenshots(chatCtx, complianceSystemPrompt, userPrompt, imagesData)
	if err != nil {
		return nil, fmt.Errorf("ollama batch analysis failed: %w", err)
	}

	decision, message := parseComplianceResponse(resp.Message.Content)

	var primaryScreenshotID *uuid.UUID
	if len(screenshotIDs) > 0 {
		id := screenshotIDs[len(screenshotIDs)-1]
		primaryScreenshotID = &id
	}

	alert := &domain.ComplianceAlert{
		ID:           uuid.New(),
		DeviceID:     deviceID,
		EmployeeID:   device.EmployeeID,
		ScreenshotID: primaryScreenshotID,
		Decision:     decision,
		Message:      message,
		ModelUsed:    resp.Model,
		RawResponse:  stringPtr(resp.Message.Content),
		Status:       "pending",
		CreatedAt:    time.Now(),
	}

	if err := s.alertRepo.Create(ctx, alert); err != nil {
		return nil, fmt.Errorf("save alert: %w", err)
	}

	return alert, nil
}

// GetPendingAlerts returns pending alerts for a device and marks them delivered.
func (s *ComplianceService) GetPendingAlerts(ctx context.Context, deviceID uuid.UUID) ([]domain.ComplianceAlert, error) {
	alerts, err := s.alertRepo.ListPendingByDevice(ctx, deviceID)
	if err != nil {
		return nil, err
	}

	// Mark all returned alerts as delivered
	for _, alert := range alerts {
		_ = s.alertRepo.MarkDelivered(ctx, alert.ID)
	}

	return alerts, nil
}

// AckAlert marks an alert as acknowledged.
func (s *ComplianceService) AckAlert(ctx context.Context, alertID uuid.UUID) error {
	return s.alertRepo.MarkAcked(ctx, alertID)
}

// parseComplianceResponse extracts DECISION and MESSAGE from the LLM output.
func parseComplianceResponse(raw string) (decision, message string) {
	decisionRe := regexp.MustCompile(`(?i)DECISION:\s*(\S+)`)
	messageRe := regexp.MustCompile(`(?i)MESSAGE:\s*(.+)`)

	if m := decisionRe.FindStringSubmatch(raw); len(m) > 1 {
		decision = strings.ToLower(strings.TrimSpace(m[1]))
	}
	if m := messageRe.FindStringSubmatch(raw); len(m) > 1 {
		message = strings.TrimSpace(m[1])
	}

	if decision == "" {
		decision = "productive"
	}
	if message == "" {
		message = "Activity detected."
	}

	return decision, message
}

func stringPtr(s string) *string { return &s }

func (s *ComplianceService) ListCompanyAlerts(ctx context.Context, companyID uuid.UUID, status string) ([]domain.ComplianceAlertWithDetails, error) {
	alerts, err := s.alertRepo.ListByCompany(ctx, companyID, status)
	if err != nil {
		return nil, fmt.Errorf("list company alerts: %w", err)
	}
	return s.enrichAlerts(ctx, alerts)
}

func (s *ComplianceService) ListEmployeeAlerts(ctx context.Context, employeeID uuid.UUID, status string) ([]domain.ComplianceAlertWithDetails, error) {
	alerts, err := s.alertRepo.ListByEmployee(ctx, employeeID, status)
	if err != nil {
		return nil, fmt.Errorf("list employee alerts: %w", err)
	}
	return s.enrichAlerts(ctx, alerts)
}

const activityAnalysisSystemPrompt = `You are a workplace activity classifier. You receive app usage events for an employee and must classify each one.

## Classification Rules

Given the employee's role and their project's "working_apps" whitelist:

- **productive**: The app/activity directly serves the employee's assigned project, matches a whitelisted working app, or is a general-purpose work tool (terminal, IDE, code browser, documentation).
- **unproductive**: The app/activity is clearly personal or recreational and has no relation to the project or general work duties (social media for leisure, personal shopping, entertainment, gaming).
- **neutral**: The app/activity could be either productive or unproductive depending on context you cannot determine (generic browser tab, communication tool, file explorer).

## Output Format

Return a single JSON object with this EXACT structure. Do NOT wrap it in markdown code fences. Do NOT add any text before or after the JSON. The response must start with { and end with }.

{
  "events": [
    {
      "app_name": "string - exact app_name from input",
      "window_title": "string - exact window_title from input",
      "classification": "productive | unproductive | neutral",
      "reason": "string - one-sentence justification referencing the project or whitelist",
      "duration_sec": number - exact duration_sec from input"
    }
  ]
}

Every input event MUST appear in the output events array. Copy app_name, window_title, and duration_sec verbatim from the input. Only classification and reason are your own judgment.`

func (s *ComplianceService) AnalyzeActivity(ctx context.Context, projectSvc *ProjectService, employeeID uuid.UUID, startTimeStr, endTimeStr string) (*domain.ActivityAnalysisResult, error) {
	fromTime, err := time.Parse(time.RFC3339, startTimeStr)
	if err != nil {
		fromTime, _ = time.Parse("2006-01-02", startTimeStr)
	}
	toTime, err := time.Parse(time.RFC3339, endTimeStr)
	if err != nil {
		toTime, _ = time.Parse("2006-01-02", endTimeStr)
	}

	cached, err := s.analysisCacheRepo.GetCached(ctx, employeeID, fromTime, toTime)
	if err == nil && cached != nil {
		return cached, nil
	}

	employee, err := s.employeeRepo.GetByID(ctx, employeeID)
	if err != nil {
		return nil, fmt.Errorf("employee not found: %w", err)
	}

	devices, err := s.deviceRepo.GetByEmployeeID(ctx, employeeID)
	if err != nil {
		return nil, fmt.Errorf("list employee devices: %w", err)
	}
	if len(devices) == 0 {
		return nil, fmt.Errorf("no devices found for employee")
	}

	var allEvents []domain.AppUsageEventMeta
	for _, device := range devices {
		events, err := s.eventRepo.GetEventsByDevice(ctx, device.ID.String(), 50, &fromTime, &toTime)
		if err != nil {
			continue
		}
		allEvents = append(allEvents, events...)
	}

	assignments, err := projectSvc.ListActiveEmployeeProjects(ctx, employeeID)
	if err != nil {
		return nil, fmt.Errorf("list employee projects: %w", err)
	}

	var projectName string
	var workingApps []string
	var primaryProjectID *uuid.UUID

	for _, a := range assignments {
		if a.IsPrimary || primaryProjectID == nil {
			proj, err := projectSvc.GetProject(ctx, a.ProjectID)
			if err != nil {
				continue
			}
			projectName = proj.Name
			workingApps = proj.WorkingApps
			pid := a.ProjectID
			primaryProjectID = &pid
			if a.IsPrimary {
				break
			}
		}
	}

	roleDescription := "employee"
	if employee.RoleID != nil {
		roleDescription = employee.RoleID.String()
	}

	var eventsJSON strings.Builder
	for i, e := range allEvents {
		if i > 0 {
			eventsJSON.WriteString(",\n")
		}
		eventsJSON.WriteString(fmt.Sprintf(
			`{"app_name":%q,"window_title":%q,"duration_sec":%.0f,"process":%q}`,
			e.AppName, e.WindowTitle, e.DurationSec, e.ProcessName,
		))
	}

	userPrompt := fmt.Sprintf(
		`## Employee Context
- Name: %s %s
- Role: %s
- Active Project: %s
- Working Apps Whitelist: %v
- Monitoring Period: %s to %s

## Input Events (classify ALL of these)
[%s]

Return the JSON object now. Remember: copy app_name, window_title, and duration_sec verbatim from each input event. Only classification and reason are yours.`,
		employee.FirstName, employee.LastName, roleDescription, projectName, workingApps,
		startTimeStr, endTimeStr, eventsJSON.String(),
	)

	chatCtx, cancel := context.WithTimeout(ctx, 120*time.Second)
	defer cancel()

	resp, err := s.ollamaClient.Chat(chatCtx, &ollama.ChatRequest{
		Model: s.ollamaClient.Model(),
		Messages: []ollama.Message{
			{Role: "system", Content: activityAnalysisSystemPrompt},
			{Role: "user", Content: userPrompt},
		},
		Stream: false,
		Options: &ollama.Options{
			Temperature: 0.3,
			NumCtx:     8192,
			NumPredict: 8192,
		},
	})
	if err != nil {
		return nil, fmt.Errorf("ollama activity analysis failed: %w", err)
	}

	content := resp.Message.Content
	if content == "" && resp.Message.Thinking != "" {
		content = resp.Message.Thinking
	}

	content = strings.TrimPrefix(content, "```json")
	content = strings.TrimPrefix(content, "```")
	content = strings.TrimSpace(content)
	if strings.HasSuffix(content, "```") {
		content = content[:len(content)-3]
		content = strings.TrimSpace(content)
	}

	var parsed struct {
		Events []struct {
			AppName        string  `json:"app_name"`
			WindowTitle    string  `json:"window_title"`
			Classification string  `json:"classification"`
			Reason         string  `json:"reason"`
			DurationSec    float64 `json:"duration_sec"`
		} `json:"events"`
	}

	jsonStart := strings.Index(content, `{"events"`)
	if jsonStart == -1 {
		jsonStart = strings.Index(content, "{")
	}
	if jsonStart == -1 {
		log.Printf("[AI-Activity] No JSON found in response (content len=%d, thinking len=%d)", len(resp.Message.Content), len(resp.Message.Thinking))
		return nil, fmt.Errorf("no JSON in AI response")
	}

	jsonStr := content[jsonStart:]

	if err := json.Unmarshal([]byte(jsonStr), &parsed); err != nil {
		lastBrace := strings.LastIndex(jsonStr, "}")
		if lastBrace > 0 {
			if err2 := json.Unmarshal([]byte(jsonStr[:lastBrace+1]), &parsed); err2 != nil {
				return nil, fmt.Errorf("parse AI response: %w", err2)
			}
		} else {
			return nil, fmt.Errorf("parse AI response: %w", err)
		}
	}

	var totalDuration, productiveDuration, unproductiveDuration, neutralDuration float64
	events := make([]domain.ActivityEventClassification, 0, len(parsed.Events))

	for _, e := range parsed.Events {
		classification := strings.ToLower(e.Classification)
		if classification != "productive" && classification != "unproductive" && classification != "neutral" {
			classification = "neutral"
		}

		dur := e.DurationSec
		if dur == 0 {
			for _, orig := range allEvents {
				if orig.AppName == e.AppName && orig.WindowTitle == e.WindowTitle {
					dur = orig.DurationSec
					break
				}
			}
		}

		totalDuration += dur
		switch classification {
		case "productive":
			productiveDuration += dur
		case "unproductive":
			unproductiveDuration += dur
		default:
			neutralDuration += dur
		}

		events = append(events, domain.ActivityEventClassification{
			AppName:        e.AppName,
			WindowTitle:    e.WindowTitle,
			DurationSec:    dur,
			Classification: classification,
			Reason:         e.Reason,
		})
	}

	result := &domain.ActivityAnalysisResult{
		EmployeeID:           employeeID.String(),
		EmployeeName:         employee.FirstName + " " + employee.LastName,
		ProjectName:          projectName,
		WorkingApps:          workingApps,
		TotalDurationSec:     totalDuration,
		ProductiveDuration:   productiveDuration,
		UnproductiveDuration: unproductiveDuration,
		NeutralDuration:      neutralDuration,
		Events:               events,
		AIModel:              resp.Model,
		AnalyzedAt:           time.Now(),
		StartTime:            fromTime,
		EndTime:              toTime,
	}

	if totalDuration > 0 {
		result.ProductivePct = (productiveDuration / totalDuration) * 100
		result.UnproductivePct = (unproductiveDuration / totalDuration) * 100
		result.NeutralPct = (neutralDuration / totalDuration) * 100
	}

	if saveErr := s.analysisCacheRepo.Save(ctx, result); saveErr != nil {
		log.Printf("[compliance] Failed to cache analysis result: %v", saveErr)
	}

	return result, nil
}

func (s *ComplianceService) enrichAlerts(ctx context.Context, alerts []domain.ComplianceAlert) ([]domain.ComplianceAlertWithDetails, error) {
	if len(alerts) == 0 {
		return []domain.ComplianceAlertWithDetails{}, nil
	}

	employeeCache := make(map[uuid.UUID]*domain.Employee)
	deviceCache := make(map[uuid.UUID]*domain.Device)

	var alertIDs []uuid.UUID
	for _, a := range alerts {
		if a.ScreenshotID != nil {
			alertIDs = append(alertIDs, a.ID)
		}
	}

	popupAnswers, err := s.alertRepo.GetPopupAnswersByAlertIDs(ctx, alertIDs)
	if err != nil {
		log.Printf("[compliance] Failed to fetch popup answers: %v", err)
		popupAnswers = map[uuid.UUID]domain.PopupEventDB{}
	}

	results := make([]domain.ComplianceAlertWithDetails, 0, len(alerts))
	for _, a := range alerts {
		detail := domain.ComplianceAlertWithDetails{
			ComplianceAlert: a,
		}

		if emp, ok := employeeCache[a.EmployeeID]; ok {
			detail.EmployeeName = emp.FirstName + " " + emp.LastName
		} else {
			emp, err := s.employeeRepo.GetByID(ctx, a.EmployeeID)
			if err == nil {
				employeeCache[a.EmployeeID] = emp
				detail.EmployeeName = emp.FirstName + " " + emp.LastName
			} else {
				detail.EmployeeName = a.EmployeeID.String()[:8] + "..."
			}
		}

		if dev, ok := deviceCache[a.DeviceID]; ok {
			if dev.Hostname != nil {
				detail.DeviceHostname = *dev.Hostname
			}
		} else {
			dev, err := s.deviceRepo.GetByID(ctx, a.DeviceID)
			if err == nil {
				deviceCache[a.DeviceID] = dev
				if dev.Hostname != nil {
					detail.DeviceHostname = *dev.Hostname
				}
			}
		}

		if pa, ok := popupAnswers[a.ID]; ok {
			detail.PopupAnswer = &pa
		}

		results = append(results, detail)
	}
	return results, nil
}

func truncate(s string, maxLen int) string {
	if len(s) <= maxLen {
		return s
	}
	return s[:maxLen] + "..."
}
