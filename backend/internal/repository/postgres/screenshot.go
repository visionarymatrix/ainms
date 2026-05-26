package postgres

import (
	"context"
	"fmt"

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

const screenshotRequestColumns = `id, device_id, requested_by, reason, policy, status, image_path, created_at, completed_at`

func scanScreenshotRequest(scanner interface{ Scan(...interface{}) error }, s *domain.ScreenshotRequestDB) error {
	return scanner.Scan(
		&s.ID, &s.DeviceID, &s.RequestedBy, &s.Reason, &s.Policy,
		&s.Status, &s.ImagePath, &s.CreatedAt, &s.CompletedAt,
	)
}

func (r *ScreenshotRepo) Create(ctx context.Context, req *domain.ScreenshotRequestDB) error {
	query := `INSERT INTO screenshot_requests (id, device_id, requested_by, reason, policy, status, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, NOW())
		RETURNING created_at`

	if req.ID == uuid.Nil {
		req.ID = uuid.New()
	}

	return r.pool.QueryRow(ctx, query,
		req.ID, req.DeviceID, req.RequestedBy, req.Reason, req.Policy, req.Status,
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
	query := `SELECT ` + screenshotRequestColumns + ` FROM screenshot_requests WHERE device_id = $1 ORDER BY created_at DESC LIMIT 50`

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