package handler

import (
	"net/http"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/middleware"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

func CreateCompany(svc *service.CompanyService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req domain.CreateCompanyRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		tenantID, err := uuid.Parse(req.TenantID)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid tenant_id")
			return
		}

		company := &domain.Company{
			TenantID: tenantID,
			Name:     req.Name,
			Plan:     req.Plan,
		}

		if err := svc.Create(r.Context(), company); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, company)
	}
}

func GetCompany(svc *service.CompanyService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyIDStr := chi.URLParam(r, "companyID")
		companyID, err := uuid.Parse(companyIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company id")
			return
		}

		company, err := svc.GetByID(r.Context(), companyID)
		if err != nil {
			writeError(w, http.StatusNotFound, "company not found")
			return
		}

		writeJSON(w, http.StatusOK, company)
	}
}

func ListCompanies(svc *service.CompanyService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		role := middleware.GetRole(r.Context())
		companyID := middleware.GetCompanyID(r.Context())

		if role == "super_admin" {
			tenantIDStr := r.URL.Query().Get("tenant_id")
			if tenantIDStr != "" {
				tenantID, err := uuid.Parse(tenantIDStr)
				if err != nil {
					writeError(w, http.StatusBadRequest, "invalid tenant_id")
					return
				}
				companies, err := svc.List(r.Context(), tenantID)
				if err != nil {
					writeError(w, http.StatusInternalServerError, err.Error())
					return
				}
				writeJSON(w, http.StatusOK, companies)
				return
			}
			companies, err := svc.ListAll(r.Context())
			if err != nil {
				writeError(w, http.StatusInternalServerError, err.Error())
				return
			}
			writeJSON(w, http.StatusOK, companies)
			return
		}

		if companyID == nil {
			writeError(w, http.StatusForbidden, "no company associated")
			return
		}

		company, err := svc.GetByID(r.Context(), uuid.MustParse(*companyID))
		if err != nil {
			writeError(w, http.StatusNotFound, "company not found")
			return
		}
		writeJSON(w, http.StatusOK, []domain.Company{*company})
	}
}