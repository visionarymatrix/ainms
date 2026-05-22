package handler

import (
	"net/http"
)

func RequestScreenshot(svc interface{}) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusNotImplemented, map[string]string{"message": "RequestScreenshot not yet implemented"})
	}
}

func UploadScreenshot(svc interface{}) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusNotImplemented, map[string]string{"message": "UploadScreenshot not yet implemented"})
	}
}