package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type AlertRuleRepo struct {
	pool *pgxpool.Pool
}

func NewAlertRuleRepo(pool *pgxpool.Pool) *AlertRuleRepo {
	return &AlertRuleRepo{pool: pool}
}

const alertRuleColumns = `id, role_id, category, threshold_min, popup_type, created_at`

func scanAlertRule(scanner interface {
	Scan(dest ...interface{}) error
}, ar *domain.AlertRule) error {
	return scanner.Scan(&ar.ID, &ar.RoleID, &ar.Category, &ar.ThresholdMin, &ar.PopupType, &ar.CreatedAt)
}

func (r *AlertRuleRepo) ListByRole(ctx context.Context, roleID uuid.UUID) ([]domain.AlertRule, error) {
	query := `SELECT ` + alertRuleColumns + ` FROM alert_rules WHERE role_id = $1 ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query, roleID)
	if err != nil {
		return nil, fmt.Errorf("list alert rules: %w", err)
	}
	defer rows.Close()

	rules := make([]domain.AlertRule, 0)
	for rows.Next() {
		var ar domain.AlertRule
		if err := scanAlertRule(rows, &ar); err != nil {
			return nil, fmt.Errorf("scan alert rule: %w", err)
		}
		rules = append(rules, ar)
	}
	return rules, rows.Err()
}

func (r *AlertRuleRepo) Create(ctx context.Context, ar *domain.AlertRule) error {
	query := `INSERT INTO alert_rules (id, role_id, category, threshold_min, popup_type, created_at)
		VALUES ($1, $2, $3, $4, $5, NOW())
		RETURNING created_at`

	if ar.ID == uuid.Nil {
		ar.ID = uuid.New()
	}

	return r.pool.QueryRow(ctx, query,
		ar.ID, ar.RoleID, ar.Category, ar.ThresholdMin, ar.PopupType,
	).Scan(&ar.CreatedAt)
}

func (r *AlertRuleRepo) Delete(ctx context.Context, id uuid.UUID) error {
	query := `DELETE FROM alert_rules WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("delete alert rule: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("alert rule not found: %s", id)
	}
	return nil
}