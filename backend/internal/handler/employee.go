package handler

import (
	"net/http"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/middleware"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

func RegisterEmployee(svc *service.EmployeeService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyIDStr := chi.URLParam(r, "companyID")
		companyID, err := uuid.Parse(companyIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company id")
			return
		}

		var req domain.RegisterEmployeeRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		emp, err := svc.Register(r.Context(), companyID, req)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, emp)
	}
}

func GetEmployee(svc *service.EmployeeService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee id")
			return
		}

		emp, err := svc.GetByID(r.Context(), employeeID)
		if err != nil {
			writeError(w, http.StatusNotFound, "employee not found")
			return
		}

		writeJSON(w, http.StatusOK, emp)
	}
}

func ListEmployees(svc *service.EmployeeService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyIDStr := chi.URLParam(r, "companyID")
		companyID, err := uuid.Parse(companyIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company id")
			return
		}

		employees, err := svc.List(r.Context(), companyID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, employees)
	}
}

func UpdateEmployee(svc *service.EmployeeService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusNotImplemented, map[string]string{"message": "UpdateEmployee not yet implemented"})
	}
}

func DeactivateEmployee(svc *service.EmployeeService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee id")
			return
		}

		if err := svc.Deactivate(r.Context(), employeeID); err != nil {
			writeError(w, http.StatusNotFound, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "deactivated"})
	}
}

func GetEmployeeDevices(deviceRepo *postgres.DeviceRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeUUID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee_id")
			return
		}

		devices, err := deviceRepo.GetByEmployeeID(r.Context(), employeeUUID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		now := time.Now()
		for i := range devices {
			if devices[i].LastHeartbeat == nil {
				devices[i].ConnectionStatus = "offline"
			} else if now.Sub(*devices[i].LastHeartbeat) < 5*time.Minute {
				devices[i].ConnectionStatus = "online"
			} else if now.Sub(*devices[i].LastHeartbeat) < 30*time.Minute {
				devices[i].ConnectionStatus = "idle"
			} else {
				devices[i].ConnectionStatus = "offline"
			}
		}

		writeJSON(w, http.StatusOK, devices)
	}
}

func GenerateEmployeeInstallToken(svc *service.InstallTokenService, empSvc *service.EmployeeService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		role := middleware.GetRole(r.Context())
		if role != "company_admin" && role != "super_admin" {
			writeError(w, http.StatusForbidden, "only company_admin or super_admin can generate install tokens")
			return
		}

		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee_id")
			return
		}

		var companyIDStr string
		if cidStr := middleware.GetCompanyID(r.Context()); cidStr != nil && *cidStr != "" {
			companyIDStr = *cidStr
		} else {
			emp, err := empSvc.GetByID(r.Context(), employeeID)
			if err != nil {
				writeError(w, http.StatusBadRequest, "employee not found")
				return
			}
			companyIDStr = emp.CompanyID.String()
		}

		userIDStr := middleware.GetUserID(r.Context())

		resp, err := svc.GetOrCreateForEmployee(r.Context(), employeeID.String(), companyIDStr, userIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, resp)
	}
}