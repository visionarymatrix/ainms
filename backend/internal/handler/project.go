package handler

import (
	"net/http"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/middleware"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

func CreateProject(svc *service.ProjectService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req struct {
			CompanyID   string   `json:"company_id"`
			Name        string   `json:"name"`
			Description *string  `json:"description"`
			Status      string   `json:"status"`
			WorkingApps []string `json:"working_apps"`
		}
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if req.Name == "" {
			writeError(w, http.StatusBadRequest, "name is required")
			return
		}

		var companyID uuid.UUID
		var err error
		if req.CompanyID != "" {
			companyID, err = uuid.Parse(req.CompanyID)
			if err != nil {
				writeError(w, http.StatusBadRequest, "invalid company_id")
				return
			}
		} else {
			companyIDStr := middleware.GetCompanyID(r.Context())
			if companyIDStr == nil || *companyIDStr == "" {
				writeError(w, http.StatusBadRequest, "company_id is required (provide in request body or use a company admin account)")
				return
			}
			companyID, err = uuid.Parse(*companyIDStr)
			if err != nil {
				writeError(w, http.StatusBadRequest, "invalid company_id in token")
				return
			}
		}

		project := &domain.Project{
			CompanyID:   companyID,
			Name:        req.Name,
			Description: req.Description,
			Status:      req.Status,
			WorkingApps: req.WorkingApps,
		}

		result, err := svc.CreateProject(r.Context(), project)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, result)
	}
}

func ListProjects(svc *service.ProjectService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyID, ok := resolveCompanyID(w, r)
		if !ok {
			return
		}

		projects, err := svc.ListProjectsByCompany(r.Context(), companyID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, projects)
	}
}

func GetProject(svc *service.ProjectService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		projectIDStr := chi.URLParam(r, "projectID")
		projectID, err := uuid.Parse(projectIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid project_id")
			return
		}

		project, err := svc.GetProject(r.Context(), projectID)
		if err != nil {
			writeError(w, http.StatusNotFound, "project not found")
			return
		}

		writeJSON(w, http.StatusOK, project)
	}
}

func UpdateProject(svc *service.ProjectService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		projectIDStr := chi.URLParam(r, "projectID")
		projectID, err := uuid.Parse(projectIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid project_id")
			return
		}

		var req struct {
			Name        *string  `json:"name"`
			Description *string  `json:"description"`
			Status      *string  `json:"status"`
			WorkingApps []string `json:"working_apps"`
		}
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		project, err := svc.GetProject(r.Context(), projectID)
		if err != nil {
			writeError(w, http.StatusNotFound, "project not found")
			return
		}

		if req.Name != nil {
			project.Name = *req.Name
		}
		if req.Description != nil {
			project.Description = req.Description
		}
		if req.Status != nil {
			project.Status = *req.Status
		}
		if req.WorkingApps != nil {
			project.WorkingApps = req.WorkingApps
		}

		if err := svc.UpdateProject(r.Context(), project); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, project)
	}
}

func DeleteProject(svc *service.ProjectService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		projectIDStr := chi.URLParam(r, "projectID")
		projectID, err := uuid.Parse(projectIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid project_id")
			return
		}

		if err := svc.DeleteProject(r.Context(), projectID); err != nil {
			writeError(w, http.StatusNotFound, "project not found")
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "deleted"})
	}
}

func AssignEmployeeToProject(svc *service.ProjectService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		projectIDStr := chi.URLParam(r, "projectID")
		projectID, err := uuid.Parse(projectIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid project_id")
			return
		}

		var req struct {
			EmployeeID string `json:"employee_id"`
			IsPrimary  bool   `json:"is_primary"`
		}
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		employeeID, err := uuid.Parse(req.EmployeeID)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee_id")
			return
		}

		userIDStr := middleware.GetUserID(r.Context())
		assignedBy, err := uuid.Parse(userIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid user_id in token")
			return
		}

		assignment := &domain.EmployeeProjectAssignment{
			EmployeeID: employeeID,
			ProjectID:  projectID,
			AssignedBy: assignedBy,
			IsPrimary:  req.IsPrimary,
		}

		result, err := svc.AssignEmployee(r.Context(), assignment)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, result)
	}
}

func UnassignEmployeeFromProject(svc *service.ProjectService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		projectIDStr := chi.URLParam(r, "projectID")
		projectID, err := uuid.Parse(projectIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid project_id")
			return
		}

		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee_id")
			return
		}

		if err := svc.UnassignEmployee(r.Context(), employeeID, projectID); err != nil {
			writeError(w, http.StatusNotFound, "assignment not found")
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "unassigned"})
	}
}

func ListEmployeeProjects(svc *service.ProjectService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee_id")
			return
		}

		assignments, err := svc.ListEmployeeProjects(r.Context(), employeeID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, assignments)
	}
}

func GetAIActivityAnalysis(complianceSvc *service.ComplianceService, projectSvc *service.ProjectService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee_id")
			return
		}

		startTimeStr := r.URL.Query().Get("start_time")
		endTimeStr := r.URL.Query().Get("end_time")
		if startTimeStr == "" || endTimeStr == "" {
			writeError(w, http.StatusBadRequest, "start_time and end_time query params are required")
			return
		}

		result, err := complianceSvc.AnalyzeActivity(r.Context(), projectSvc, employeeID, startTimeStr, endTimeStr)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, result)
	}
}