package handler

import (
	"context"
	"encoding/json"
	"io"
	"log"
	"net/http"
	"time"

	"github.com/ainms/gateway/internal/middleware"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

func RequestScreenshot(svc *service.ScreenshotService, hub *service.SocketHub) http.HandlerFunc {
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

		if err := hub.SendToAgent(req.DeviceID, "screenshot_request", map[string]interface{}{
			"request_id":   result.ID.String(),
			"device_id":    req.DeviceID,
			"reason":       req.Reason,
			"policy":       req.Policy,
			"requested_by": userIDStr,
		}); err != nil {
			log.Printf("[ScreenshotHandler] failed to send screenshot_request via Socket.IO to device %s: %v", req.DeviceID, err)
		}

		writeJSON(w, http.StatusCreated, result)
	}
}

func UploadScreenshot(svc *service.ScreenshotService, hub *service.SocketHub, compSvc *service.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if err := r.ParseMultipartForm(20 << 20); err != nil {
			writeError(w, http.StatusBadRequest, "failed to parse form: "+err.Error())
			return
		}

		requestIDStr := r.FormValue("request_id")
		deviceIDStr := r.FormValue("device_id")
		windowTitle := r.FormValue("window_title")
		appName := r.FormValue("app_name")

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

		if compSvc != nil && windowTitle != "" {
			go func() {
				ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
				defer cancel()
				alert, err := compSvc.AnalyzeScreenshot(ctx, requestID, deviceID, windowTitle, appName)
				if err != nil {
					log.Printf("[compliance] AnalyzeScreenshot failed: %v", err)
				} else {
					log.Printf("[compliance] Alert created: id=%s decision=%s message=%q", alert.ID, alert.Decision, alert.Message)
				}
			}()
		}

		companyID := svc.GetDeviceCompanyID(r.Context(), deviceID)
		hub.BroadcastToCompanyAdmins(companyID, "screenshot_ready", map[string]interface{}{
			"request_id": requestIDStr,
			"device_id":  deviceIDStr,
			"status":     "completed",
		})

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

// BatchUploadScreenshots accepts up to 5 screenshots in a single multipart request
// along with per-screenshot metadata and app usage data, then triggers a single
// batch compliance analysis via Ollama Cloud.
func BatchUploadScreenshots(svc *service.ScreenshotService, hub *service.SocketHub, compSvc *service.ComplianceService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if err := r.ParseMultipartForm(100 << 20); err != nil {
			writeError(w, http.StatusBadRequest, "failed to parse form: "+err.Error())
			return
		}

		deviceIDStr := r.FormValue("device_id")
		deviceID, err := uuid.Parse(deviceIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid device_id")
			return
		}

		metadataJSON := r.FormValue("metadata")
		appUsageJSON := r.FormValue("app_usage")

		var metas []service.ScreenshotMeta
		if err := json.Unmarshal([]byte(metadataJSON), &metas); err != nil {
			writeError(w, http.StatusBadRequest, "invalid metadata JSON: "+err.Error())
			return
		}

		var appUsage []service.AppUsageEntry
		if appUsageJSON != "" {
			if err := json.Unmarshal([]byte(appUsageJSON), &appUsage); err != nil {
				writeError(w, http.StatusBadRequest, "invalid app_usage JSON: "+err.Error())
				return
			}
		}

		type imgEntry struct {
			id   uuid.UUID
			data []byte
		}
		var images []imgEntry
		for i := 0; ; i++ {
			key := "image_" + string(rune('0'+i))
			file, _, err := r.FormFile(key)
			if err != nil {
				break
			}
			data, err := io.ReadAll(file)
			file.Close()
			if err != nil {
				writeError(w, http.StatusInternalServerError, "failed to read image "+key)
				return
			}

			var id uuid.UUID
			if i < len(metas) {
				id = metas[i].RequestID
			} else {
				id = uuid.New()
			}
			images = append(images, imgEntry{id: id, data: data})
		}

		if len(images) == 0 {
			writeError(w, http.StatusBadRequest, "no images provided")
			return
		}

		screenshotIDs := make([]uuid.UUID, len(images))
		imageDataMap := make(map[uuid.UUID][]byte, len(images))
		for i, img := range images {
			screenshotIDs[i] = img.id
			imageDataMap[img.id] = img.data

			_, err := svc.UploadScreenshot(r.Context(), img.id, deviceID, img.data)
			if err != nil {
				log.Printf("[batch-upload] UploadScreenshot for %s failed: %v", img.id, err)
			}
		}

		if len(metas) > len(images) {
			metas = metas[:len(images)]
		}

		if compSvc != nil {
			go func() {
				ctx, cancel := context.WithTimeout(context.Background(), 150*time.Second)
				defer cancel()
				alert, err := compSvc.AnalyzeScreenshotsBatch(ctx, deviceID, screenshotIDs, imageDataMap, metas, appUsage)
				if err != nil {
					log.Printf("[compliance] AnalyzeScreenshotsBatch failed: %v", err)
				} else {
					log.Printf("[compliance] Batch alert created: id=%s decision=%s message=%q", alert.ID, alert.Decision, alert.Message)
				}
			}()
		}

		companyID := svc.GetDeviceCompanyID(r.Context(), deviceID)
		hub.BroadcastToCompanyAdmins(companyID, "screenshot_ready", map[string]interface{}{
			"device_id": deviceIDStr,
			"count":     len(images),
			"status":    "completed",
		})

		writeJSON(w, http.StatusOK, map[string]interface{}{
			"status":         "ok",
			"screenshots_saved": len(images),
		})
	}
}