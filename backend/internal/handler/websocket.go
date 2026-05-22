package handler

import (
	"net/http"
)

func WebSocketCommands(svc interface{}) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusNotImplemented, map[string]string{"message": "WebSocketCommands not yet implemented"})
	}
}