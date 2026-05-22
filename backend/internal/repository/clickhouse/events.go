package clickhouse

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type EventRepo struct {
	pgPool *pgxpool.Pool
}

func NewEventRepo(pgPool *pgxpool.Pool) *EventRepo {
	return &EventRepo{pgPool: pgPool}
}

func (r *EventRepo) StoreBulkEvents(ctx context.Context, deviceID string, summary domain.AppUsageSummary, metadata []domain.AppUsageEventMeta) error {
	for _, meta := range metadata {
		_, err := r.pgPool.Exec(ctx, `
			INSERT INTO app_usage_events (device_id, app_name, window_title, process_name, process_id, 
				start_time, end_time, duration_sec, classification, confidence, created_at)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())`,
			meta.DeviceID, meta.AppName, meta.WindowTitle, meta.ProcessName, meta.ProcessID,
			meta.StartTime, meta.EndTime, meta.DurationSec, meta.Classification, meta.Confidence,
		)
		if err != nil {
			return fmt.Errorf("store event: %w", err)
		}
	}

	_, err := r.pgPool.Exec(ctx, `
		INSERT INTO app_usage_summaries (device_id, app_name, total_duration_sec, session_count, 
			productive_duration_sec, unproductive_duration_sec, neutral_duration_sec, updated_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
		ON CONFLICT (device_id, app_name) DO UPDATE SET
			total_duration_sec = EXCLUDED.total_duration_sec,
			session_count = app_usage_summaries.session_count + EXCLUDED.session_count,
			productive_duration_sec = EXCLUDED.productive_duration_sec,
			unproductive_duration_sec = EXCLUDED.unproductive_duration_sec,
			neutral_duration_sec = EXCLUDED.neutral_duration_sec,
			updated_at = NOW()`,
		summary.DeviceID, summary.AppName, summary.TotalDurationSec, summary.SessionCount,
		summary.ProductiveDuration, summary.UnproductiveDuration, summary.NeutralDuration,
	)
	if err != nil {
		return fmt.Errorf("store summary: %w", err)
	}

	if deviceUUID, err := uuid.Parse(deviceID); err == nil {
		_ = r.updateHeartbeat(ctx, deviceUUID)
	}

	return nil
}

func (r *EventRepo) StorePriorityEvent(ctx context.Context, req domain.PriorityEventRequest) error {
	_, err := r.pgPool.Exec(ctx, `
		INSERT INTO priority_events (device_id, event_type, app_name, window_title, 
			classification, confidence, event_time, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())`,
		req.DeviceID, req.EventType, req.AppName, req.WindowTitle,
		req.Classification, req.Confidence, req.Timestamp,
	)
	if err != nil {
		return fmt.Errorf("store priority event: %w", err)
	}
	return nil
}

func (r *EventRepo) StorePopupEvent(ctx context.Context, req domain.PopupEvent) error {
	_, err := r.pgPool.Exec(ctx, `
		INSERT INTO popup_events (device_id, app_name, window_title, explanation, 
			popup_type, classification, confidence, event_time, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())`,
		req.DeviceID, req.AppName, req.WindowTitle, req.Explanation,
		req.PopupType, req.Classification, req.Confidence, req.Timestamp,
	)
	if err != nil {
		return fmt.Errorf("store popup event: %w", err)
	}
	return nil
}

func (r *EventRepo) GetAppUsageSummaries(ctx context.Context, deviceIDs []string) ([]domain.AppUsageSummary, error) {
	if len(deviceIDs) == 0 {
		return []domain.AppUsageSummary{}, nil
	}

	query := `SELECT device_id, app_name, total_duration_sec, session_count, 
		productive_duration_sec, unproductive_duration_sec, neutral_duration_sec
		FROM app_usage_summaries WHERE device_id = ANY($1)
		ORDER BY total_duration_sec DESC`

	rows, err := r.pgPool.Query(ctx, query, deviceIDs)
	if err != nil {
		return nil, fmt.Errorf("get summaries: %w", err)
	}
	defer rows.Close()

	summaries := make([]domain.AppUsageSummary, 0)
	for rows.Next() {
		var s domain.AppUsageSummary
		if err := rows.Scan(&s.DeviceID, &s.AppName, &s.TotalDurationSec, &s.SessionCount,
			&s.ProductiveDuration, &s.UnproductiveDuration, &s.NeutralDuration); err != nil {
			return nil, fmt.Errorf("scan summary: %w", err)
		}
		summaries = append(summaries, s)
	}

	return summaries, rows.Err()
}

func (r *EventRepo) updateHeartbeat(ctx context.Context, deviceID uuid.UUID) error {
	_, err := r.pgPool.Exec(ctx, `UPDATE devices SET last_heartbeat = NOW(), updated_at = NOW() WHERE id = $1`, deviceID)
	return err
}