package handler

import (
	"errors"
	"net/http"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/middleware"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

func EnrollDevice(svc *service.EnrollmentService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req domain.EnrollmentRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if req.EmployeeID == "" {
			writeError(w, http.StatusBadRequest, "employee_id is required")
			return
		}

		if req.OSType == "" {
			writeError(w, http.StatusBadRequest, "os_type is required")
			return
		}

		if req.Fingerprint == "" {
			writeError(w, http.StatusBadRequest, "fingerprint is required")
			return
		}

		if req.CompanyID == "" {
			writeError(w, http.StatusBadRequest, "company_id is required")
			return
		}

		resp, err := svc.Enroll(r.Context(), req)
		if err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, resp)
	}
}

func DeviceHeartbeat(svc *service.EnrollmentService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		deviceIDStr := chi.URLParam(r, "deviceID")
		if deviceIDStr == "" {
			writeError(w, http.StatusBadRequest, "device_id is required")
			return
		}

		if err := svc.Heartbeat(r.Context(), deviceIDStr); err != nil {
			var approvalErr *service.ApprovalError
			if errors.As(err, &approvalErr) {
				writeError(w, http.StatusForbidden, approvalErr.Message)
				return
			}
			writeError(w, http.StatusNotFound, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}

func DeviceFleetStatus(svc *service.EnrollmentService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		role := middleware.GetRole(r.Context())
		companyIDStr := middleware.GetCompanyID(r.Context())

		devices, err := svc.FleetStatus(r.Context(), role, companyIDStr)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, devices)
	}
}

func ApproveDevice(svc *service.EnrollmentService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		role := middleware.GetRole(r.Context())
		if role != "company_admin" && role != "super_admin" {
			writeError(w, http.StatusForbidden, "only company_admin or super_admin can approve devices")
			return
		}

		deviceIDStr := chi.URLParam(r, "deviceID")
		deviceID, err := uuid.Parse(deviceIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		userIDStr := middleware.GetUserID(r.Context())
		approvedBy, err := uuid.Parse(userIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid user_id in token")
			return
		}

		device, err := svc.ApproveDevice(r.Context(), deviceID, approvedBy)
		if err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, device)
	}
}

func RejectDevice(svc *service.EnrollmentService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		role := middleware.GetRole(r.Context())
		if role != "company_admin" && role != "super_admin" {
			writeError(w, http.StatusForbidden, "only company_admin or super_admin can reject devices")
			return
		}

		deviceIDStr := chi.URLParam(r, "deviceID")
		deviceID, err := uuid.Parse(deviceIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		if err := svc.RejectDevice(r.Context(), deviceID); err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "rejected"})
	}
}

func ListPendingDevices(svc *service.EnrollmentService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		role := middleware.GetRole(r.Context())
		companyIDStr := middleware.GetCompanyID(r.Context())

		var devices []domain.Device
		var err error

		if role == "super_admin" {
			devices, err = svc.ListAllPending(r.Context())
		} else {
			if companyIDStr == nil || *companyIDStr == "" {
				writeError(w, http.StatusBadRequest, "company_id is required")
				return
			}

			companyID, err := uuid.Parse(*companyIDStr)
			if err != nil {
				writeError(w, http.StatusBadRequest, "invalid company_id")
				return
			}

			devices, err = svc.ListPendingByCompany(r.Context(), companyID)
		}

		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, devices)
	}
}

func GetDeviceStatus(svc *service.EnrollmentService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		deviceIDStr := chi.URLParam(r, "deviceID")
		deviceID, err := uuid.Parse(deviceIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		status, err := svc.GetDeviceStatus(r.Context(), deviceID)
		if err != nil {
			writeError(w, http.StatusNotFound, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": status, "device_id": deviceIDStr})
	}
}