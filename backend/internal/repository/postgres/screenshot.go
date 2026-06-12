package postgres

import (
	"context"
	"fmt"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type ScreenshotRepo struct {
	pool *pgxpool.Pool
}

func NewScreenshotRepo(pool *pgxpool.Pool) *ScreenshotRepo {
	return &ScreenshotRepo{pool: pool}
}

const screenshotRequestColumns = `id, device_id, requested_by, reason, policy, status, image_path, schedule_id, created_at, completed_at`
const screenshotRequestColumnsPrefixed = `sr.id, sr.device_id, sr.requested_by, sr.reason, sr.policy, sr.status, sr.image_path, sr.schedule_id, sr.created_at, sr.completed_at`

func scanScreenshotRequest(scanner interface{ Scan(...interface{}) error }, s *domain.ScreenshotRequestDB) error {
	return scanner.Scan(
		&s.ID, &s.DeviceID, &s.RequestedBy, &s.Reason, &s.Policy,
		&s.Status, &s.ImagePath, &s.ScheduleID, &s.CreatedAt, &s.CompletedAt,
	)
}

func (r *ScreenshotRepo) Create(ctx context.Context, req *domain.ScreenshotRequestDB) error {
	query := `INSERT INTO screenshot_requests (id, device_id, requested_by, reason, policy, status, schedule_id, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
		RETURNING created_at`

	if req.ID == uuid.Nil {
		req.ID = uuid.New()
	}

	return r.pool.QueryRow(ctx, query,
		req.ID, req.DeviceID, req.RequestedBy, req.Reason, req.Policy, req.Status, req.ScheduleID,
	).Scan(&req.CreatedAt)
}

func (r *ScreenshotRepo) GetByID(ctx context.Context, id uuid.UUID) (*domain.ScreenshotRequestDB, error) {
	query := `SELECT ` + screenshotRequestColumns + ` FROM screenshot_requests WHERE id = $1`

	var s domain.ScreenshotRequestDB
	if err := scanScreenshotRequest(r.pool.QueryRow(ctx, query, id), &s); err != nil {
		return nil, fmt.Errorf("get screenshot request: %w", err)
	}
	return &s, nil
}

func (r *ScreenshotRepo) ListByDevice(ctx context.Context, deviceID uuid.UUID) ([]domain.ScreenshotRequestDB, error) {
	query := `SELECT ` + screenshotRequestColumns + ` FROM screenshot_requests WHERE device_id = $1 AND image_path IS NOT NULL ORDER BY created_at DESC LIMIT 50`

	rows, err := r.pool.Query(ctx, query, deviceID)
	if err != nil {
		return nil, fmt.Errorf("list screenshots by device: %w", err)
	}
	defer rows.Close()

	screenshots := make([]domain.ScreenshotRequestDB, 0)
	for rows.Next() {
		var s domain.ScreenshotRequestDB
		if err := scanScreenshotRequest(rows, &s); err != nil {
			return nil, fmt.Errorf("scan screenshot request: %w", err)
		}
		screenshots = append(screenshots, s)
	}
	return screenshots, rows.Err()
}

func (r *ScreenshotRepo) UpdateStatus(ctx context.Context, id uuid.UUID, status string, imagePath *string) error {
	if imagePath != nil {
		query := `UPDATE screenshot_requests SET status = $2, image_path = $3, completed_at = NOW() WHERE id = $1`
		_, err := r.pool.Exec(ctx, query, id, status, *imagePath)
		if err != nil {
			return fmt.Errorf("update screenshot status: %w", err)
		}
	} else {
		query := `UPDATE screenshot_requests SET status = $2 WHERE id = $1`
		_, err := r.pool.Exec(ctx, query, id, status)
		if err != nil {
			return fmt.Errorf("update screenshot status: %w", err)
		}
	}
	return nil
}

func (r *ScreenshotRepo) ListPendingByDevice(ctx context.Context, deviceID uuid.UUID) ([]domain.ScreenshotRequestDB, error) {
	query := `SELECT ` + screenshotRequestColumns + ` FROM screenshot_requests WHERE device_id = $1 AND status = 'pending' ORDER BY created_at ASC`

	rows, err := r.pool.Query(ctx, query, deviceID)
	if err != nil {
		return nil, fmt.Errorf("list pending screenshots: %w", err)
	}
	defer rows.Close()

	screenshots := make([]domain.ScreenshotRequestDB, 0)
	for rows.Next() {
		var s domain.ScreenshotRequestDB
		if err := scanScreenshotRequest(rows, &s); err != nil {
			return nil, fmt.Errorf("scan screenshot request: %w", err)
		}
		screenshots = append(screenshots, s)
	}
	return screenshots, rows.Err()
}

func (r *ScreenshotRepo) ClearImagePath(ctx context.Context, id uuid.UUID) error {
	query := `UPDATE screenshot_requests SET image_path = NULL WHERE id = $1`
	_, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("clear screenshot image_path: %w", err)
	}
	return nil
}

func (r *ScreenshotRepo) ListCompletedWithImages(ctx context.Context, olderThanMinutes int) ([]domain.ScreenshotRequestDB, error) {
	query := `SELECT ` + screenshotRequestColumns + ` FROM screenshot_requests WHERE status = 'completed' AND image_path IS NOT NULL AND completed_at < NOW() - ($1 * INTERVAL '1 minute') ORDER BY created_at ASC`

	rows, err := r.pool.Query(ctx, query, olderThanMinutes)
	if err != nil {
		return nil, fmt.Errorf("list completed screenshots with images: %w", err)
	}
	defer rows.Close()

	screenshots := make([]domain.ScreenshotRequestDB, 0)
	for rows.Next() {
		var s domain.ScreenshotRequestDB
		if err := scanScreenshotRequest(rows, &s); err != nil {
			return nil, fmt.Errorf("scan screenshot request: %w", err)
		}
		screenshots = append(screenshots, s)
	}
	return screenshots, rows.Err()
}

func (r *ScreenshotRepo) ListByScheduleID(ctx context.Context, scheduleID uuid.UUID) ([]domain.ScreenshotRequestDB, error) {
	query := `SELECT ` + screenshotRequestColumns + ` FROM screenshot_requests WHERE schedule_id = $1 ORDER BY created_at DESC LIMIT 100`

	rows, err := r.pool.Query(ctx, query, scheduleID)
	if err != nil {
		return nil, fmt.Errorf("list screenshots by schedule: %w", err)
	}
	defer rows.Close()

	screenshots := make([]domain.ScreenshotRequestDB, 0)
	for rows.Next() {
		var s domain.ScreenshotRequestDB
		if err := scanScreenshotRequest(rows, &s); err != nil {
			return nil, fmt.Errorf("scan screenshot request: %w", err)
		}
		screenshots = append(screenshots, s)
	}
	return screenshots, rows.Err()
}

func (r *ScreenshotRepo) ListByCompanyWithFilters(ctx context.Context, companyID uuid.UUID, scheduleID *uuid.UUID, employeeID *uuid.UUID, fromDate *time.Time, toDate *time.Time) ([]domain.ScreenshotRequestDB, error) {
	query := `SELECT ` + screenshotRequestColumnsPrefixed + ` FROM screenshot_requests sr
		JOIN devices d ON sr.device_id = d.id
		JOIN employees e ON d.employee_id = e.id
		WHERE e.company_id = $1`

	args := []interface{}{companyID}
	argNum := 2

	if scheduleID != nil {
		query += fmt.Sprintf(` AND sr.schedule_id = $%d`, argNum)
		args = append(args, *scheduleID)
		argNum++
	}

	if employeeID != nil {
		query += fmt.Sprintf(` AND d.employee_id = $%d`, argNum)
		args = append(args, *employeeID)
		argNum++
	}

	if fromDate != nil {
		query += fmt.Sprintf(` AND sr.created_at >= $%d`, argNum)
		args = append(args, *fromDate)
		argNum++
	}

	if toDate != nil {
		query += fmt.Sprintf(` AND sr.created_at <= $%d`, argNum)
		args = append(args, *toDate)
		argNum++
	}

	query += ` ORDER BY sr.created_at DESC LIMIT 500`

	rows, err := r.pool.Query(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("list screenshots by company with filters: %w", err)
	}
	defer rows.Close()

	screenshots := make([]domain.ScreenshotRequestDB, 0)
	for rows.Next() {
		var s domain.ScreenshotRequestDB
		if err := scanScreenshotRequest(rows, &s); err != nil {
			return nil, fmt.Errorf("scan screenshot request: %w", err)
		}
		screenshots = append(screenshots, s)
	}
	return screenshots, rows.Err()
}