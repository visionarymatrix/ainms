package handler

import (
	"fmt"
	"net/http"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/middleware"
	"github.com/ainms/gateway/internal/service"
	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
	"github.com/robfig/cron/v3"
)

// resolveCompanyID returns the company_id from the query param if provided,
// otherwise falls back to the JWT token's company_id.
// Returns an error response if neither is available.
func resolveCompanyID(w http.ResponseWriter, r *http.Request) (uuid.UUID, bool) {
	if cid := r.URL.Query().Get("company_id"); cid != "" {
		companyID, err := uuid.Parse(cid)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid company_id query param")
			return uuid.Nil, false
		}
		return companyID, true
	}
	companyIDStr := middleware.GetCompanyID(r.Context())
	if companyIDStr == nil || *companyIDStr == "" {
		writeError(w, http.StatusBadRequest, "company_id is required (provide as query param or use a company admin account)")
		return uuid.Nil, false
	}
	companyID, err := uuid.Parse(*companyIDStr)
	if err != nil {
		writeError(w, http.StatusBadRequest, "invalid company_id in token")
		return uuid.Nil, false
	}
	return companyID, true
}

func CreateTargetedSchedule(svc *service.TargetedScheduleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req struct {
			CompanyID       string `json:"company_id"`
			EmployeeID      string `json:"employee_id"`
			Name            string `json:"name"`
			IntervalMinutes int    `json:"interval_minutes"`
			CronExpression  string `json:"cron_expression"`
			StartTime       string `json:"start_time"`
			EndTime         string `json:"end_time"`
			StartDate       string `json:"start_date"`
			EndDate         string `json:"end_date"`
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
		createdBy, err := uuid.Parse(userIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid user_id in token")
			return
		}

		var companyID uuid.UUID
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

		if req.IntervalMinutes < 1 {
			req.IntervalMinutes = 5
		}

		var cronExpression *string
		if req.CronExpression != "" {
			// Validate cron expression format (5-field: min hour day-of-month month day-of-week)
			parser := cron.NewParser(cron.Minute | cron.Hour | cron.Dom | cron.Month | cron.Dow)
			if _, err := parser.Parse(req.CronExpression); err != nil {
				writeError(w, http.StatusBadRequest, fmt.Sprintf("invalid cron_expression: %v", err))
				return
			}
			cronExpression = &req.CronExpression
		}

		if req.StartTime == "" {
			req.StartTime = "09:00"
		}
		if req.EndTime == "" {
			req.EndTime = "17:00"
		}

		var startDate, endDate time.Time
		if req.StartDate != "" {
			startDate, err = time.Parse("2006-01-02", req.StartDate)
			if err != nil {
				writeError(w, http.StatusBadRequest, "invalid start_date format, use YYYY-MM-DD")
				return
			}
		} else {
			startDate = time.Now()
		}
		if req.EndDate != "" {
			endDate, err = time.Parse("2006-01-02", req.EndDate)
			if err != nil {
				writeError(w, http.StatusBadRequest, "invalid end_date format, use YYYY-MM-DD")
				return
			}
		} else {
			endDate = startDate.AddDate(0, 0, 30)
		}

		schedule := &domain.TargetedScreenshotScheduleDB{
			CompanyID:       companyID,
			EmployeeID:      employeeID,
			CreatedBy:       createdBy,
			Name:            req.Name,
			IntervalMinutes: req.IntervalMinutes,
			CronExpression:  cronExpression,
			StartTime:       req.StartTime,
			EndTime:         req.EndTime,
			StartDate:       startDate,
			EndDate:         endDate,
			Status:          "active",
		}

		result, err := svc.CreateSchedule(r.Context(), schedule)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusCreated, result)
	}
}

func GetTargetedSchedule(svc *service.TargetedScheduleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		scheduleIDStr := chi.URLParam(r, "scheduleID")
		scheduleID, err := uuid.Parse(scheduleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid schedule_id")
			return
		}

		schedule, err := svc.GetSchedule(r.Context(), scheduleID)
		if err != nil {
			writeError(w, http.StatusNotFound, "schedule not found")
			return
		}

		writeJSON(w, http.StatusOK, schedule)
	}
}

func ListTargetedSchedules(svc *service.TargetedScheduleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyID, ok := resolveCompanyID(w, r)
		if !ok {
			return
		}

		schedules, err := svc.ListSchedulesByCompany(r.Context(), companyID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, schedules)
	}
}

func ListTargetedSchedulesByEmployee(svc *service.TargetedScheduleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		employeeIDStr := chi.URLParam(r, "employeeID")
		employeeID, err := uuid.Parse(employeeIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid employee_id")
			return
		}

		schedules, err := svc.ListSchedulesByEmployee(r.Context(), employeeID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, schedules)
	}
}

func UpdateTargetedSchedule(svc *service.TargetedScheduleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		scheduleIDStr := chi.URLParam(r, "scheduleID")
		scheduleID, err := uuid.Parse(scheduleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid schedule_id")
			return
		}

		var req struct {
			Name            *string `json:"name"`
			IntervalMinutes *int    `json:"interval_minutes"`
			CronExpression  *string `json:"cron_expression"`
			StartTime       *string `json:"start_time"`
			EndTime         *string `json:"end_time"`
			StartDate       *string `json:"start_date"`
			EndDate         *string `json:"end_date"`
			Status          *string `json:"status"`
		}
		if err := decodeJSON(r, &req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		schedule, err := svc.GetSchedule(r.Context(), scheduleID)
		if err != nil {
			writeError(w, http.StatusNotFound, "schedule not found")
			return
		}

		if req.Name != nil {
			schedule.Name = *req.Name
		}
		if req.IntervalMinutes != nil {
			schedule.IntervalMinutes = *req.IntervalMinutes
		}
		if req.CronExpression != nil {
			if *req.CronExpression != "" {
				parser := cron.NewParser(cron.Minute | cron.Hour | cron.Dom | cron.Month | cron.Dow)
				if _, err := parser.Parse(*req.CronExpression); err != nil {
					writeError(w, http.StatusBadRequest, fmt.Sprintf("invalid cron_expression: %v", err))
					return
				}
			}
			schedule.CronExpression = req.CronExpression
		}
		if req.StartTime != nil {
			schedule.StartTime = *req.StartTime
		}
		if req.EndTime != nil {
			schedule.EndTime = *req.EndTime
		}
		if req.StartDate != nil {
			sd, err := time.Parse("2006-01-02", *req.StartDate)
			if err != nil {
				writeError(w, http.StatusBadRequest, "invalid start_date format, use YYYY-MM-DD")
				return
			}
			schedule.StartDate = sd
		}
		if req.EndDate != nil {
			ed, err := time.Parse("2006-01-02", *req.EndDate)
			if err != nil {
				writeError(w, http.StatusBadRequest, "invalid end_date format, use YYYY-MM-DD")
				return
			}
			schedule.EndDate = ed
		}
		if req.Status != nil {
			schedule.Status = *req.Status
		}

		if err := svc.UpdateSchedule(r.Context(), schedule); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, schedule)
	}
}

func DeleteTargetedSchedule(svc *service.TargetedScheduleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		scheduleIDStr := chi.URLParam(r, "scheduleID")
		scheduleID, err := uuid.Parse(scheduleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid schedule_id")
			return
		}

		if err := svc.DeleteSchedule(r.Context(), scheduleID); err != nil {
			writeError(w, http.StatusNotFound, "schedule not found")
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "deleted"})
	}
}

func GetTargetedScreenshots(svc *service.TargetedScheduleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		companyID, ok := resolveCompanyID(w, r)
		if !ok {
			return
		}

		var scheduleID *uuid.UUID
		if sid := r.URL.Query().Get("schedule_id"); sid != "" {
			id, err := uuid.Parse(sid)
			if err == nil {
				scheduleID = &id
			}
		}

		var employeeID *uuid.UUID
		if eid := r.URL.Query().Get("employee_id"); eid != "" {
			id, err := uuid.Parse(eid)
			if err == nil {
				employeeID = &id
			}
		}

		var fromDate *time.Time
		if from := r.URL.Query().Get("from"); from != "" {
			t, err := time.Parse(time.RFC3339, from)
			if err == nil {
				fromDate = &t
			}
		}

		var toDate *time.Time
		if to := r.URL.Query().Get("to"); to != "" {
			t, err := time.Parse(time.RFC3339, to)
			if err == nil {
				toDate = &t
			}
		}

		screenshots, err := svc.GetScreenshotsByCompanyWithFilters(r.Context(), companyID, scheduleID, employeeID, fromDate, toDate)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, screenshots)
	}
}

func GetScheduleScreenshots(svc *service.TargetedScheduleService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		scheduleIDStr := chi.URLParam(r, "scheduleID")
		scheduleID, err := uuid.Parse(scheduleIDStr)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid schedule_id")
			return
		}

		screenshots, err := svc.GetScreenshotsBySchedule(r.Context(), scheduleID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, screenshots)
	}
}