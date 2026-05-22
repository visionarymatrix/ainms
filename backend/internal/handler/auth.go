package handler

import (
	"net/http"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/middleware"
	"github.com/ainms/gateway/internal/service"
)

func Login(authSvc *service.AuthService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req domain.LoginRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		resp, err := authSvc.Login(r.Context(), req.Email, req.Password)
		if err != nil {
			writeError(w, http.StatusUnauthorized, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, resp)
	}
}

func RegisterCompany(authSvc *service.AuthService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req domain.RegisterCompanyRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		resp, err := authSvc.RegisterCompany(r.Context(), req)
		if err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, resp)
	}
}

func GetCurrentUser(authSvc *service.AuthService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		userID := middleware.GetUserID(r.Context())
		if userID == "" {
			writeError(w, http.StatusUnauthorized, "not authenticated")
			return
		}

		user, err := authSvc.GetUserByID(r.Context(), userID)
		if err != nil {
			writeError(w, http.StatusNotFound, "user not found")
			return
		}

		writeJSON(w, http.StatusOK, user)
	}
}