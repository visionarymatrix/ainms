package service

import (
	"context"
	"fmt"
	"log"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/google/uuid"
	"github.com/robfig/cron/v3"
)

type TargetedScheduleService struct {
	scheduleRepo    *postgres.TargetedScheduleRepo
	employeeRepo    *postgres.EmployeeRepo
	deviceRepo      *postgres.DeviceRepo
	screenshotSvc   *ScreenshotService
	socketHub       *SocketHub
	screenshotRepo  *postgres.ScreenshotRepo
}

func NewTargetedScheduleService(
	scheduleRepo *postgres.TargetedScheduleRepo,
	employeeRepo *postgres.EmployeeRepo,
	deviceRepo *postgres.DeviceRepo,
	screenshotSvc *ScreenshotService,
	socketHub *SocketHub,
	screenshotRepo *postgres.ScreenshotRepo,
) *TargetedScheduleService {
	return &TargetedScheduleService{
		scheduleRepo:   scheduleRepo,
		employeeRepo:   employeeRepo,
		deviceRepo:     deviceRepo,
		screenshotSvc:  screenshotSvc,
		socketHub:      socketHub,
		screenshotRepo: screenshotRepo,
	}
}

func (s *TargetedScheduleService) CreateSchedule(ctx context.Context, schedule *domain.TargetedScreenshotScheduleDB) (*domain.TargetedScreenshotScheduleDB, error) {
	if schedule.IntervalMinutes < 1 {
		return nil, fmt.Errorf("interval_minutes must be at least 1")
	}
	if schedule.StartTime == "" {
		schedule.StartTime = "09:00"
	}
	if schedule.EndTime == "" {
		schedule.EndTime = "17:00"
	}
	if schedule.Status == "" {
		schedule.Status = "active"
	}
	if err := s.scheduleRepo.Create(ctx, schedule); err != nil {
		return nil, fmt.Errorf("create targeted schedule: %w", err)
	}
	return schedule, nil
}

func (s *TargetedScheduleService) GetSchedule(ctx context.Context, id uuid.UUID) (*domain.TargetedScreenshotScheduleDB, error) {
	return s.scheduleRepo.GetByID(ctx, id)
}

func (s *TargetedScheduleService) ListSchedulesByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.TargetedScreenshotScheduleDB, error) {
	return s.scheduleRepo.ListByCompany(ctx, companyID)
}

func (s *TargetedScheduleService) ListSchedulesByEmployee(ctx context.Context, employeeID uuid.UUID) ([]domain.TargetedScreenshotScheduleDB, error) {
	return s.scheduleRepo.ListByEmployee(ctx, employeeID)
}

func (s *TargetedScheduleService) UpdateSchedule(ctx context.Context, schedule *domain.TargetedScreenshotScheduleDB) error {
	return s.scheduleRepo.Update(ctx, schedule)
}

func (s *TargetedScheduleService) DeleteSchedule(ctx context.Context, id uuid.UUID) error {
	return s.scheduleRepo.Delete(ctx, id)
}

func (s *TargetedScheduleService) GetScreenshotsBySchedule(ctx context.Context, scheduleID uuid.UUID) ([]domain.ScreenshotRequestDB, error) {
	return s.screenshotRepo.ListByScheduleID(ctx, scheduleID)
}

func (s *TargetedScheduleService) GetScreenshotsByCompanyWithFilters(ctx context.Context, companyID uuid.UUID, scheduleID *uuid.UUID, employeeID *uuid.UUID, fromDate *time.Time, toDate *time.Time) ([]domain.ScreenshotRequestDB, error) {
	return s.screenshotRepo.ListByCompanyWithFilters(ctx, companyID, scheduleID, employeeID, fromDate, toDate)
}

// StartScheduler runs a background goroutine that checks active schedules every minute
// and triggers screenshot requests for eligible employees.
func (s *TargetedScheduleService) StartScheduler(ctx context.Context) {
	go func() {
		ticker := time.NewTicker(1 * time.Minute)
		defer ticker.Stop()

		for {
			select {
			case <-ctx.Done():
				log.Println("[targeted-scheduler] Stopping scheduler")
				return
			case <-ticker.C:
				s.tick(ctx)
			}
		}
	}()
	log.Println("[targeted-scheduler] Scheduler started, checking every 1 minute")
}

func (s *TargetedScheduleService) tick(ctx context.Context) {
	expired, err := s.scheduleRepo.MarkExpired(ctx)
	if err != nil {
		log.Printf("[targeted-scheduler] Error marking expired schedules: %v", err)
	}
	if expired > 0 {
		log.Printf("[targeted-scheduler] Marked %d schedules as expired", expired)
	}

	schedules, err := s.scheduleRepo.ListActive(ctx)
	if err != nil {
		log.Printf("[targeted-scheduler] Error listing active schedules: %v", err)
		return
	}

	now := time.Now()
	currentTime := now.Format("15:04")
	currentDate := now.Format("2006-01-02")

	for _, schedule := range schedules {
		if !s.shouldTrigger(schedule, currentTime, currentDate, now) {
			continue
		}

		if err := s.triggerSchedule(ctx, schedule, now); err != nil {
			log.Printf("[targeted-scheduler] Error triggering schedule %s: %v", schedule.ID, err)
		}
	}
}

func (s *TargetedScheduleService) shouldTrigger(schedule domain.TargetedScreenshotScheduleDB, currentTime string, currentDate string, now time.Time) bool {
	if schedule.Status != "active" {
		return false
	}

	dateOnly := schedule.StartDate.Format("2006-01-02")
	if dateOnly > currentDate {
		return false
	}
	endDateOnly := schedule.EndDate.Format("2006-01-02")
	if endDateOnly < currentDate {
		return false
	}

	if schedule.CronExpression != nil && *schedule.CronExpression != "" {
		parser := cron.NewParser(cron.Minute | cron.Hour | cron.Dom | cron.Month | cron.Dow)
		scheduleObj, err := parser.Parse(*schedule.CronExpression)
		if err != nil {
			log.Printf("[targeted-scheduler] Invalid cron expression '%s' for schedule %s: %v", *schedule.CronExpression, schedule.ID, err)
			return false
		}
		nextRun := scheduleObj.Next(now.Add(-time.Minute))
		if nextRun.After(now) || nextRun.Add(time.Minute).Before(now) {
			return false
		}

		if currentTime < schedule.StartTime || currentTime >= schedule.EndTime {
			return false
		}

		return true
	}

	if currentTime < schedule.StartTime || currentTime >= schedule.EndTime {
		return false
	}

	if schedule.LastTriggeredAt != nil {
		elapsed := now.Sub(*schedule.LastTriggeredAt)
		minInterval := time.Duration(schedule.IntervalMinutes) * time.Minute
		if elapsed < minInterval {
			return false
		}
	}

	return true
}

func (s *TargetedScheduleService) triggerSchedule(ctx context.Context, schedule domain.TargetedScreenshotScheduleDB, now time.Time) error {
	employee, err := s.employeeRepo.GetByID(ctx, schedule.EmployeeID)
	if err != nil {
		return fmt.Errorf("get employee: %w", err)
	}

	devices, err := s.deviceRepo.GetByEmployeeID(ctx, schedule.EmployeeID)
	if err != nil {
		return fmt.Errorf("get employee devices: %w", err)
	}

	if len(devices) == 0 {
		log.Printf("[targeted-scheduler] No devices found for employee %s, skipping", schedule.EmployeeID)
		if err := s.scheduleRepo.UpdateLastTriggered(ctx, schedule.ID); err != nil {
			log.Printf("[targeted-scheduler] Failed to update last_triggered_at: %v", err)
		}
		return nil
	}

	triggered := 0
	for _, device := range devices {
		if device.Status != "active" {
			continue
		}

		reason := fmt.Sprintf("Targeted screenshot schedule: %s", schedule.Name)
		if schedule.Name == "" {
			reason = fmt.Sprintf("Scheduled screenshot for employee %s %s", employee.FirstName, employee.LastName)
		}

		req, err := s.screenshotSvc.RequestTargetedScreenshot(ctx, device.ID, schedule.CreatedBy, reason, "upload_image", schedule.ID)
		if err != nil {
			log.Printf("[targeted-scheduler] Error creating screenshot request for device %s: %v", device.ID, err)
			continue
		}

		if err := s.socketHub.SendToAgent(device.ID.String(), "screenshot_request", map[string]interface{}{
			"request_id":   req.ID.String(),
			"device_id":    device.ID.String(),
			"reason":       reason,
			"policy":       "upload_image",
			"requested_by": schedule.CreatedBy.String(),
			"schedule_id":  schedule.ID.String(),
		}); err != nil {
			log.Printf("[targeted-scheduler] Failed to send screenshot_request via Socket.IO to device %s: %v", device.ID, err)
		}

		triggered++
	}

	if triggered > 0 {
		log.Printf("[targeted-scheduler] Triggered %d screenshot(s) for schedule %s (employee %s)", triggered, schedule.ID, schedule.EmployeeID)
	}

	if err := s.scheduleRepo.UpdateLastTriggered(ctx, schedule.ID); err != nil {
		log.Printf("[targeted-scheduler] Failed to update last_triggered_at: %v", err)
	}

	return nil
}