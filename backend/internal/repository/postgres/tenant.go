package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type TenantRepo struct {
	pool *pgxpool.Pool
}

func NewTenantRepo(pool *pgxpool.Pool) *TenantRepo {
	return &TenantRepo{pool: pool}
}

func (r *TenantRepo) Create(ctx context.Context, tenant *domain.Tenant) error {
	query := `INSERT INTO tenants (id, name, plan, settings, created_at, updated_at)
		VALUES ($1, $2, $3, $4, NOW(), NOW())
		RETURNING created_at, updated_at`

	if tenant.ID == uuid.Nil {
		tenant.ID = uuid.New()
	}

	if tenant.Settings == nil {
		tenant.Settings = domain.JSONMap{}
	}

	return r.pool.QueryRow(ctx, query,
		tenant.ID, tenant.Name, tenant.Plan, tenant.Settings,
	).Scan(&tenant.CreatedAt, &tenant.UpdatedAt)
}

func (r *TenantRepo) GetByID(ctx context.Context, id uuid.UUID) (*domain.Tenant, error) {
	query := `SELECT id, name, plan, settings, created_at, updated_at
		FROM tenants WHERE id = $1`

	var t domain.Tenant
	err := r.pool.QueryRow(ctx, query, id).Scan(
		&t.ID, &t.Name, &t.Plan, &t.Settings, &t.CreatedAt, &t.UpdatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("get tenant: %w", err)
	}
	return &t, nil
}