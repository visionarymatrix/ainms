package postgres

import (
	"context"
	"fmt"
	"log"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type TargetedScheduleRepo struct {
	pool *pgxpool.Pool
}

func NewTargetedScheduleRepo(pool *pgxpool.Pool) *TargetedScheduleRepo {
	r := &TargetedScheduleRepo{pool: pool}
	// Ensure cron_expression column exists for backward compatibility
	r.ensureCronColumn(context.Background())
	return r
}

const targetedScheduleColumns = `id, company_id, employee_id, created_by, name, interval_minutes, cron_expression, start_time, end_time, start_date, end_date, status, last_triggered_at, created_at, updated_at`

func scanTargetedSchedule(scanner interface{ Scan(...interface{}) error }, s *domain.TargetedScreenshotScheduleDB) error {
	return scanner.Scan(
		&s.ID, &s.CompanyID, &s.EmployeeID, &s.CreatedBy, &s.Name,
		&s.IntervalMinutes, &s.CronExpression, &s.StartTime, &s.EndTime,
		&s.StartDate, &s.EndDate, &s.Status, &s.LastTriggeredAt,
		&s.CreatedAt, &s.UpdatedAt,
	)
}

func (r *TargetedScheduleRepo) ensureCronColumn(ctx context.Context) {
	_, err := r.pool.Exec(ctx, `ALTER TABLE targeted_screenshot_schedules ADD COLUMN IF NOT EXISTS cron_expression TEXT`)
	if err != nil {
		log.Printf("[targeted-schedule] Failed to ensure cron_expression column: %v", err)
	}
}

func (r *TargetedScheduleRepo) Create(ctx context.Context, schedule *domain.TargetedScreenshotScheduleDB) error {
	query := `INSERT INTO targeted_screenshot_schedules
		(company_id, employee_id, created_by, name, interval_minutes, cron_expression, start_time, end_time, start_date, end_date, status)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
		RETURNING id, created_at, updated_at`

	return r.pool.QueryRow(ctx, query,
		schedule.CompanyID, schedule.EmployeeID, schedule.CreatedBy, schedule.Name,
		schedule.IntervalMinutes, schedule.CronExpression, schedule.StartTime, schedule.EndTime,
		schedule.StartDate, schedule.EndDate, schedule.Status,
	).Scan(&schedule.ID, &schedule.CreatedAt, &schedule.UpdatedAt)
}

func (r *TargetedScheduleRepo) GetByID(ctx context.Context, id uuid.UUID) (*domain.TargetedScreenshotScheduleDB, error) {
	query := `SELECT ` + targetedScheduleColumns + ` FROM targeted_screenshot_schedules WHERE id = $1`

	var s domain.TargetedScreenshotScheduleDB
	if err := scanTargetedSchedule(r.pool.QueryRow(ctx, query, id), &s); err != nil {
		return nil, fmt.Errorf("get targeted schedule: %w", err)
	}
	return &s, nil
}

func (r *TargetedScheduleRepo) ListByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.TargetedScreenshotScheduleDB, error) {
	query := `SELECT ` + targetedScheduleColumns + ` FROM targeted_screenshot_schedules WHERE company_id = $1 ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query, companyID)
	if err != nil {
		return nil, fmt.Errorf("list targeted schedules by company: %w", err)
	}
	defer rows.Close()

	schedules := make([]domain.TargetedScreenshotScheduleDB, 0)
	for rows.Next() {
		var s domain.TargetedScreenshotScheduleDB
		if err := scanTargetedSchedule(rows, &s); err != nil {
			return nil, fmt.Errorf("scan targeted schedule: %w", err)
		}
		schedules = append(schedules, s)
	}
	return schedules, rows.Err()
}

func (r *TargetedScheduleRepo) ListByEmployee(ctx context.Context, employeeID uuid.UUID) ([]domain.TargetedScreenshotScheduleDB, error) {
	query := `SELECT ` + targetedScheduleColumns + ` FROM targeted_screenshot_schedules WHERE employee_id = $1 ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query, employeeID)
	if err != nil {
		return nil, fmt.Errorf("list targeted schedules by employee: %w", err)
	}
	defer rows.Close()

	schedules := make([]domain.TargetedScreenshotScheduleDB, 0)
	for rows.Next() {
		var s domain.TargetedScreenshotScheduleDB
		if err := scanTargetedSchedule(rows, &s); err != nil {
			return nil, fmt.Errorf("scan targeted schedule: %w", err)
		}
		schedules = append(schedules, s)
	}
	return schedules, rows.Err()
}

func (r *TargetedScheduleRepo) ListActive(ctx context.Context) ([]domain.TargetedScreenshotScheduleDB, error) {
	query := `SELECT ` + targetedScheduleColumns + ` FROM targeted_screenshot_schedules
		WHERE status = 'active'
		AND start_date <= NOW()
		AND end_date >= NOW()
		ORDER BY created_at ASC`

	rows, err := r.pool.Query(ctx, query)
	if err != nil {
		return nil, fmt.Errorf("list active targeted schedules: %w", err)
	}
	defer rows.Close()

	schedules := make([]domain.TargetedScreenshotScheduleDB, 0)
	for rows.Next() {
		var s domain.TargetedScreenshotScheduleDB
		if err := scanTargetedSchedule(rows, &s); err != nil {
			return nil, fmt.Errorf("scan targeted schedule: %w", err)
		}
		schedules = append(schedules, s)
	}
	return schedules, rows.Err()
}

func (r *TargetedScheduleRepo) Update(ctx context.Context, schedule *domain.TargetedScreenshotScheduleDB) error {
	query := `UPDATE targeted_screenshot_schedules
		SET name = $2, interval_minutes = $3, cron_expression = $4, start_time = $5, end_time = $6,
		    start_date = $7, end_date = $8, status = $9, updated_at = NOW()
		WHERE id = $1`

	result, err := r.pool.Exec(ctx, query,
		schedule.ID, schedule.Name, schedule.IntervalMinutes, schedule.CronExpression, schedule.StartTime, schedule.EndTime,
		schedule.StartDate, schedule.EndDate, schedule.Status,
	)
	if err != nil {
		return fmt.Errorf("update targeted schedule: %w", err)
	}
	if result.RowsAffected() == 0 {
		return fmt.Errorf("targeted schedule not found")
	}
	return nil
}

func (r *TargetedScheduleRepo) UpdateLastTriggered(ctx context.Context, id uuid.UUID) error {
	query := `UPDATE targeted_screenshot_schedules SET last_triggered_at = NOW(), updated_at = NOW() WHERE id = $1`
	_, err := r.pool.Exec(ctx, query, id)
	return err
}

func (r *TargetedScheduleRepo) Delete(ctx context.Context, id uuid.UUID) error {
	query := `DELETE FROM targeted_screenshot_schedules WHERE id = $1`
	result, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("delete targeted schedule: %w", err)
	}
	if result.RowsAffected() == 0 {
		return fmt.Errorf("targeted schedule not found")
	}
	return nil
}

func (r *TargetedScheduleRepo) MarkExpired(ctx context.Context) (int, error) {
	query := `UPDATE targeted_screenshot_schedules SET status = 'expired', updated_at = NOW()
		WHERE status = 'active' AND end_date < NOW()`
	result, err := r.pool.Exec(ctx, query)
	if err != nil {
		return 0, fmt.Errorf("mark expired schedules: %w", err)
	}
	return int(result.RowsAffected()), nil
}
