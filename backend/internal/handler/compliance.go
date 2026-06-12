package handler

import (
	"net/http"

	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

func GetDeviceAlerts(compSvc *service.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		deviceIDStr := chi.URLParam(r, "deviceID")
		deviceID, err := uuid.Parse(deviceIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		alerts, err := compSvc.GetPendingAlerts(r.Context(), deviceID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, alerts)
	}
}

func AckAlert(compSvc *service.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		alertIDStr := chi.URLParam(r, "alertID")
		alertID, err := uuid.Parse(alertIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid alert_id")
			return
		}

		if err := compSvc.AckAlert(r.Context(), alertID); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "acked"})
	}
}

func ListCompanyAlerts(compSvc *service.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyIDStr := chi.URLParam(r, "companyID")
		companyID, err := uuid.Parse(companyIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company_id")
			return
		}

		status := r.URL.Query().Get("status")

		alerts, err := compSvc.ListCompanyAlerts(r.Context(), companyID, status)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, alerts)
	}
}

func ListEmployeeAlerts(compSvc *service.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee_id")
			return
		}

		status := r.URL.Query().Get("status")

		alerts, err := compSvc.ListEmployeeAlerts(r.Context(), employeeID, status)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, alerts)
	}
}
