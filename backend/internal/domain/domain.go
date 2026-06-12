package domain

import (
	"encoding/json"
	"fmt"
	"time"

	"github.com/google/uuid"
)

type Tenant struct {
	ID        uuid.UUID  `json:"id" db:"id"`
	Name      string     `json:"name" db:"name"`
	Plan      string     `json:"plan" db:"plan"`
	Settings  JSONMap     `json:"settings" db:"settings"`
	CreatedAt time.Time   `json:"created_at" db:"created_at"`
	UpdatedAt time.Time   `json:"updated_at" db:"updated_at"`
}

type Company struct {
	ID        uuid.UUID  `json:"id" db:"id"`
	TenantID  uuid.UUID  `json:"tenant_id" db:"tenant_id"`
	Name      string     `json:"name" db:"name"`
	Plan      string     `json:"plan" db:"plan"`
	Settings  JSONMap     `json:"settings" db:"settings"`
	CreatedAt time.Time   `json:"created_at" db:"created_at"`
	UpdatedAt time.Time   `json:"updated_at" db:"updated_at"`
}

type CreateCompanyRequest struct {
	TenantID string `json:"tenant_id" validate:"required,uuid"`
	Name     string `json:"name" validate:"required,min=1,max=255"`
	Plan     string `json:"plan" validate:"omitempty,oneof=starter professional enterprise"`
}

type Employee struct {
	ID         uuid.UUID `json:"id" db:"id"`
	CompanyID  uuid.UUID `json:"company_id" db:"company_id"`
	EmployeeID string    `json:"employee_id" db:"employee_id"`
	FirstName  string    `json:"first_name" db:"first_name"`
	LastName   string    `json:"last_name" db:"last_name"`
	Email      *string   `json:"email" db:"email"`
	RoleID     *uuid.UUID `json:"role_id" db:"role_id"`
	Status     string    `json:"status" db:"status"`
	CreatedAt  time.Time `json:"created_at" db:"created_at"`
	UpdatedAt  time.Time `json:"updated_at" db:"updated_at"`
}

type RegisterEmployeeRequest struct {
	FirstName string  `json:"first_name" validate:"required,min=1,max=255"`
	LastName  string  `json:"last_name" validate:"required,min=1,max=255"`
	Email     *string `json:"email" validate:"omitempty,email"`
	RoleID    *string `json:"role_id" validate:"omitempty,uuid"`
}

type Device struct {
	ID               uuid.UUID  `json:"id" db:"id"`
	EmployeeID       uuid.UUID  `json:"employee_id" db:"employee_id"`
	Hostname         *string    `json:"hostname" db:"hostname"`
	OSType           string     `json:"os_type" db:"os_type"`
	OSVersion        *string    `json:"os_version" db:"os_version"`
	AgentVersion     *string    `json:"agent_version" db:"agent_version"`
	MTLSCertSN       *string    `json:"mtls_cert_sn" db:"mtls_cert_sn"`
	Status           string     `json:"status" db:"status"`
	LastHeartbeat    *time.Time `json:"last_heartbeat" db:"last_heartbeat"`
	EnrolledAt       time.Time  `json:"enrolled_at" db:"enrolled_at"`
	Fingerprint      *string    `json:"fingerprint" db:"fingerprint"`
	CPUInfo          *string    `json:"cpu_info" db:"cpu_info"`
	RAMInfo          *string    `json:"ram_info" db:"ram_info"`
	DiskInfo         *string    `json:"disk_info" db:"disk_info"`
	MACAddresses     *string    `json:"mac_addresses" db:"mac_addresses"`
	IPAddresses      *string    `json:"ip_addresses" db:"ip_addresses"`
	ApprovedBy       *uuid.UUID `json:"approved_by" db:"approved_by"`
	ApprovedAt       *time.Time `json:"approved_at" db:"approved_at"`
	CreatedAt        time.Time  `json:"created_at" db:"created_at"`
	UpdatedAt        time.Time  `json:"updated_at" db:"updated_at"`
	ConnectionStatus string     `json:"connection_status" db:"-"`
}

type DeviceSession struct {
	ID            uuid.UUID  `json:"id" db:"id"`
	DeviceID      uuid.UUID  `json:"device_id" db:"device_id"`
	ConnectedAt   time.Time  `json:"connected_at" db:"connected_at"`
	DisconnectedAt *time.Time `json:"disconnected_at" db:"disconnected_at"`
	Status        string     `json:"status" db:"status"`
}

type EnrollmentRequest struct {
	EmployeeID   string `json:"employee_id" validate:"required"`
	CompanyID    string `json:"company_id" validate:"required,uuid"`
	Hostname     string `json:"hostname" validate:"omitempty"`
	OSType       string `json:"os_type" validate:"required,oneof=windows macos linux"`
	OSVersion    string `json:"os_version" validate:"omitempty"`
	Fingerprint  string `json:"fingerprint" validate:"required"`
	CPUInfo      string `json:"cpu_info" validate:"omitempty"`
	RAMInfo      string `json:"ram_info" validate:"omitempty"`
	DiskInfo     string `json:"disk_info" validate:"omitempty"`
	MACAddresses string `json:"mac_addresses" validate:"omitempty"`
	IPAddresses  string `json:"ip_addresses" validate:"omitempty"`
}

type EnrollmentResponse struct {
	DeviceID    uuid.UUID       `json:"device_id"`
	EmployeeID  string          `json:"employee_id"`
	Employee    Employee        `json:"employee"`
	DeviceToken string          `json:"device_token"`
	Status      string          `json:"status"`
	Rules       RuleSetResponse `json:"rules"`
}

type AppClassification struct {
	ID        uuid.UUID `json:"id" db:"id"`
	RoleID    uuid.UUID `json:"role_id" db:"role_id"`
	AppName   string    `json:"app_name" db:"app_name"`
	Category  string    `json:"category" db:"category"`
	CreatedAt time.Time `json:"created_at" db:"created_at"`
}

type Role struct {
	ID                uuid.UUID `json:"id" db:"id"`
	CompanyID         uuid.UUID `json:"company_id" db:"company_id"`
	Name              string    `json:"name" db:"name"`
	Description       string    `json:"description" db:"description"`
	WorkDescription   string    `json:"work_description" db:"work_description"`
	AllowedCategories []string  `json:"allowed_categories" db:"allowed_categories"`
	BlockedCategories []string  `json:"blocked_categories" db:"blocked_categories"`
	CreatedAt         time.Time `json:"created_at" db:"created_at"`
	UpdatedAt         time.Time `json:"updated_at" db:"updated_at"`
}

type CreateRoleRequest struct {
	Name              string   `json:"name" validate:"required,min=1,max=255"`
	Description       string   `json:"description" validate:"omitempty"`
	WorkDescription   string   `json:"work_description" validate:"omitempty"`
	AllowedCategories []string `json:"allowed_categories" validate:"omitempty"`
	BlockedCategories []string `json:"blocked_categories" validate:"omitempty"`
}

type UpdateRoleRequest struct {
	Name              *string  `json:"name" validate:"omitempty,min=1,max=255"`
	Description       *string  `json:"description" validate:"omitempty"`
	WorkDescription   *string  `json:"work_description" validate:"omitempty"`
	AllowedCategories []string `json:"allowed_categories" validate:"omitempty"`
	BlockedCategories []string `json:"blocked_categories" validate:"omitempty"`
}

type AlertRule struct {
	ID           uuid.UUID `json:"id" db:"id"`
	RoleID       uuid.UUID `json:"role_id" db:"role_id"`
	Category     string    `json:"category" db:"category"`
	ThresholdMin int       `json:"threshold_min" db:"threshold_min"`
	PopupType   string    `json:"popup_type" db:"popup_type"`
	CreatedAt    time.Time `json:"created_at" db:"created_at"`
}

type Policy struct {
	ID                uuid.UUID `json:"id" db:"id"`
	TenantID          uuid.UUID `json:"tenant_id" db:"tenant_id"`
	UploadInterval    int       `json:"upload_interval" db:"upload_interval"`
	ScreenshotEnabled bool      `json:"screenshot_enabled" db:"screenshot_enabled"`
	ScreenshotPolicy  string    `json:"screenshot_policy" db:"screenshot_policy"`
	CreatedAt         time.Time `json:"created_at" db:"created_at"`
	UpdatedAt         time.Time `json:"updated_at" db:"updated_at"`
}

type RoleInfo struct {
	ID                uuid.UUID `json:"id"`
	Name              string    `json:"name"`
	Description       string    `json:"description"`
	WorkDescription   string    `json:"work_description"`
	AllowedCategories []string  `json:"allowed_categories"`
	BlockedCategories []string  `json:"blocked_categories"`
}

type RuleSetResponse struct {
	AppClassifications []AppClassification `json:"app_classifications"`
	AlertRules        []AlertRule          `json:"alert_rules"`
	Policy            Policy               `json:"policy"`
	RoleInfo          *RoleInfo            `json:"role_info,omitempty"`
}

type BulkEventRequest struct {
	DeviceID string              `json:"device_id" validate:"required,uuid"`
	Summary  AppUsageSummary      `json:"summary"`
	Metadata []AppUsageEventMeta  `json:"metadata" validate:"min=1"`
}

type AppUsageEntry struct {
	AppName       string  `json:"app_name" validate:"required"`
	DurationSec   float64 `json:"duration_sec"`
	OpenCount     uint64  `json:"open_count"`
	Classification string `json:"classification"`
	Confidence    float64 `json:"confidence"`
}

type AppUsageUpdateRequest struct {
	DeviceID string          `json:"device_id" validate:"required,uuid"`
	Apps     []AppUsageEntry `json:"apps" validate:"min=1"`
}

type AppUsageSummary struct {
	DeviceID            string    `json:"device_id"`
	AppName             string    `json:"app_name"`
	Date                *string   `json:"date,omitempty"`
	TotalDurationSec   float64   `json:"total_duration_sec"`
	SessionCount        int       `json:"session_count"`
	ProductiveDuration  float64   `json:"productive_duration_sec"`
	UnproductiveDuration float64  `json:"unproductive_duration_sec"`
	NeutralDuration     float64   `json:"neutral_duration_sec"`
}

type AppUsageEventMeta struct {
	AppName      string            `json:"app_name"`
	WindowTitle  string            `json:"window_title"`
	ProcessName  string            `json:"process_name"`
	ProcessID    int               `json:"process_id"`
	StartTime    time.Time         `json:"start_time"`
	EndTime      time.Time         `json:"end_time"`
	DurationSec  float64           `json:"duration_sec"`
	Classification string         `json:"classification"`
	Confidence   float64           `json:"confidence"`
	RoleID       *uuid.UUID        `json:"role_id"`
	DeviceID     uuid.UUID         `json:"device_id"`
}

type PriorityEventRequest struct {
	DeviceID      uuid.UUID `json:"device_id" validate:"required"`
	EventType     string    `json:"event_type" validate:"required"`
	AppName       string    `json:"app_name"`
	WindowTitle   string    `json:"window_title"`
	Classification string  `json:"classification"`
	Confidence    float64   `json:"confidence"`
	Timestamp     time.Time `json:"timestamp"`
}

type PopupEvent struct {
	DeviceID    uuid.UUID  `json:"device_id" validate:"required"`
	AlertID     *uuid.UUID `json:"alert_id"`
	Decision    string     `json:"decision"`
	AppName     string     `json:"app_name"`
	WindowTitle string     `json:"window_title"`
	Explanation string     `json:"explanation" validate:"required"`
	PopupType   string     `json:"popup_type" validate:"required,oneof=toast modal soft_block"`
	Classification string  `json:"classification"`
	Confidence  float64    `json:"confidence"`
	Timestamp   time.Time  `json:"timestamp"`
}

type ScreenshotRequest struct {
	DeviceID    uuid.UUID `json:"device_id" validate:"required,uuid"`
	RequestedBy uuid.UUID `json:"requested_by" validate:"required,uuid"`
	Reason      string    `json:"reason" validate:"required,min=10"`
	Policy      string    `json:"policy" validate:"required,oneof=upload_image metadata_only"`
}

type ScreenshotUpload struct {
	RequestID   uuid.UUID `json:"request_id" validate:"required,uuid"`
	DeviceID    uuid.UUID `json:"device_id" validate:"required,uuid"`
	Category    string    `json:"category"`
	Confidence  float64   `json:"confidence"`
	ImageURL    *string   `json:"image_url"`
	UploadedAt  time.Time `json:"uploaded_at"`
}

type ScreenshotRequestDB struct {
	ID          uuid.UUID  `json:"id" db:"id"`
	DeviceID    uuid.UUID  `json:"device_id" db:"device_id"`
	RequestedBy uuid.UUID  `json:"requested_by" db:"requested_by"`
	Reason      string     `json:"reason" db:"reason"`
	Policy      string     `json:"policy" db:"policy"`
	Status      string     `json:"status" db:"status"`
	ImagePath   *string    `json:"image_path" db:"image_path"`
	ScheduleID  *uuid.UUID `json:"schedule_id" db:"schedule_id"`
	CreatedAt   time.Time  `json:"created_at" db:"created_at"`
	CompletedAt *time.Time `json:"completed_at" db:"completed_at"`
}

type TargetedScreenshotSchedule struct {
	CompanyID      uuid.UUID  `json:"company_id" validate:"required,uuid"`
	EmployeeID     uuid.UUID  `json:"employee_id" validate:"required,uuid"`
	CreatedBy      uuid.UUID  `json:"created_by" validate:"required,uuid"`
	Name           string     `json:"name"`
	IntervalMinutes int       `json:"interval_minutes" validate:"required,min=1"`
	StartTime      string     `json:"start_time" validate:"required"` // HH:MM
	EndTime        string     `json:"end_time" validate:"required"`   // HH:MM
	StartDate      time.Time  `json:"start_date" validate:"required"`
	EndDate        time.Time  `json:"end_date" validate:"required"`
	Status         string     `json:"status" validate:"required,oneof=active paused expired"`
}

type TargetedScreenshotScheduleDB struct {
	ID               uuid.UUID  `json:"id" db:"id"`
	CompanyID        uuid.UUID  `json:"company_id" db:"company_id"`
	EmployeeID       uuid.UUID  `json:"employee_id" db:"employee_id"`
	CreatedBy        uuid.UUID  `json:"created_by" db:"created_by"`
	Name             string     `json:"name" db:"name"`
	IntervalMinutes  int        `json:"interval_minutes" db:"interval_minutes"`
	CronExpression   *string    `json:"cron_expression,omitempty" db:"cron_expression"`
	StartTime        string     `json:"start_time" db:"start_time"`
	EndTime          string     `json:"end_time" db:"end_time"`
	StartDate        time.Time  `json:"start_date" db:"start_date"`
	EndDate          time.Time  `json:"end_date" db:"end_date"`
	Status           string     `json:"status" db:"status"`
	LastTriggeredAt  *time.Time `json:"last_triggered_at" db:"last_triggered_at"`
	CreatedAt        time.Time  `json:"created_at" db:"created_at"`
	UpdatedAt        time.Time  `json:"updated_at" db:"updated_at"`
}

type PendingCommandDB struct {
	ID          uuid.UUID       `json:"id" db:"id"`
	DeviceID    uuid.UUID       `json:"device_id" db:"device_id"`
	CommandType string          `json:"command_type" db:"command_type"`
	Payload     json.RawMessage `json:"payload" db:"payload"`
	Status      string          `json:"status" db:"status"`
	CreatedAt   time.Time       `json:"created_at" db:"created_at"`
	SentAt      *time.Time      `json:"sent_at" db:"sent_at"`
	AckedAt     *time.Time      `json:"acked_at" db:"acked_at"`
}

// InstallToken represents a permanent auth token for agent installation and API access.
type InstallToken struct {
	ID          uuid.UUID   `json:"id" db:"id"`
	Token       string      `json:"token" db:"token"`
	EmployeeID  uuid.UUID   `json:"employee_id" db:"employee_id"`
	CompanyID   uuid.UUID   `json:"company_id" db:"company_id"`
	Description string      `json:"description" db:"description"`
	ExpiresAt   *time.Time  `json:"expires_at" db:"expires_at"`
	CreatedBy   uuid.UUID   `json:"created_by" db:"created_by"`
	CreatedAt   time.Time   `json:"created_at" db:"created_at"`
	RevokedAt   *time.Time  `json:"revoked_at" db:"revoked_at"`
}

type CreateInstallTokenRequest struct {
	EmployeeID  string  `json:"employee_id" validate:"required,uuid"`
	CompanyID   string  `json:"company_id" validate:"required,uuid"`
	Description string  `json:"description" validate:"omitempty"`
	ExpiresIn   *string `json:"expires_in" validate:"omitempty"`
}

type InstallTokenResponse struct {
	ID          uuid.UUID  `json:"id"`
	Token       string     `json:"token"`
	InstallCmd  string     `json:"install_cmd"`
	WindowsCmd  string     `json:"windows_cmd"`
	EmployeeID  uuid.UUID  `json:"employee_id"`
	CompanyID   uuid.UUID  `json:"company_id"`
	Description string     `json:"description"`
	ExpiresAt   *time.Time `json:"expires_at"`
	CreatedAt   time.Time  `json:"created_at"`
	RevokedAt   *time.Time `json:"revoked_at"`
}

type TokenEnrollRequest struct {
	InstallToken string `json:"install_token" validate:"required"`
	Hostname     string `json:"hostname" validate:"omitempty"`
	OSType       string `json:"os_type" validate:"required,oneof=windows macos linux"`
	OSVersion    string `json:"os_version" validate:"omitempty"`
	Fingerprint  string `json:"fingerprint" validate:"required"`
	CPUInfo      string `json:"cpu_info" validate:"omitempty"`
	RAMInfo      string `json:"ram_info" validate:"omitempty"`
	DiskInfo     string `json:"disk_info" validate:"omitempty"`
	MACAddresses string `json:"mac_addresses" validate:"omitempty"`
	IPAddresses  string `json:"ip_addresses" validate:"omitempty"`
}

type InstallTokenClaims struct {
	EmployeeID string
	CompanyID  string
	Role       string
}

// ── LLM Models ────────────────────────────────────────────────────────────

// LLMModelFile describes a single downloadable model file (e.g. a GGUF file).
type LLMModelFile struct {
	Filename    string `json:"filename"`
	DownloadURL string `json:"download_url"`
	FileSizeMB  int64  `json:"file_size_mb"`
	SHA256      string `json:"sha256,omitempty"`
}

// LLMModelParameters holds the llama.cpp inference parameters the agent
// should use when loading this model.
type LLMModelParameters struct {
	NCtx       uint32 `json:"n_ctx"`
	NThreads   uint32 `json:"n_threads"`
	NGpuLayers uint32 `json:"n_gpu_layers"`
}

// LLMModel is the top-level representation of an LLM the agent can download.
type LLMModel struct {
	ID          string             `json:"id"`
	Name        string             `json:"name"`
	Description string             `json:"description"`
	Provider    string             `json:"provider"`
	Version     string             `json:"version"`
	Files       []LLMModelFile    `json:"files"`
	Parameters  LLMModelParameters `json:"parameters"`
}

// LLMModelsResponse is the JSON envelope returned by the /v1/models endpoint.
type LLMModelsResponse struct {
	Models []LLMModel `json:"models"`
}

// JSONMap is a helper type for JSONB columns.
type JSONMap map[string]interface{}

// GenerateEmployeeID creates a sequential employee ID in the format EMP-XXXX.
func GenerateEmployeeID(sequence int) string {
	return fmt.Sprintf("EMP-%04d", sequence)
}

// ActivitySummary represents an AI-generated summary of a 5-minute activity window
type ActivitySummary struct {
	ID              uuid.UUID `json:"id" db:"id"`
	DeviceID        uuid.UUID `json:"device_id" db:"device_id"`
	WindowStart     time.Time `json:"window_start" db:"window_start"`
	WindowEnd       time.Time `json:"window_end" db:"window_end"`
	SummaryText     string    `json:"summary_text" db:"summary_text"`
	TopApps         []string  `json:"top_apps" db:"top_apps"`
	ScreenshotCount int       `json:"screenshot_count" db:"screenshot_count"`
	CreatedAt       time.Time `json:"created_at" db:"created_at"`
}

// BulkActivitySummaryRequest is the payload for batch activity summary uploads
type BulkActivitySummaryRequest struct {
	DeviceID  string            `json:"device_id" validate:"required"`
	Summaries []ActivitySummary `json:"summaries" validate:"min=1"`
}

// InstalledApp represents a single installed application on a device.
type InstalledApp struct {
	ID          uuid.UUID  `json:"id" db:"id"`
	DeviceID    uuid.UUID  `json:"device_id" db:"device_id"`
	AppName     string     `json:"app_name" db:"app_name"`
	DisplayName string     `json:"display_name" db:"display_name"`
	Publisher   string     `json:"publisher" db:"publisher"`
	InstallPath *string    `json:"install_path" db:"install_path"`
	Category    string     `json:"category" db:"category"`
	Confidence  float64    `json:"confidence" db:"confidence"`
	Source      string     `json:"source" db:"source"`
	CreatedAt   time.Time  `json:"created_at" db:"created_at"`
	UpdatedAt   time.Time  `json:"updated_at" db:"updated_at"`
}

// InstalledAppEntry is a single app entry in an upload request from the agent.
type InstalledAppEntry struct {
	AppName     string  `json:"app_name" validate:"required"`
	DisplayName string  `json:"display_name"`
	Publisher   string  `json:"publisher"`
	InstallPath *string `json:"install_path,omitempty"`
	Category    string  `json:"category"`
	Confidence  float64 `json:"confidence"`
	Source      string  `json:"source"`
}

// InstalledAppsUploadRequest is the payload for uploading installed apps from an agent.
type InstalledAppsUploadRequest struct {
	DeviceID string             `json:"device_id" validate:"required,uuid"`
	Apps     []InstalledAppEntry `json:"apps" validate:"min=1"`
}

// InstalledAppWithDevice extends InstalledApp with device/employee info for admin queries.
type InstalledAppWithDevice struct {
	InstalledApp
	DeviceIDStr   string  `json:"device_id"`
	DeviceHostname *string `json:"device_hostname"`
	EmployeeID    string  `json:"employee_id"`
	EmployeeName  string  `json:"employee_name"`
}

// ComplianceAlert represents an AI-generated compliance decision for a screenshot.
type ComplianceAlert struct {
	ID           uuid.UUID  `json:"id" db:"id"`
	DeviceID     uuid.UUID  `json:"device_id" db:"device_id"`
	EmployeeID   uuid.UUID  `json:"employee_id" db:"employee_id"`
	ScreenshotID *uuid.UUID `json:"screenshot_id,omitempty" db:"screenshot_id"`
	Decision     string     `json:"decision" db:"decision"`
	Message      string     `json:"message" db:"message"`
	ModelUsed    string     `json:"model_used" db:"model_used"`
	RawResponse  *string    `json:"raw_response,omitempty" db:"raw_response"`
	Status       string     `json:"status" db:"status"`
	CreatedAt    time.Time  `json:"created_at" db:"created_at"`
	DeliveredAt  *time.Time `json:"delivered_at,omitempty" db:"delivered_at"`
	AckedAt      *time.Time `json:"acked_at,omitempty" db:"acked_at"`
}

type InstalledAppValidation struct {
	ID               uuid.UUID  `json:"id" db:"id"`
	DeviceID         uuid.UUID  `json:"device_id" db:"device_id"`
	AppName          string     `json:"app_name" db:"app_name"`
	RoleID           *uuid.UUID `json:"role_id,omitempty" db:"role_id"`
	DisplayName      string     `json:"display_name" db:"display_name"`
	AgentCategory    string     `json:"agent_category" db:"agent_category"`
	ValidatedCategory string    `json:"validated_category" db:"validated_category"`
	IsCompliant      bool       `json:"is_compliant" db:"is_compliant"`
	Reason           string     `json:"reason" db:"reason"`
	ValidatedAt      time.Time  `json:"validated_at" db:"validated_at"`
	CreatedAt        time.Time  `json:"created_at" db:"created_at"`
	UpdatedAt        time.Time  `json:"updated_at" db:"updated_at"`
}

type ComplianceAlertWithDetails struct {
	ComplianceAlert
	EmployeeName   string        `json:"employee_name"`
	DeviceHostname string        `json:"device_hostname"`
	PopupAnswer    *PopupEventDB `json:"popup_answer,omitempty"`
}

type PopupEventDB struct {
	ID             uuid.UUID  `json:"id" db:"id"`
	DeviceID       uuid.UUID  `json:"device_id" db:"device_id"`
	AlertID        *uuid.UUID `json:"alert_id" db:"alert_id"`
	Decision       string     `json:"decision" db:"decision"`
	AppName        string     `json:"app_name" db:"app_name"`
	WindowTitle    string     `json:"window_title" db:"window_title"`
	Explanation    string     `json:"explanation" db:"explanation"`
	PopupType      string     `json:"popup_type" db:"popup_type"`
	Classification string     `json:"classification" db:"classification"`
	Confidence     float64    `json:"confidence" db:"confidence"`
	EventTime      time.Time  `json:"event_time" db:"event_time"`
	CreatedAt      time.Time  `json:"created_at" db:"created_at"`
}

type InstalledAppValidationWithDetails struct {
	InstalledAppValidation
	DeviceHostname string `json:"device_hostname"`
	EmployeeID     string `json:"employee_id"`
	EmployeeName   string `json:"employee_name"`
	RoleName       string `json:"role_name,omitempty"`
}

// ── Project & Assignments ────────────────────────────────────────────────────

type Project struct {
	ID          uuid.UUID `json:"id" db:"id"`
	CompanyID   uuid.UUID `json:"company_id" db:"company_id"`
	Name        string    `json:"name" db:"name"`
	Description *string   `json:"description,omitempty" db:"description"`
	Status      string    `json:"status" db:"status"`
	WorkingApps []string  `json:"working_apps" db:"working_apps"`
	CreatedAt   time.Time `json:"created_at" db:"created_at"`
	UpdatedAt   time.Time `json:"updated_at" db:"updated_at"`
}

type EmployeeProjectAssignment struct {
	ID         uuid.UUID  `json:"id" db:"id"`
	EmployeeID uuid.UUID  `json:"employee_id" db:"employee_id"`
	ProjectID  uuid.UUID  `json:"project_id" db:"project_id"`
	AssignedBy uuid.UUID  `json:"assigned_by" db:"assigned_by"`
	StartedAt  time.Time  `json:"started_at" db:"started_at"`
	EndedAt    *time.Time `json:"ended_at,omitempty" db:"ended_at"`
	IsPrimary  bool       `json:"is_primary" db:"is_primary"`
	CreatedAt  time.Time  `json:"created_at" db:"created_at"`
	UpdatedAt  time.Time  `json:"updated_at" db:"updated_at"`
}

// ── AI Activity Analysis ─────────────────────────────────────────────────────

type ActivityAnalysisRequest struct {
	EmployeeID string    `json:"employee_id" validate:"required,uuid"`
	StartTime  string    `json:"start_time" validate:"required"`
	EndTime    string    `json:"end_time" validate:"required"`
}

type AppUsageEventForAnalysis struct {
	AppName     string  `json:"app_name"`
	WindowTitle string  `json:"window_title"`
	DurationSec float64 `json:"duration_sec"`
}

type ActivityAnalysisResult struct {
	EmployeeID           string                      `json:"employee_id"`
	EmployeeName         string                      `json:"employee_name"`
	ProjectName          string                      `json:"project_name,omitempty"`
	WorkingApps          []string                    `json:"working_apps"`
	TotalDurationSec     float64                     `json:"total_duration_sec"`
	ProductiveDuration   float64                     `json:"productive_duration_sec"`
	UnproductiveDuration float64                    `json:"unproductive_duration_sec"`
	NeutralDuration      float64                     `json:"neutral_duration_sec"`
	ProductivePct        float64                     `json:"productive_pct"`
	UnproductivePct      float64                     `json:"unproductive_pct"`
	NeutralPct           float64                     `json:"neutral_pct"`
	Events               []ActivityEventClassification `json:"events"`
	AIModel              string                      `json:"ai_model"`
	AnalyzedAt           time.Time                   `json:"analyzed_at"`
	StartTime            time.Time                   `json:"start_time"`
	EndTime              time.Time                   `json:"end_time"`
}

type ActivityEventClassification struct {
	AppName      string  `json:"app_name"`
	WindowTitle  string  `json:"window_title"`
	DurationSec  float64 `json:"duration_sec"`
	Classification string `json:"classification"` // productive, unproductive, neutral
	Reason        string  `json:"reason"`
}