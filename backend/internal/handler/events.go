package handler

import (
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