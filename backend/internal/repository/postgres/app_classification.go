package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type AppClassificationRepo struct {
	pool *pgxpool.Pool
}

func NewAppClassificationRepo(pool *pgxpool.Pool) *AppClassificationRepo {
	return &AppClassificationRepo{pool: pool}
}

const appClassificationColumns = `id, role_id, app_name, category, created_at`

func scanAppClassification(scanner interface {
	Scan(dest ...interface{}) error
}, ac *domain.AppClassification) error {
	return scanner.Scan(&ac.ID, &ac.RoleID, &ac.AppName, &ac.Category, &ac.CreatedAt)
}

func (r *AppClassificationRepo) ListByRole(ctx context.Context, roleID uuid.UUID) ([]domain.AppClassification, error) {
	query := `SELECT ` + appClassificationColumns + ` FROM app_classifications WHERE role_id = $1 ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query, roleID)
	if err != nil {
		return nil, fmt.Errorf("list app classifications: %w", err)
	}
	defer rows.Close()

	classifications := make([]domain.AppClassification, 0)
	for rows.Next() {
		var ac domain.AppClassification
		if err := scanAppClassification(rows, &ac); err != nil {
			return nil, fmt.Errorf("scan app classification: %w", err)
		}
		classifications = append(classifications, ac)
	}
	return classifications, rows.Err()
}

func (r *AppClassificationRepo) Create(ctx context.Context, ac *domain.AppClassification) error {
	query := `INSERT INTO app_classifications (id, role_id, app_name, category, created_at)
		VALUES ($1, $2, $3, $4, NOW())
		RETURNING created_at`

	if ac.ID == uuid.Nil {
		ac.ID = uuid.New()
	}

	return r.pool.QueryRow(ctx, query,
		ac.ID, ac.RoleID, ac.AppName, ac.Category,
	).Scan(&ac.CreatedAt)
}

func (r *AppClassificationRepo) Delete(ctx context.Context, id uuid.UUID) error {
	query := `DELETE FROM app_classifications WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("delete app classification: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("app classification not found: %s", id)
	}
	return nil
}