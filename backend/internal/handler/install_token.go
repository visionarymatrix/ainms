package handler

import (
	"net/http"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/middleware"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

func GenerateInstallToken(svc *service.InstallTokenService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		role := middleware.GetRole(r.Context())
		if role != "company_admin" && role != "super_admin" {
			writeError(w, http.StatusForbidden, "only company_admin or super_admin can generate install tokens")
			return
		}

		userIDStr := middleware.GetUserID(r.Context())
		createdBy, err := uuid.Parse(userIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid user_id in token")
			return
		}

		var req domain.CreateInstallTokenRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if req.EmployeeID == "" {
			writeError(w, http.StatusBadRequest, "employee_id is required")
			return
		}
		if req.CompanyID == "" {
			writeError(w, http.StatusBadRequest, "company_id is required")
			return
		}

		resp, err := svc.Generate(r.Context(), req, createdBy)
		if err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, resp)
	}
}

func ListInstallTokens(svc *service.InstallTokenService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		role := middleware.GetRole(r.Context())
		if role != "company_admin" && role != "super_admin" {
			writeError(w, http.StatusForbidden, "only company_admin or super_admin can list install tokens")
			return
		}

		companyIDStr := middleware.GetCompanyID(r.Context())
		if companyIDStr == nil || *companyIDStr == "" {
			writeError(w, http.StatusBadRequest, "company_id is required")
			return
		}

		companyID, err := uuid.Parse(*companyIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company_id")
			return
		}

		tokens, err := svc.ListByCompany(r.Context(), companyID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, tokens)
	}
}

func RevokeInstallToken(svc *service.InstallTokenService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		role := middleware.GetRole(r.Context())
		if role != "company_admin" && role != "super_admin" {
			writeError(w, http.StatusForbidden, "only company_admin or super_admin can revoke install tokens")
			return
		}

		tokenIDStr := chi.URLParam(r, "tokenID")
		tokenID, err := uuid.Parse(tokenIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid token_id")
			return
		}

		if err := svc.Revoke(r.Context(), tokenID); err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "revoked"})
	}
}