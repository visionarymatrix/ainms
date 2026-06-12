package handler

import (
	"net/http"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/google/uuid"
)

func SyncRules(
	appClassificationRepo *postgres.AppClassificationRepo,
	alertRuleRepo *postgres.AlertRuleRepo,
	policyRepo *postgres.PolicyRepo,
	deviceRepo *postgres.DeviceRepo,
	employeeRepo *postgres.EmployeeRepo,
	companyRepo *postgres.CompanyRepo,
	roleRepo *postgres.RoleRepo,
) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		deviceIDStr := r.URL.Query().Get("device_id")
		if deviceIDStr == "" {
			writeError(w, http.StatusBadRequest, "device_id query parameter is required")
			return
		}

		deviceID, err := uuid.Parse(deviceIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		device, err := deviceRepo.GetByID(r.Context(), deviceID)
		if err != nil {
			writeError(w, http.StatusNotFound, "device not found")
			return
		}

		rules := domain.RuleSetResponse{
			AppClassifications: []domain.AppClassification{},
			AlertRules:        []domain.AlertRule{},
			Policy: domain.Policy{
				ID:                uuid.New(),
				TenantID:          uuid.Nil,
				UploadInterval:    300,
				ScreenshotEnabled: true,
				ScreenshotPolicy:  "metadata_only",
			},
		}

		employee, empErr := employeeRepo.GetByID(r.Context(), device.EmployeeID)
		if empErr == nil && employee.RoleID != nil {
			if role, err := roleRepo.GetByID(r.Context(), *employee.RoleID); err == nil {
				rules.RoleInfo = &domain.RoleInfo{
					ID:                role.ID,
					Name:              role.Name,
					Description:       role.Description,
					WorkDescription:   role.WorkDescription,
					AllowedCategories: role.AllowedCategories,
					BlockedCategories: role.BlockedCategories,
				}
			}

			if acs, err := appClassificationRepo.ListByRole(r.Context(), *employee.RoleID); err == nil {
				rules.AppClassifications = acs
			}

			if ars, err := alertRuleRepo.ListByRole(r.Context(), *employee.RoleID); err == nil {
				rules.AlertRules = ars
			}
		}

		if employee != nil {
			if company, err := companyRepo.GetByID(r.Context(), employee.CompanyID); err == nil {
				if policy, err := policyRepo.GetByTenantID(r.Context(), company.TenantID); err == nil && policy != nil {
					rules.Policy = *policy
				}
			}
		}

		writeJSON(w, http.StatusOK, rules)
	}
}