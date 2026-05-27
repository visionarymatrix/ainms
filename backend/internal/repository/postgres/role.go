package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
	"github.com/jackc/pgx/v5/pgxpool"
)

type RoleRepo struct {
	pool *pgxpool.Pool
}

func NewRoleRepo(pool *pgxpool.Pool) *RoleRepo {
	return &RoleRepo{pool: pool}
}

const roleColumns = `id, company_id, name, description, work_description, allowed_categories, blocked_categories, created_at, updated_at`

func scanRole(scanner interface {
	Scan(dest ...interface{}) error
}, r *domain.Role) error {
	var allowedPg pgtype.Array[string]
	var blockedPg pgtype.Array[string]

	if err := scanner.Scan(
		&r.ID, &r.CompanyID, &r.Name, &r.Description, &r.WorkDescription,
		&allowedPg, &blockedPg, &r.CreatedAt, &r.UpdatedAt,
	); err != nil {
		return err
	}

	r.AllowedCategories = allowedPg.Elements
	r.BlockedCategories = blockedPg.Elements
	return nil
}

func (r *RoleRepo) Create(ctx context.Context, role *domain.Role) error {
	query := `INSERT INTO roles (id, company_id, name, description, work_description, allowed_categories, blocked_categories, created_at, updated_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
		RETURNING created_at, updated_at`

	if role.ID == uuid.Nil {
		role.ID = uuid.New()
	}

	allowedPg := pgtype.Array[string]{Elements: role.AllowedCategories, Dims: []pgtype.ArrayDimension{{Length: int32(len(role.AllowedCategories)), LowerBound: 1}}, Valid: true}
	if len(role.AllowedCategories) == 0 {
		allowedPg.Valid = true
		allowedPg.Dims = []pgtype.ArrayDimension{{Length: 0, LowerBound: 1}}
	}
	blockedPg := pgtype.Array[string]{Elements: role.BlockedCategories, Dims: []pgtype.ArrayDimension{{Length: int32(len(role.BlockedCategories)), LowerBound: 1}}, Valid: true}
	if len(role.BlockedCategories) == 0 {
		blockedPg.Valid = true
		blockedPg.Dims = []pgtype.ArrayDimension{{Length: 0, LowerBound: 1}}
	}

	return r.pool.QueryRow(ctx, query,
		role.ID, role.CompanyID, role.Name, role.Description, role.WorkDescription,
		allowedPg, blockedPg,
	).Scan(&role.CreatedAt, &role.UpdatedAt)
}

func (r *RoleRepo) GetByID(ctx context.Context, id uuid.UUID) (*domain.Role, error) {
	query := `SELECT ` + roleColumns + ` FROM roles WHERE id = $1`

	var role domain.Role
	if err := scanRole(r.pool.QueryRow(ctx, query, id), &role); err != nil {
		return nil, fmt.Errorf("get role: %w", err)
	}
	return &role, nil
}

func (r *RoleRepo) List(ctx context.Context, companyID uuid.UUID) ([]domain.Role, error) {
	query := `SELECT ` + roleColumns + ` FROM roles WHERE company_id = $1 ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query, companyID)
	if err != nil {
		return nil, fmt.Errorf("list roles: %w", err)
	}
	defer rows.Close()

	roles := make([]domain.Role, 0)
	for rows.Next() {
		var role domain.Role
		if err := scanRole(rows, &role); err != nil {
			return nil, fmt.Errorf("scan role: %w", err)
		}
		roles = append(roles, role)
	}
	return roles, rows.Err()
}

func (r *RoleRepo) Update(ctx context.Context, role *domain.Role) error {
	query := `UPDATE roles SET name = $2, description = $3, work_description = $4, allowed_categories = $5, blocked_categories = $6, updated_at = NOW()
		WHERE id = $1
		RETURNING updated_at`

	allowedPg := pgtype.Array[string]{Elements: role.AllowedCategories, Dims: []pgtype.ArrayDimension{{Length: int32(len(role.AllowedCategories)), LowerBound: 1}}, Valid: true}
	if len(role.AllowedCategories) == 0 {
		allowedPg.Valid = true
		allowedPg.Dims = []pgtype.ArrayDimension{{Length: 0, LowerBound: 1}}
	}
	blockedPg := pgtype.Array[string]{Elements: role.BlockedCategories, Dims: []pgtype.ArrayDimension{{Length: int32(len(role.BlockedCategories)), LowerBound: 1}}, Valid: true}
	if len(role.BlockedCategories) == 0 {
		blockedPg.Valid = true
		blockedPg.Dims = []pgtype.ArrayDimension{{Length: 0, LowerBound: 1}}
	}

	return r.pool.QueryRow(ctx, query,
		role.ID, role.Name, role.Description, role.WorkDescription,
		allowedPg, blockedPg,
	).Scan(&role.UpdatedAt)
}

func (r *RoleRepo) Delete(ctx context.Context, id uuid.UUID) error {
	query := `DELETE FROM roles WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("delete role: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("role not found: %s", id)
	}
	return nil
}