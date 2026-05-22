package handler

import (
	"net/http"
)

func GetLatestModel(svc interface{}) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusNotImplemented, map[string]string{"message": "GetLatestModel not yet implemented"})
	}
}