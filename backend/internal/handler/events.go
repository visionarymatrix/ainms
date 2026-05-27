package handler

import (
	"encoding/json"
	"log"
	"net/http"
	"strconv"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/clickhouse"
	"github.com/go-chi/chi/v5"
)

func BulkEvents(svc *clickhouse.EventRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req domain.BulkEventRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if req.DeviceID == "" {
			writeError(w, http.StatusBadRequest, "device_id is required")
			return
		}

		if len(req.Metadata) == 0 {
			writeError(w, http.StatusBadRequest, "metadata is required")
			return
		}

		if err := svc.StoreBulkEvents(r.Context(), req.DeviceID, req.Summary, req.Metadata); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}

func PriorityEvent(svc *clickhouse.EventRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req domain.PriorityEventRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if err := svc.StorePriorityEvent(r.Context(), req); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}

func GetDeviceUsageSummaries(svc *clickhouse.EventRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		deviceID := chi.URLParam(r, "deviceID")
		if deviceID == "" {
			writeError(w, http.StatusBadRequest, "device_id is required")
			return
		}
		summaries, err := svc.GetAppUsageSummaries(r.Context(), []string{deviceID})
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		writeJSON(w, http.StatusOK, summaries)
	}
}

func GetDeviceEvents(svc *clickhouse.EventRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		deviceID := chi.URLParam(r, "deviceID")
		if deviceID == "" {
			writeError(w, http.StatusBadRequest, "device_id is required")
			return
		}
		limit := 1000
		if l := r.URL.Query().Get("limit"); l != "" {
			if v, err := strconv.Atoi(l); err == nil && v > 0 && v <= 5000 {
				limit = v
			}
		}
		events, err := svc.GetEventsByDevice(r.Context(), deviceID, limit)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		writeJSON(w, http.StatusOK, events)
	}
}

func PopupEvent(svc *clickhouse.EventRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req domain.PopupEvent
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if err := svc.StorePopupEvent(r.Context(), req); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}

func BrowserTabsEvent(svc *clickhouse.EventRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req struct {
			DeviceID string `json:"device_id"`
			Tabs     []struct {
				Title   string `json:"title"`
				URL     string `json:"url"`
				Browser string `json:"browser"`
				Active  bool   `json:"active"`
			} `json:"tabs"`
		}
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		log.Printf("[BrowserTabs] device=%s tab_count=%d", req.DeviceID, len(req.Tabs))

		// For now, just log and acknowledge. Analytics storage can be added later.
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}

func NetworkTrafficEvent(svc *clickhouse.EventRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req struct {
			DeviceID    string          `json:"device_id"`
			Summary     json.RawMessage  `json:"summary"`
			Connections []struct {
				Protocol        string  `json:"protocol"`
				LocalIP         string  `json:"local_ip"`
				LocalPort       uint16  `json:"local_port"`
				RemoteIP        string  `json:"remote_ip"`
				RemotePort      uint16  `json:"remote_port"`
				State           string  `json:"state"`
				ProcessID       int32   `json:"process_id"`
				ProcessName     string  `json:"process_name"`
				RemoteHostname  *string `json:"remote_hostname"`
				ReconstructedURL *string `json:"reconstructed_url"`
			} `json:"connections"`
		}
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		log.Printf("[NetworkTraffic] device=%s conn_count=%d", req.DeviceID, len(req.Connections))

		// For now, just log and acknowledge. Analytics storage can be added later.
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}