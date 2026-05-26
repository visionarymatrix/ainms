package service

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/google/uuid"
)

type ScreenshotService struct {
	screenshotRepo *postgres.ScreenshotRepo
	commandRepo    *postgres.CommandRepo
	deviceRepo     *postgres.DeviceRepo
	employeeRepo   *postgres.EmployeeRepo
	uploadDir      string
}

func NewScreenshotService(sr *postgres.ScreenshotRepo, cr *postgres.CommandRepo, dr *postgres.DeviceRepo, er *postgres.EmployeeRepo, uploadDir string) *ScreenshotService {
	return &ScreenshotService{
		screenshotRepo: sr,
		commandRepo:    cr,
		deviceRepo:     dr,
		employeeRepo:   er,
		uploadDir:      uploadDir,
	}
}

func (s *ScreenshotService) RequestScreenshot(ctx context.Context, deviceID uuid.UUID, requestedBy uuid.UUID, reason string, policy string) (*domain.ScreenshotRequestDB, error) {
	req := &domain.ScreenshotRequestDB{
		DeviceID:    deviceID,
		RequestedBy: requestedBy,
		Reason:      reason,
		Policy:      policy,
		Status:      "pending",
	}

	if err := s.screenshotRepo.Create(ctx, req); err != nil {
		return nil, fmt.Errorf("create screenshot request: %w", err)
	}

	payload, _ := json.Marshal(map[string]string{
		"request_id": req.ID.String(),
		"device_id":  deviceID.String(),
	})
	cmd := &domain.PendingCommandDB{
		DeviceID:    deviceID,
		CommandType: "screenshot_request",
		Payload:     payload,
		Status:      "pending",
	}
	if err := s.commandRepo.Create(ctx, cmd); err != nil {
		return nil, fmt.Errorf("create pending command: %w", err)
	}

	return req, nil
}

func (s *ScreenshotService) GetPendingCommands(ctx context.Context, deviceID uuid.UUID) ([]domain.PendingCommandDB, error) {
	commands, err := s.commandRepo.ListPendingByDevice(ctx, deviceID)
	if err != nil {
		return nil, err
	}

	for i := range commands {
		if commands[i].Status == "pending" {
			_ = s.commandRepo.MarkSent(ctx, commands[i].ID)
			commands[i].Status = "sent"
			now := time.Now()
			commands[i].SentAt = &now
		}
	}

	return commands, nil
}

func (s *ScreenshotService) AcknowledgeCommand(ctx context.Context, commandID uuid.UUID) error {
	return s.commandRepo.MarkAcked(ctx, commandID)
}

func (s *ScreenshotService) UploadScreenshot(ctx context.Context, requestID uuid.UUID, deviceID uuid.UUID, imageData []byte) (*domain.ScreenshotRequestDB, error) {
	req, err := s.screenshotRepo.GetByID(ctx, requestID)
	if err != nil {
		return nil, fmt.Errorf("screenshot request not found: %w", err)
	}
	if req.DeviceID != deviceID {
		return nil, fmt.Errorf("device_id mismatch for screenshot request")
	}

	if err := os.MkdirAll(s.uploadDir, 0755); err != nil {
		return nil, fmt.Errorf("create upload directory: %w", err)
	}

	filename := fmt.Sprintf("%s.png", requestID.String())
	filePath := filepath.Join(s.uploadDir, filename)
	if err := os.WriteFile(filePath, imageData, 0644); err != nil {
		return nil, fmt.Errorf("write screenshot file: %w", err)
	}

	if err := s.screenshotRepo.UpdateStatus(ctx, requestID, "completed", &filename); err != nil {
		return nil, fmt.Errorf("update screenshot status: %w", err)
	}

	return s.screenshotRepo.GetByID(ctx, requestID)
}

func (s *ScreenshotService) GetScreenshotsByDevice(ctx context.Context, deviceID uuid.UUID) ([]domain.ScreenshotRequestDB, error) {
	return s.screenshotRepo.ListByDevice(ctx, deviceID)
}

func (s *ScreenshotService) GetScreenshotImage(ctx context.Context, requestID uuid.UUID) ([]byte, string, error) {
	req, err := s.screenshotRepo.GetByID(ctx, requestID)
	if err != nil {
		return nil, "", fmt.Errorf("screenshot request not found: %w", err)
	}

	if req.ImagePath == nil || *req.ImagePath == "" {
		return nil, "", fmt.Errorf("screenshot image not available")
	}

	filePath := filepath.Join(s.uploadDir, *req.ImagePath)
	data, err := os.ReadFile(filePath)
	if err != nil {
		return nil, "", fmt.Errorf("read screenshot file: %w", err)
	}

	return data, "image/png", nil
}

func (s *ScreenshotService) GetRequestByID(ctx context.Context, id uuid.UUID) (*domain.ScreenshotRequestDB, error) {
	return s.screenshotRepo.GetByID(ctx, id)
}

func (s *ScreenshotService) GetDeviceCompanyID(ctx context.Context, deviceID uuid.UUID) string {
	device, err := s.deviceRepo.GetByID(ctx, deviceID)
	if err != nil {
		return ""
	}
	employee, err := s.employeeRepo.GetByID(ctx, device.EmployeeID)
	if err != nil {
		return ""
	}
	return employee.CompanyID.String()
}