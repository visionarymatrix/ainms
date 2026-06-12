package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type PolicyRepo struct {
	pool *pgxpool.Pool
}

func NewPolicyRepo(pool *pgxpool.Pool) *PolicyRepo {
	return &PolicyRepo{pool: pool}
}

func (r *PolicyRepo) GetByTenantID(ctx context.Context, tenantID uuid.UUID) (*domain.Policy, error) {
	query := `SELECT id, tenant_id, upload_interval, screenshot_enabled, screenshot_policy, created_at, updated_at
		FROM policies WHERE tenant_id = $1 LIMIT 1`

	var p domain.Policy
	err := r.pool.QueryRow(ctx, query, tenantID).Scan(
		&p.ID, &p.TenantID, &p.UploadInterval, &p.ScreenshotEnabled, &p.ScreenshotPolicy,
		&p.CreatedAt, &p.UpdatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("get policy by tenant_id: %w", err)
	}
	return &p, nil
}

func (r *PolicyRepo) GetDefault(ctx context.Context) (*domain.Policy, error) {
	return &domain.Policy{
		ID:                uuid.Nil,
		TenantID:          uuid.Nil,
		UploadInterval:    300,
		ScreenshotEnabled: true,
		ScreenshotPolicy:  "metadata_only",
	}, nil
}