package handler

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

type NLQueryRequest struct {
	Query string `json:"query" validate:"required"`
}

type NLQueryResponse struct {
	QueryID    string `json:"query_id"`
	EmployeeID string `json:"employee_id"`
	DeviceID   string `json:"device_id,omitempty"`
	Status     string `json:"status"`
}

func PostNLQuery(
	employeeRepo *postgres.EmployeeRepo,
	deviceRepo *postgres.DeviceRepo,
	socketHub *service.SocketHub,
) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee_id")
			return
		}

		var req NLQueryRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if req.Query == "" {
			writeError(w, http.StatusBadRequest, "query is required")
			return
		}

		// Find the employee's devices
		devices, err := deviceRepo.GetByEmployeeID(r.Context(), employeeID)
		if err != nil {
			writeError(w, http.StatusNotFound, "employee not found or no devices")
			return
		}

		// Find the first online device
		var targetDeviceID uuid.UUID
		now := time.Now()
		for _, d := range devices {
			if d.LastHeartbeat != nil && now.Sub(*d.LastHeartbeat) < 5*time.Minute {
				targetDeviceID = d.ID
				break
			}
		}

		if targetDeviceID == uuid.Nil {
			targetDeviceID = devices[0].ID
		}

		queryID := uuid.New().String()

		payload, _ := json.Marshal(map[string]interface{}{
			"query_id":    queryID,
			"query":       req.Query,
			"employee_id": employeeIDStr,
			"timestamp":   time.Now().UTC().Format(time.RFC3339),
		})

		// Push to agent via Socket.IO
		socketHub.SendToAgent(targetDeviceID.String(), "nl_query", payload)

		response := NLQueryResponse{
			QueryID:    queryID,
			EmployeeID: employeeIDStr,
			DeviceID:   targetDeviceID.String(),
			Status:     "sent",
		}

		writeJSON(w, http.StatusAccepted, response)
	}
}