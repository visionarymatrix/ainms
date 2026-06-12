package postgres

import (
	"context"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

// ComplianceAlertRepo manages compliance_alerts in Postgres.
type ComplianceAlertRepo struct {
	pool *pgxpool.Pool
}

// NewComplianceAlertRepo creates a new repository.
func NewComplianceAlertRepo(pool *pgxpool.Pool) *ComplianceAlertRepo {
	return &ComplianceAlertRepo{pool: pool}
}

// Create inserts a new compliance alert.
func (r *ComplianceAlertRepo) Create(ctx context.Context, alert *domain.ComplianceAlert) error {
	query := `
		INSERT INTO compliance_alerts (id, device_id, employee_id, screenshot_id, decision, message, model_used, raw_response, status, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
		RETURNING created_at`
	return r.pool.QueryRow(ctx, query,
		alert.ID,
		alert.DeviceID,
		alert.EmployeeID,
		alert.ScreenshotID,
		alert.Decision,
		alert.Message,
		alert.ModelUsed,
		alert.RawResponse,
		alert.Status,
		alert.CreatedAt,
	).Scan(&alert.CreatedAt)
}

// ListPendingByDevice returns all pending alerts for a device, ordered by creation time.
func (r *ComplianceAlertRepo) ListPendingByDevice(ctx context.Context, deviceID uuid.UUID) ([]domain.ComplianceAlert, error) {
	query := `
		SELECT id, device_id, employee_id, screenshot_id, decision, message, model_used, raw_response, status, created_at
		FROM compliance_alerts
		WHERE device_id = $1 AND status = 'pending'
		ORDER BY created_at ASC`

	rows, err := r.pool.Query(ctx, query, deviceID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var alerts []domain.ComplianceAlert
	for rows.Next() {
		var a domain.ComplianceAlert
		if err := rows.Scan(
			&a.ID, &a.DeviceID, &a.EmployeeID, &a.ScreenshotID,
			&a.Decision, &a.Message, &a.ModelUsed, &a.RawResponse,
			&a.Status, &a.CreatedAt,
		); err != nil {
			return nil, err
		}
		alerts = append(alerts, a)
	}
	return alerts, rows.Err()
}

// MarkDelivered updates status to 'delivered' and sets delivered_at.
func (r *ComplianceAlertRepo) MarkDelivered(ctx context.Context, id uuid.UUID) error {
	query := `UPDATE compliance_alerts SET status = 'delivered', delivered_at = $1 WHERE id = $2`
	_, err := r.pool.Exec(ctx, query, time.Now(), id)
	return err
}

// MarkAcked updates status to 'acked' and sets acked_at.
func (r *ComplianceAlertRepo) MarkAcked(ctx context.Context, id uuid.UUID) error {
	query := `UPDATE compliance_alerts SET status = 'acked', acked_at = $1 WHERE id = $2`
	_, err := r.pool.Exec(ctx, query, time.Now(), id)
	return err
}

// ListByCompany returns all alerts for a company's employees, with optional status filter.
func (r *ComplianceAlertRepo) ListByCompany(ctx context.Context, companyID uuid.UUID, status string) ([]domain.ComplianceAlert, error) {
	query := `
		SELECT ca.id, ca.device_id, ca.employee_id, ca.screenshot_id, ca.decision, ca.message, ca.model_used, ca.raw_response, ca.status, ca.created_at, ca.delivered_at, ca.acked_at
		FROM compliance_alerts ca
		JOIN employees e ON ca.employee_id = e.id
		WHERE e.company_id = $1 AND ($2 = '' OR ca.status = $2)
		ORDER BY ca.created_at DESC`

	rows, err := r.pool.Query(ctx, query, companyID, status)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var alerts []domain.ComplianceAlert
	for rows.Next() {
		var a domain.ComplianceAlert
		if err := rows.Scan(
			&a.ID, &a.DeviceID, &a.EmployeeID, &a.ScreenshotID,
			&a.Decision, &a.Message, &a.ModelUsed, &a.RawResponse,
			&a.Status, &a.CreatedAt, &a.DeliveredAt, &a.AckedAt,
		); err != nil {
			return nil, err
		}
		alerts = append(alerts, a)
	}
	return alerts, rows.Err()
}

// ListByEmployee returns all alerts for a specific employee, with optional status filter.
func (r *ComplianceAlertRepo) ListByEmployee(ctx context.Context, employeeID uuid.UUID, status string) ([]domain.ComplianceAlert, error) {
	query := `
		SELECT id, device_id, employee_id, screenshot_id, decision, message, model_used, raw_response, status, created_at, delivered_at, acked_at
		FROM compliance_alerts
		WHERE employee_id = $1 AND ($2 = '' OR status = $2)
		ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query, employeeID, status)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var alerts []domain.ComplianceAlert
	for rows.Next() {
		var a domain.ComplianceAlert
		if err := rows.Scan(
			&a.ID, &a.DeviceID, &a.EmployeeID, &a.ScreenshotID,
			&a.Decision, &a.Message, &a.ModelUsed, &a.RawResponse,
			&a.Status, &a.CreatedAt, &a.DeliveredAt, &a.AckedAt,
		); err != nil {
			return nil, err
		}
		alerts = append(alerts, a)
	}
	return alerts, rows.Err()
}

// GetByID returns a single alert by ID.
func (r *ComplianceAlertRepo) GetByID(ctx context.Context, id uuid.UUID) (*domain.ComplianceAlert, error) {
	query := `
		SELECT id, device_id, employee_id, screenshot_id, decision, message, model_used, raw_response, status, created_at, delivered_at, acked_at
		FROM compliance_alerts
		WHERE id = $1`

	var a domain.ComplianceAlert
	err := r.pool.QueryRow(ctx, query, id).Scan(
		&a.ID, &a.DeviceID, &a.EmployeeID, &a.ScreenshotID,
		&a.Decision, &a.Message, &a.ModelUsed, &a.RawResponse,
		&a.Status, &a.CreatedAt, &a.DeliveredAt, &a.AckedAt,
	)
	if err != nil {
		return nil, err
	}
	return &a, nil
}

// GetPopupAnswersByAlertIDs returns popup events for the given alert IDs.
func (r *ComplianceAlertRepo) GetPopupAnswersByAlertIDs(ctx context.Context, alertIDs []uuid.UUID) (map[uuid.UUID]domain.PopupEventDB, error) {
	if len(alertIDs) == 0 {
		return map[uuid.UUID]domain.PopupEventDB{}, nil
	}

	query := `
		SELECT id, device_id, alert_id, decision, app_name, window_title, explanation, popup_type, classification, confidence, event_time, created_at
		FROM popup_events
		WHERE alert_id = ANY($1)`

	rows, err := r.pool.Query(ctx, query, alertIDs)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	result := make(map[uuid.UUID]domain.PopupEventDB)
	for rows.Next() {
		var pe domain.PopupEventDB
		if err := rows.Scan(
			&pe.ID, &pe.DeviceID, &pe.AlertID, &pe.Decision,
			&pe.AppName, &pe.WindowTitle, &pe.Explanation,
			&pe.PopupType, &pe.Classification, &pe.Confidence,
			&pe.EventTime, &pe.CreatedAt,
		); err != nil {
			return nil, err
		}
		if pe.AlertID != nil {
			result[*pe.AlertID] = pe
		}
	}
	return result, rows.Err()
}
