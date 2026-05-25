package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type InstallTokenRepo struct {
	pool *pgxpool.Pool
}

func NewInstallTokenRepo(pool *pgxpool.Pool) *InstallTokenRepo {
	return &InstallTokenRepo{pool: pool}
}

const installTokenColumns = `id, token, employee_id, company_id, description, expires_at, created_by, created_at, revoked_at`

func scanInstallToken(scanner interface{ Scan(...interface{}) error }, t *domain.InstallToken) error {
	return scanner.Scan(
		&t.ID, &t.Token, &t.EmployeeID, &t.CompanyID, &t.Description,
		&t.ExpiresAt, &t.CreatedBy, &t.CreatedAt, &t.RevokedAt,
	)
}

func (r *InstallTokenRepo) Create(ctx context.Context, token *domain.InstallToken) error {
	query := `INSERT INTO install_tokens (id, token, employee_id, company_id, description, expires_at, created_by, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
		RETURNING created_at`

	if token.ID == uuid.Nil {
		token.ID = uuid.New()
	}

	return r.pool.QueryRow(ctx, query,
		token.ID, token.Token, token.EmployeeID, token.CompanyID,
		token.Description, token.ExpiresAt, token.CreatedBy,
	).Scan(&token.CreatedAt)
}

func (r *InstallTokenRepo) GetByToken(ctx context.Context, tokenString string) (*domain.InstallToken, error) {
	query := `SELECT ` + installTokenColumns + ` FROM install_tokens WHERE token = $1`

	var t domain.InstallToken
	if err := scanInstallToken(r.pool.QueryRow(ctx, query, tokenString), &t); err != nil {
		return nil, fmt.Errorf("get install token: %w", err)
	}
	return &t, nil
}

func (r *InstallTokenRepo) GetByTokenWithCompany(ctx context.Context, tokenString string) (*domain.InstallToken, error) {
	query := `SELECT it.id, it.token, it.employee_id, e.company_id, it.description, it.expires_at, it.created_by, it.created_at, it.revoked_at
		FROM install_tokens it
		JOIN employees e ON it.employee_id = e.id
		WHERE it.token = $1`

	var t domain.InstallToken
	err := r.pool.QueryRow(ctx, query, tokenString).Scan(
		&t.ID, &t.Token, &t.EmployeeID, &t.CompanyID, &t.Description,
		&t.ExpiresAt, &t.CreatedBy, &t.CreatedAt, &t.RevokedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("get install token with company: %w", err)
	}
	return &t, nil
}

func (r *InstallTokenRepo) ListByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.InstallToken, error) {
	query := `SELECT ` + installTokenColumns + ` FROM install_tokens WHERE company_id = $1 ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query, companyID)
	if err != nil {
		return nil, fmt.Errorf("list install tokens: %w", err)
	}
	defer rows.Close()

	tokens := make([]domain.InstallToken, 0)
	for rows.Next() {
		var t domain.InstallToken
		if err := scanInstallToken(rows, &t); err != nil {
			return nil, fmt.Errorf("scan install token: %w", err)
		}
		tokens = append(tokens, t)
	}
	return tokens, rows.Err()
}

func (r *InstallTokenRepo) GetActiveByEmployee(ctx context.Context, employeeID uuid.UUID) (*domain.InstallToken, error) {
	query := `SELECT ` + installTokenColumns + ` FROM install_tokens
		WHERE employee_id = $1 AND revoked_at IS NULL
		ORDER BY created_at DESC LIMIT 1`

	var t domain.InstallToken
	if err := scanInstallToken(r.pool.QueryRow(ctx, query, employeeID), &t); err != nil {
		return nil, fmt.Errorf("get active install token by employee: %w", err)
	}
	return &t, nil
}

func (r *InstallTokenRepo) Revoke(ctx context.Context, id uuid.UUID) error {
	query := `UPDATE install_tokens SET revoked_at = NOW() WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("revoke install token: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("install token not found: %s", id)
	}
	return nil
}

func (r *InstallTokenRepo) Delete(ctx context.Context, id uuid.UUID) error {
	query := `DELETE FROM install_tokens WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("delete install token: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("install token not found: %s", id)
	}
	return nil
}