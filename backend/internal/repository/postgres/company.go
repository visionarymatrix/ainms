package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type CompanyRepo struct {
	pool *pgxpool.Pool
}

func NewCompanyRepo(pool *pgxpool.Pool) *CompanyRepo {
	return &CompanyRepo{pool: pool}
}

func (r *CompanyRepo) Create(ctx context.Context, company *domain.Company) error {
	query := `INSERT INTO companies (id, tenant_id, name, plan, settings, created_at, updated_at)
		VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
		RETURNING created_at, updated_at`

	if company.ID == uuid.Nil {
		company.ID = uuid.New()
	}

	return r.pool.QueryRow(ctx, query,
		company.ID, company.TenantID, company.Name, company.Plan, company.Settings,
	).Scan(&company.CreatedAt, &company.UpdatedAt)
}

func (r *CompanyRepo) GetByID(ctx context.Context, id uuid.UUID) (*domain.Company, error) {
	query := `SELECT id, tenant_id, name, plan, settings, created_at, updated_at
		FROM companies WHERE id = $1`

	var c domain.Company
	err := r.pool.QueryRow(ctx, query, id).Scan(
		&c.ID, &c.TenantID, &c.Name, &c.Plan, &c.Settings, &c.CreatedAt, &c.UpdatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("get company: %w", err)
	}
	return &c, nil
}

func (r *CompanyRepo) List(ctx context.Context, tenantID uuid.UUID) ([]domain.Company, error) {
	query := `SELECT id, tenant_id, name, plan, settings, created_at, updated_at
		FROM companies WHERE tenant_id = $1 ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query, tenantID)
	if err != nil {
		return nil, fmt.Errorf("list companies: %w", err)
	}
	defer rows.Close()

	companies := make([]domain.Company, 0)
	for rows.Next() {
		var c domain.Company
		if err := rows.Scan(&c.ID, &c.TenantID, &c.Name, &c.Plan, &c.Settings, &c.CreatedAt, &c.UpdatedAt); err != nil {
			return nil, fmt.Errorf("scan company: %w", err)
		}
		companies = append(companies, c)
	}
	return companies, rows.Err()
}

func (r *CompanyRepo) ListAll(ctx context.Context) ([]domain.Company, error) {
	query := `SELECT id, tenant_id, name, plan, settings, created_at, updated_at
		FROM companies ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query)
	if err != nil {
		return nil, fmt.Errorf("list all companies: %w", err)
	}
	defer rows.Close()

	companies := make([]domain.Company, 0)
	for rows.Next() {
		var c domain.Company
		if err := rows.Scan(&c.ID, &c.TenantID, &c.Name, &c.Plan, &c.Settings, &c.CreatedAt, &c.UpdatedAt); err != nil {
			return nil, fmt.Errorf("scan company: %w", err)
		}
		companies = append(companies, c)
	}
	return companies, rows.Err()
}

func (r *CompanyRepo) Update(ctx context.Context, company *domain.Company) error {
	query := `UPDATE companies SET name = $2, plan = $3, settings = $4, updated_at = NOW()
		WHERE id = $1
		RETURNING updated_at`

	return r.pool.QueryRow(ctx, query,
		company.ID, company.Name, company.Plan, company.Settings,
	).Scan(&company.UpdatedAt)
}