package service

import (
	"context"
	"fmt"
	"log"
	"strings"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/google/uuid"
)

type InstalledAppService struct {
	appRepo        *postgres.InstalledAppRepo
	validationRepo *postgres.InstalledAppValidationRepo
	deviceRepo     *postgres.DeviceRepo
	employeeRepo   *postgres.EmployeeRepo
	classRepo      *postgres.AppClassificationRepo
	roleRepo       *postgres.RoleRepo
}

func NewInstalledAppService(
	appRepo *postgres.InstalledAppRepo,
	validationRepo *postgres.InstalledAppValidationRepo,
	deviceRepo *postgres.DeviceRepo,
	employeeRepo *postgres.EmployeeRepo,
	classRepo *postgres.AppClassificationRepo,
	roleRepo *postgres.RoleRepo,
) *InstalledAppService {
	return &InstalledAppService{
		appRepo:        appRepo,
		validationRepo: validationRepo,
		deviceRepo:     deviceRepo,
		employeeRepo:   employeeRepo,
		classRepo:      classRepo,
		roleRepo:       roleRepo,
	}
}

func (s *InstalledAppService) UpsertInstalledApps(ctx context.Context, deviceID uuid.UUID, apps []domain.InstalledAppEntry) error {
	if err := s.appRepo.UpsertInstalledApps(ctx, deviceID, apps); err != nil {
		return fmt.Errorf("upsert installed apps: %w", err)
	}

	go func() {
		bgCtx := context.Background()
		if err := s.ValidateInstalledApps(bgCtx, deviceID); err != nil {
			log.Printf("warning: background validation failed for device %s: %v", deviceID, err)
		}
	}()

	return nil
}

func (s *InstalledAppService) ValidateInstalledApps(ctx context.Context, deviceID uuid.UUID) error {
	apps, err := s.appRepo.GetByDeviceID(ctx, deviceID)
	if err != nil {
		return fmt.Errorf("get installed apps for validation: %w", err)
	}
	if len(apps) == 0 {
		return nil
	}

	device, err := s.deviceRepo.GetByID(ctx, deviceID)
	if err != nil {
		return fmt.Errorf("get device: %w", err)
	}

	employee, err := s.employeeRepo.GetByID(ctx, device.EmployeeID)
	if err != nil {
		return fmt.Errorf("get employee: %w", err)
	}

	var roleID *uuid.UUID
	var role *domain.Role
	var classificationMap map[string]string

	if employee.RoleID != nil {
		roleID = employee.RoleID

		r, err := s.roleRepo.GetByID(ctx, *roleID)
		if err != nil {
			return fmt.Errorf("get role: %w", err)
		}
		role = r

		classifications, err := s.classRepo.ListByRole(ctx, *roleID)
		if err != nil {
			return fmt.Errorf("get app classifications: %w", err)
		}

		classificationMap = make(map[string]string, len(classifications))
		for _, ac := range classifications {
			classificationMap[strings.ToLower(ac.AppName)] = ac.Category
		}
	}

	now := time.Now()
	validations := make([]domain.InstalledAppValidation, 0, len(apps))

	for _, app := range apps {
		v := domain.InstalledAppValidation{
			ID:            uuid.New(),
			DeviceID:      deviceID,
			AppName:       app.AppName,
			RoleID:        roleID,
			DisplayName:   app.DisplayName,
			AgentCategory: app.Category,
			ValidatedAt:   now,
		}

		if role == nil {
			v.ValidatedCategory = app.Category
			v.IsCompliant = true
			v.Reason = "no_role_assigned"
		} else {
			appLower := strings.ToLower(app.AppName)
			displayLower := strings.ToLower(app.DisplayName)

			if ruleCategory, found := matchClassification(classificationMap, appLower, displayLower); found {
				v.ValidatedCategory = ruleCategory
				v.IsCompliant = ruleCategory != "unproductive"
				v.Reason = "app_classification_rule"
			} else if isInCategories(app.Category, role.BlockedCategories) {
				v.ValidatedCategory = app.Category
				v.IsCompliant = false
				v.Reason = "category_blocked_by_role"
			} else if len(role.AllowedCategories) > 0 && !isInCategories(app.Category, role.AllowedCategories) {
				v.ValidatedCategory = app.Category
				v.IsCompliant = false
				v.Reason = "category_not_in_allowed_list"
			} else {
				v.ValidatedCategory = app.Category
				v.IsCompliant = true
				v.Reason = "category_allowed_by_role"
			}
		}

		validations = append(validations, v)
	}

	if err := s.validationRepo.UpsertValidations(ctx, validations); err != nil {
		return fmt.Errorf("upsert validations: %w", err)
	}

	return nil
}

func (s *InstalledAppService) GetValidationsByDevice(ctx context.Context, deviceID uuid.UUID) ([]domain.InstalledAppValidation, error) {
	return s.validationRepo.GetByDeviceID(ctx, deviceID)
}

func (s *InstalledAppService) GetValidationsByEmployee(ctx context.Context, employeeID uuid.UUID) ([]domain.InstalledAppValidationWithDetails, error) {
	return s.validationRepo.GetByEmployeeID(ctx, employeeID)
}

func (s *InstalledAppService) GetValidationsByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.InstalledAppValidationWithDetails, error) {
	return s.validationRepo.GetAllByCompany(ctx, companyID)
}

func (s *InstalledAppService) GetNonCompliantByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.InstalledAppValidationWithDetails, error) {
	return s.validationRepo.GetNonCompliantByCompany(ctx, companyID)
}

func matchClassification(classMap map[string]string, appLower string, displayLower string) (string, bool) {
	for ruleName, category := range classMap {
		ruleLower := strings.ToLower(ruleName)
		if strings.Contains(appLower, ruleLower) || strings.Contains(displayLower, ruleLower) {
			return category, true
		}
	}
	return "", false
}

func isInCategories(category string, categories []string) bool {
	catLower := strings.ToLower(category)
	for _, c := range categories {
		if strings.ToLower(c) == catLower {
			return true
		}
	}
	return false
}