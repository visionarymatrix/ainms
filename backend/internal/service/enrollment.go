package service

import (
	"context"
	"fmt"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/google/uuid"
)

type ApprovalError struct {
	DeviceStatus string
	Message      string
}

func (e *ApprovalError) Error() string {
	return e.Message
}

type EnrollmentService struct {
	employeeRepo *postgres.EmployeeRepo
	deviceRepo   *postgres.DeviceRepo
}

func NewEnrollmentService(employeeRepo *postgres.EmployeeRepo, deviceRepo *postgres.DeviceRepo) *EnrollmentService {
	return &EnrollmentService{
		employeeRepo: employeeRepo,
		deviceRepo:   deviceRepo,
	}
}

func (s *EnrollmentService) Enroll(ctx context.Context, req domain.EnrollmentRequest) (*domain.EnrollmentResponse, error) {
	companyID, err := uuid.Parse(req.CompanyID)
	if err != nil {
		return nil, fmt.Errorf("invalid company_id: %w", err)
	}

	employee, err := s.employeeRepo.GetByEmployeeID(ctx, companyID, req.EmployeeID)
	if err != nil {
		return nil, fmt.Errorf("employee not found: %w", err)
	}

	if employee.Status != "active" {
		return nil, fmt.Errorf("employee %s is %s, cannot enroll device", employee.EmployeeID, employee.Status)
	}

	// If fingerprint provided, check if this device was already enrolled and approved
	if req.Fingerprint != "" {
		existing, err := s.deviceRepo.GetByFingerprint(ctx, req.Fingerprint, employee.ID)
		if err == nil && existing != nil {
			if existing.Status == "active" || existing.Status == "pending" {
				updates := map[string]interface{}{}
				if req.Hostname != "" {
					updates["hostname"] = req.Hostname
				}
				if req.OSType != "" {
					updates["os_type"] = req.OSType
				}
				if req.OSVersion != "" {
					updates["os_version"] = req.OSVersion
				}
				if req.CPUInfo != "" {
					updates["cpu_info"] = req.CPUInfo
				}
				if req.RAMInfo != "" {
					updates["ram_info"] = req.RAMInfo
				}
				if req.DiskInfo != "" {
					updates["disk_info"] = req.DiskInfo
				}
				if req.MACAddresses != "" {
					updates["mac_addresses"] = req.MACAddresses
				}
				if req.IPAddresses != "" {
					updates["ip_addresses"] = req.IPAddresses
				}
				if err := s.deviceRepo.UpdateHardwareInfo(ctx, existing.ID, updates); err != nil {
					return nil, fmt.Errorf("update device hardware info: %w", err)
				}

				deviceToken := uuid.New().String()
				response := &domain.EnrollmentResponse{
					DeviceID:    existing.ID,
					EmployeeID:  employee.EmployeeID,
					Employee:    *employee,
					DeviceToken: deviceToken,
					Status:      existing.Status,
					Rules: domain.RuleSetResponse{
						AppClassifications: []domain.AppClassification{},
						AlertRules:        []domain.AlertRule{},
						Policy: domain.Policy{
							ID:                uuid.New(),
							TenantID:          employee.CompanyID,
							UploadInterval:    300,
							ScreenshotEnabled: false,
							ScreenshotPolicy:  "metadata_only",
						},
					},
				}
				return response, nil
			}
		}
	}

	deviceToken := uuid.New().String()

	device := &domain.Device{
		ID:         uuid.New(),
		EmployeeID: employee.ID,
		Hostname:   &req.Hostname,
		OSType:     req.OSType,
		OSVersion:  &req.OSVersion,
		Status:     "pending",
	}

	if req.Hostname == "" {
		device.Hostname = nil
	}
	if req.OSVersion == "" {
		device.OSVersion = nil
	}

	if req.Fingerprint != "" {
		device.Fingerprint = &req.Fingerprint
	}
	if req.CPUInfo != "" {
		device.CPUInfo = &req.CPUInfo
	}
	if req.RAMInfo != "" {
		device.RAMInfo = &req.RAMInfo
	}
	if req.DiskInfo != "" {
		device.DiskInfo = &req.DiskInfo
	}
	if req.MACAddresses != "" {
		device.MACAddresses = &req.MACAddresses
	}
	if req.IPAddresses != "" {
		device.IPAddresses = &req.IPAddresses
	}

	if err := s.deviceRepo.Create(ctx, device); err != nil {
		return nil, fmt.Errorf("create device: %w", err)
	}

	response := &domain.EnrollmentResponse{
		DeviceID:    device.ID,
		EmployeeID:  employee.EmployeeID,
		Employee:    *employee,
		DeviceToken: deviceToken,
		Status:      "pending",
		Rules: domain.RuleSetResponse{
			AppClassifications: []domain.AppClassification{},
			AlertRules:        []domain.AlertRule{},
			Policy: domain.Policy{
				ID:                uuid.New(),
				TenantID:          employee.CompanyID,
				UploadInterval:    300,
				ScreenshotEnabled: false,
				ScreenshotPolicy:  "metadata_only",
			},
		},
	}

	return response, nil
}

func (s *EnrollmentService) Heartbeat(ctx context.Context, deviceIDStr string) error {
	deviceID, err := uuid.Parse(deviceIDStr)
	if err != nil {
		return fmt.Errorf("invalid device_id: %w", err)
	}

	device, err := s.deviceRepo.GetByID(ctx, deviceID)
	if err != nil {
		return fmt.Errorf("device not found: %w", err)
	}

	if device.Status != "active" {
		return &ApprovalError{
			DeviceStatus: device.Status,
			Message:      fmt.Sprintf("device is not approved, current status: %s", device.Status),
		}
	}

	if err := s.deviceRepo.UpdateHeartbeat(ctx, deviceID); err != nil {
		return fmt.Errorf("heartbeat failed: %w", err)
	}

	return nil
}

func (s *EnrollmentService) ApproveDevice(ctx context.Context, deviceID uuid.UUID, approvedBy uuid.UUID) (*domain.Device, error) {
	device, err := s.deviceRepo.GetByID(ctx, deviceID)
	if err != nil {
		return nil, fmt.Errorf("device not found: %w", err)
	}

	if device.Status != "pending" {
		return nil, fmt.Errorf("device cannot be approved: current status is %s", device.Status)
	}

	if err := s.deviceRepo.UpdateApproval(ctx, deviceID, approvedBy); err != nil {
		return nil, fmt.Errorf("approve device: %w", err)
	}

	return s.deviceRepo.GetByID(ctx, deviceID)
}

func (s *EnrollmentService) RejectDevice(ctx context.Context, deviceID uuid.UUID) error {
	device, err := s.deviceRepo.GetByID(ctx, deviceID)
	if err != nil {
		return fmt.Errorf("device not found: %w", err)
	}

	if device.Status != "pending" {
		return fmt.Errorf("device cannot be rejected: current status is %s", device.Status)
	}

	return s.deviceRepo.RejectDevice(ctx, deviceID)
}

func (s *EnrollmentService) ListPendingByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.Device, error) {
	devices, err := s.deviceRepo.ListPendingByCompany(ctx, companyID)
	if err != nil {
		return nil, err
	}
	setConnectionStatus(devices)
	return devices, nil
}

func (s *EnrollmentService) ListAllPending(ctx context.Context) ([]domain.Device, error) {
	devices, err := s.deviceRepo.ListAllPending(ctx)
	if err != nil {
		return nil, err
	}
	setConnectionStatus(devices)
	return devices, nil
}

func (s *EnrollmentService) GetDeviceStatus(ctx context.Context, deviceID uuid.UUID) (string, error) {
	return s.deviceRepo.GetDeviceStatus(ctx, deviceID)
}

func (s *EnrollmentService) FleetStatus(ctx context.Context, role string, companyID *string) ([]domain.Device, error) {
	var devices []domain.Device
	var err error

	if role == "super_admin" {
		devices, err = s.deviceRepo.List(ctx)
	} else if companyID != nil && *companyID != "" {
		var cid uuid.UUID
		cid, err = uuid.Parse(*companyID)
		if err != nil {
			return nil, fmt.Errorf("invalid company_id: %w", err)
		}
		devices, err = s.deviceRepo.ListByCompany(ctx, cid)
	} else {
		return []domain.Device{}, nil
	}

	if err != nil {
		return nil, err
	}

	setConnectionStatus(devices)
	return devices, nil
}

func (s *EnrollmentService) EnrollWithToken(ctx context.Context, req domain.EnrollmentRequest) (*domain.EnrollmentResponse, error) {
	employeeUUID, err := uuid.Parse(req.EmployeeID)
	if err != nil {
		return nil, fmt.Errorf("invalid employee_id: %w", err)
	}

	employee, err := s.employeeRepo.GetByID(ctx, employeeUUID)
	if err != nil {
		return nil, fmt.Errorf("employee not found: %w", err)
	}

	if employee.Status != "active" {
		return nil, fmt.Errorf("employee %s is %s, cannot enroll device", employee.EmployeeID, employee.Status)
	}

	if req.Fingerprint != "" {
		existing, err := s.deviceRepo.GetByFingerprint(ctx, req.Fingerprint, employee.ID)
		if err == nil && existing != nil {
			if existing.Status == "active" || existing.Status == "pending" {
				updates := map[string]interface{}{}
				if req.Hostname != "" {
					updates["hostname"] = req.Hostname
				}
				if req.OSType != "" {
					updates["os_type"] = req.OSType
				}
				if req.OSVersion != "" {
					updates["os_version"] = req.OSVersion
				}
				if req.CPUInfo != "" {
					updates["cpu_info"] = req.CPUInfo
				}
				if req.RAMInfo != "" {
					updates["ram_info"] = req.RAMInfo
				}
				if req.DiskInfo != "" {
					updates["disk_info"] = req.DiskInfo
				}
				if req.MACAddresses != "" {
					updates["mac_addresses"] = req.MACAddresses
				}
				if req.IPAddresses != "" {
					updates["ip_addresses"] = req.IPAddresses
				}
				if err := s.deviceRepo.UpdateHardwareInfo(ctx, existing.ID, updates); err != nil {
					return nil, fmt.Errorf("update device hardware info: %w", err)
				}

				deviceToken := uuid.New().String()
				response := &domain.EnrollmentResponse{
					DeviceID:    existing.ID,
					EmployeeID:  employee.EmployeeID,
					Employee:    *employee,
					DeviceToken: deviceToken,
					Status:      existing.Status,
					Rules: domain.RuleSetResponse{
						AppClassifications: []domain.AppClassification{},
						AlertRules:        []domain.AlertRule{},
						Policy: domain.Policy{
							ID:                uuid.New(),
							TenantID:          employee.CompanyID,
							UploadInterval:    300,
							ScreenshotEnabled: false,
							ScreenshotPolicy:  "metadata_only",
						},
					},
				}
				return response, nil
			}
		}
	}

	deviceToken := uuid.New().String()

	device := &domain.Device{
		ID:         uuid.New(),
		EmployeeID: employee.ID,
		Hostname:   &req.Hostname,
		OSType:     req.OSType,
		OSVersion:  &req.OSVersion,
		Status:     "active",
	}

	if req.Hostname == "" {
		device.Hostname = nil
	}
	if req.OSVersion == "" {
		device.OSVersion = nil
	}

	if req.Fingerprint != "" {
		device.Fingerprint = &req.Fingerprint
	}
	if req.CPUInfo != "" {
		device.CPUInfo = &req.CPUInfo
	}
	if req.RAMInfo != "" {
		device.RAMInfo = &req.RAMInfo
	}
	if req.DiskInfo != "" {
		device.DiskInfo = &req.DiskInfo
	}
	if req.MACAddresses != "" {
		device.MACAddresses = &req.MACAddresses
	}
	if req.IPAddresses != "" {
		device.IPAddresses = &req.IPAddresses
	}

	if err := s.deviceRepo.Create(ctx, device); err != nil {
		return nil, fmt.Errorf("create device: %w", err)
	}

	response := &domain.EnrollmentResponse{
		DeviceID:    device.ID,
		EmployeeID:  employee.EmployeeID,
		Employee:    *employee,
		DeviceToken: deviceToken,
		Status:      "active",
		Rules: domain.RuleSetResponse{
			AppClassifications: []domain.AppClassification{},
			AlertRules:        []domain.AlertRule{},
			Policy: domain.Policy{
				ID:                uuid.New(),
				TenantID:          employee.CompanyID,
				UploadInterval:    300,
				ScreenshotEnabled: false,
				ScreenshotPolicy:  "metadata_only",
			},
		},
	}

	return response, nil
}

func setConnectionStatus(devices []domain.Device) {
	now := time.Now()
	for i := range devices {
		if devices[i].LastHeartbeat == nil {
			devices[i].ConnectionStatus = "offline"
			continue
		}
		diff := now.Sub(*devices[i].LastHeartbeat)
		if diff < 5*time.Minute {
			devices[i].ConnectionStatus = "online"
		} else if diff < 30*time.Minute {
			devices[i].ConnectionStatus = "idle"
		} else {
			devices[i].ConnectionStatus = "offline"
		}
	}
}