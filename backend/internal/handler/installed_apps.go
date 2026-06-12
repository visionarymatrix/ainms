package handler

import (
	"net/http"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

func UploadInstalledApps(svc *service.InstalledAppService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req domain.InstalledAppsUploadRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if req.DeviceID == "" {
			writeError(w, http.StatusBadRequest, "device_id is required")
			return
		}

		if len(req.Apps) == 0 {
			writeError(w, http.StatusBadRequest, "apps is required")
			return
		}

		deviceUUID, err := uuid.Parse(req.DeviceID)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		if err := svc.UpsertInstalledApps(r.Context(), deviceUUID, req.Apps); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}

func GetDeviceInstalledApps(repo *postgres.InstalledAppRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		deviceID := chi.URLParam(r, "deviceID")
		if deviceID == "" {
			writeError(w, http.StatusBadRequest, "device_id is required")
			return
		}

		deviceUUID, err := uuid.Parse(deviceID)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		apps, err := repo.GetByDeviceID(r.Context(), deviceUUID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, apps)
	}
}

func GetEmployeeInstalledApps(repo *postgres.InstalledAppRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeID := chi.URLParam(r, "employeeID")
		if employeeID == "" {
			writeError(w, http.StatusBadRequest, "employee_id is required")
			return
		}

		employeeUUID, err := uuid.Parse(employeeID)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee_id")
			return
		}

		apps, err := repo.GetByEmployeeID(r.Context(), employeeUUID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, apps)
	}
}

func GetDeviceValidations(svc *service.InstalledAppService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		deviceID := chi.URLParam(r, "deviceID")
		if deviceID == "" {
			writeError(w, http.StatusBadRequest, "device_id is required")
			return
		}

		deviceUUID, err := uuid.Parse(deviceID)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		validations, err := svc.GetValidationsByDevice(r.Context(), deviceUUID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, validations)
	}
}

func GetEmployeeValidations(svc *service.InstalledAppService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeID := chi.URLParam(r, "employeeID")
		if employeeID == "" {
			writeError(w, http.StatusBadRequest, "employee_id is required")
			return
		}

		employeeUUID, err := uuid.Parse(employeeID)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee_id")
			return
		}

		validations, err := svc.GetValidationsByEmployee(r.Context(), employeeUUID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, validations)
	}
}

func GetCompanyValidations(svc *service.InstalledAppService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyID := chi.URLParam(r, "companyID")
		if companyID == "" {
			writeError(w, http.StatusBadRequest, "company_id is required")
			return
		}

		companyUUID, err := uuid.Parse(companyID)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company_id")
			return
		}

		validations, err := svc.GetValidationsByCompany(r.Context(), companyUUID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, validations)
	}
}

func GetCompanyNonCompliantApps(svc *service.InstalledAppService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyID := chi.URLParam(r, "companyID")
		if companyID == "" {
			writeError(w, http.StatusBadRequest, "company_id is required")
			return
		}

		companyUUID, err := uuid.Parse(companyID)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company_id")
			return
		}

		validations, err := svc.GetNonCompliantByCompany(r.Context(), companyUUID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, validations)
	}
}