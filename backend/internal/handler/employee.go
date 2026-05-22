package handler

import (
	"net/http"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
)

func RegisterEmployee(svc *service.EmployeeService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyIDStr := chi.URLParam(r, "companyID")
		companyID, err := uuid.Parse(companyIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company id")
			return
		}

		var req domain.RegisterEmployeeRequest
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		emp, err := svc.Register(r.Context(), companyID, req)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, emp)
	}
}

func GetEmployee(svc *service.EmployeeService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee id")
			return
		}

		emp, err := svc.GetByID(r.Context(), employeeID)
		if err != nil {
			writeError(w, http.StatusNotFound, "employee not found")
			return
		}

		writeJSON(w, http.StatusOK, emp)
	}
}

func ListEmployees(svc *service.EmployeeService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyIDStr := chi.URLParam(r, "companyID")
		companyID, err := uuid.Parse(companyIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company id")
			return
		}

		employees, err := svc.List(r.Context(), companyID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, employees)
	}
}

func UpdateEmployee(svc *service.EmployeeService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusNotImplemented, map[string]string{"message": "UpdateEmployee not yet implemented"})
	}
}

func DeactivateEmployee(svc *service.EmployeeService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee id")
			return
		}

		if err := svc.Deactivate(r.Context(), employeeID); err != nil {
			writeError(w, http.StatusNotFound, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "deactivated"})
	}
}