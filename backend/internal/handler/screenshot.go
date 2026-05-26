package handler

import (
	"io"
	"net/http"

	"github.com/ainms/gateway/internal/middleware"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

func RequestScreenshot(svc *service.ScreenshotService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req struct {
			DeviceID string `json:"device_id"`
			Reason   string `json:"reason"`
			Policy   string `json:"policy"`
		}
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		deviceID, err := uuid.Parse(req.DeviceID)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		if req.Reason == "" {
			req.Reason = "On-demand screenshot"
		}
		if req.Policy == "" {
			req.Policy = "upload_image"
		}

		userIDStr := middleware.GetUserID(r.Context())
		requestedBy, err := uuid.Parse(userIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid user_id in token")
			return
		}

		result, err := svc.RequestScreenshot(r.Context(), deviceID, requestedBy, req.Reason, req.Policy)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, result)
	}
}

func UploadScreenshot(svc *service.ScreenshotService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if err := r.ParseMultipartForm(20 << 20); err != nil {
			writeError(w, http.StatusBadRequest, "failed to parse form: "+err.Error())
			return
		}

		requestIDStr := r.FormValue("request_id")
		deviceIDStr := r.FormValue("device_id")

		requestID, err := uuid.Parse(requestIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid request_id")
			return
		}

		deviceID, err := uuid.Parse(deviceIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		file, _, err := r.FormFile("image")
		if err != nil {
			writeError(w, http.StatusBadRequest, "image file required")
			return
		}
		defer file.Close()

		imageData, err := io.ReadAll(file)
		if err != nil {
			writeError(w, http.StatusInternalServerError, "failed to read image")
			return
		}

		result, err := svc.UploadScreenshot(r.Context(), requestID, deviceID, imageData)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, result)
	}
}

func GetDeviceScreenshots(svc *service.ScreenshotService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		deviceIDStr := chi.URLParam(r, "deviceID")
		deviceID, err := uuid.Parse(deviceIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		screenshots, err := svc.GetScreenshotsByDevice(r.Context(), deviceID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, screenshots)
	}
}

func GetScreenshotImage(svc *service.ScreenshotService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		requestIDStr := chi.URLParam(r, "requestID")
		requestID, err := uuid.Parse(requestIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid request_id")
			return
		}

		data, contentType, err := svc.GetScreenshotImage(r.Context(), requestID)
		if err != nil {
			writeError(w, http.StatusNotFound, "screenshot image not found")
			return
		}

		w.Header().Set("Content-Type", contentType)
		w.Header().Set("Cache-Control", "public, max-age=86400")
		w.Write(data)
	}
}

func GetPendingCommands(svc *service.ScreenshotService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		deviceIDStr := chi.URLParam(r, "deviceID")
		deviceID, err := uuid.Parse(deviceIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		commands, err := svc.GetPendingCommands(r.Context(), deviceID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, commands)
	}
}

func AcknowledgeCommand(svc *service.ScreenshotService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req struct {
			CommandID string `json:"command_id"`
		}
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		commandID, err := uuid.Parse(req.CommandID)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid command_id")
			return
		}

		if err := svc.AcknowledgeCommand(r.Context(), commandID); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "acked"})
	}
}