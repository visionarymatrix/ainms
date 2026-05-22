package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type UserRepo struct {
	pool *pgxpool.Pool
}

func NewUserRepo(pool *pgxpool.Pool) *UserRepo {
	return &UserRepo{pool: pool}
}

func (r *UserRepo) Create(ctx context.Context, user *domain.User) error {
	query := `INSERT INTO users (id, email, password_hash, name, role, company_id, created_at, updated_at)
		VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
		RETURNING created_at, updated_at`

	if user.ID == "" {
		user.ID = uuid.New().String()
	}

	var companyID interface{}
	if user.CompanyID != nil {
		parsed, err := uuid.Parse(*user.CompanyID)
		if err != nil {
			return fmt.Errorf("invalid company_id: %w", err)
		}
		companyID = parsed
	}

	return r.pool.QueryRow(ctx, query,
		user.ID, user.Email, user.PasswordHash, user.Name, user.Role, companyID,
	).Scan(&user.CreatedAt, &user.UpdatedAt)
}

func (r *UserRepo) GetByEmail(ctx context.Context, email string) (*domain.User, error) {
	query := `SELECT id, email, password_hash, name, role, company_id, created_at, updated_at
		FROM users WHERE email = $1`

	var u domain.User
	var companyID *uuid.UUID

	err := r.pool.QueryRow(ctx, query, email).Scan(
		&u.ID, &u.Email, &u.PasswordHash, &u.Name, &u.Role, &companyID, &u.CreatedAt, &u.UpdatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("get user by email: %w", err)
	}

	if companyID != nil {
		companyIDStr := companyID.String()
		u.CompanyID = &companyIDStr
	}

	return &u, nil
}

func (r *UserRepo) GetByID(ctx context.Context, id string) (*domain.User, error) {
	query := `SELECT id, email, password_hash, name, role, company_id, created_at, updated_at
		FROM users WHERE id = $1`

	parsedID, err := uuid.Parse(id)
	if err != nil {
		return nil, fmt.Errorf("invalid user id: %w", err)
	}

	var u domain.User
	var companyID *uuid.UUID

	err = r.pool.QueryRow(ctx, query, parsedID).Scan(
		&u.ID, &u.Email, &u.PasswordHash, &u.Name, &u.Role, &companyID, &u.CreatedAt, &u.UpdatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("get user by id: %w", err)
	}

	if companyID != nil {
		companyIDStr := companyID.String()
		u.CompanyID = &companyIDStr
	}

	return &u, nil
}

func (r *UserRepo) ListByCompany(ctx context.Context, companyID string) ([]domain.User, error) {
	parsedCompanyID, err := uuid.Parse(companyID)
	if err != nil {
		return nil, fmt.Errorf("invalid company_id: %w", err)
	}

	query := `SELECT id, email, password_hash, name, role, company_id, created_at, updated_at
		FROM users WHERE company_id = $1 ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query, parsedCompanyID)
	if err != nil {
		return nil, fmt.Errorf("list users by company: %w", err)
	}
	defer rows.Close()

	users := make([]domain.User, 0)
	for rows.Next() {
		var u domain.User
		var cid *uuid.UUID
		if err := rows.Scan(&u.ID, &u.Email, &u.PasswordHash, &u.Name, &u.Role, &cid, &u.CreatedAt, &u.UpdatedAt); err != nil {
			return nil, fmt.Errorf("scan user: %w", err)
		}
		if cid != nil {
			companyIDStr := cid.String()
			u.CompanyID = &companyIDStr
		}
		users = append(users, u)
	}
	return users, rows.Err()
}