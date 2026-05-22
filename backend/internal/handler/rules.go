package handler

import (
	"net/http"
)

func SyncRules(svc interface{}) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusNotImplemented, map[string]string{"message": "SyncRules not yet implemented"})
	}
}