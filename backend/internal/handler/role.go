package handler

import (
	"net/http"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

func CreateRole(svc *service.RoleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyIDStr := chi.URLParam(r, "companyID")
		companyID, err := uuid.Parse(companyIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company id")
			return
		}

		var req domain.CreateRoleRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		role, err := svc.Create(r.Context(), companyID, req)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, role)
	}
}

func GetRole(svc *service.RoleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roleIDStr := chi.URLParam(r, "roleID")
		roleID, err := uuid.Parse(roleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid role id")
			return
		}

		role, err := svc.GetByID(r.Context(), roleID)
		if err != nil {
			writeError(w, http.StatusNotFound, "role not found")
			return
		}

		writeJSON(w, http.StatusOK, role)
	}
}

func ListRoles(svc *service.RoleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyIDStr := chi.URLParam(r, "companyID")
		companyID, err := uuid.Parse(companyIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company id")
			return
		}

		roles, err := svc.List(r.Context(), companyID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, roles)
	}
}

func UpdateRole(svc *service.RoleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roleIDStr := chi.URLParam(r, "roleID")
		roleID, err := uuid.Parse(roleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid role id")
			return
		}

		var req domain.UpdateRoleRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		role, err := svc.Update(r.Context(), roleID, req)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, role)
	}
}

func DeleteRole(svc *service.RoleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roleIDStr := chi.URLParam(r, "roleID")
		roleID, err := uuid.Parse(roleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid role id")
			return
		}

		if err := svc.Delete(r.Context(), roleID); err != nil {
			writeError(w, http.StatusNotFound, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "deleted"})
	}
}

// AppClassification handlers

func ListAppClassifications(acRepo *postgres.AppClassificationRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roleIDStr := chi.URLParam(r, "roleID")
		roleID, err := uuid.Parse(roleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid role id")
			return
		}

		classifications, err := acRepo.ListByRole(r.Context(), roleID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, classifications)
	}
}

func CreateAppClassification(acRepo *postgres.AppClassificationRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roleIDStr := chi.URLParam(r, "roleID")
		roleID, err := uuid.Parse(roleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid role id")
			return
		}

		var req struct {
			AppName  string `json:"app_name"`
			Category string `json:"category"`
		}
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if req.AppName == "" {
			writeError(w, http.StatusBadRequest, "app_name is required")
			return
		}
		if req.Category == "" {
			writeError(w, http.StatusBadRequest, "category is required")
			return
		}

		ac := &domain.AppClassification{
			ID:       uuid.New(),
			RoleID:   roleID,
			AppName:  req.AppName,
			Category: req.Category,
		}

		if err := acRepo.Create(r.Context(), ac); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, ac)
	}
}

func DeleteAppClassification(acRepo *postgres.AppClassificationRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		classificationIDStr := chi.URLParam(r, "classificationID")
		classificationID, err := uuid.Parse(classificationIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid classification id")
			return
		}

		if err := acRepo.Delete(r.Context(), classificationID); err != nil {
			writeError(w, http.StatusNotFound, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "deleted"})
	}
}

// AlertRule handlers

func ListAlertRules(arRepo *postgres.AlertRuleRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roleIDStr := chi.URLParam(r, "roleID")
		roleID, err := uuid.Parse(roleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid role id")
			return
		}

		rules, err := arRepo.ListByRole(r.Context(), roleID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, rules)
	}
}

func CreateAlertRule(arRepo *postgres.AlertRuleRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roleIDStr := chi.URLParam(r, "roleID")
		roleID, err := uuid.Parse(roleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid role id")
			return
		}

		var req struct {
			Category     string `json:"category"`
			ThresholdMin int    `json:"threshold_min"`
			PopupType   string `json:"popup_type"`
		}
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if req.Category == "" {
			writeError(w, http.StatusBadRequest, "category is required")
			return
		}

		ar := &domain.AlertRule{
			ID:           uuid.New(),
			RoleID:       roleID,
			Category:     req.Category,
			ThresholdMin: req.ThresholdMin,
			PopupType:   req.PopupType,
		}

		if err := arRepo.Create(r.Context(), ar); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, ar)
	}
}

func DeleteAlertRule(arRepo *postgres.AlertRuleRepo) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ruleIDStr := chi.URLParam(r, "ruleID")
		ruleID, err := uuid.Parse(ruleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid rule id")
			return
		}

		if err := arRepo.Delete(r.Context(), ruleID); err != nil {
			writeError(w, http.StatusNotFound, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "deleted"})
	}
}