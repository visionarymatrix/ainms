package clickhouse

import (
	"context"
	"fmt"
	"time"

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

	// Summaries handled by UpsertAppUsage to avoid double-counting with legacy path.

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
		INSERT INTO popup_events (device_id, alert_id, decision, app_name, window_title, explanation, 
			popup_type, classification, confidence, event_time, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())`,
		req.DeviceID, req.AlertID, req.Decision, req.AppName, req.WindowTitle, req.Explanation,
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

	query := `SELECT device_id, app_name, date, total_duration_sec, session_count, 
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
		if err := rows.Scan(&s.DeviceID, &s.AppName, &s.Date, &s.TotalDurationSec, &s.SessionCount,
			&s.ProductiveDuration, &s.UnproductiveDuration, &s.NeutralDuration); err != nil {
			return nil, fmt.Errorf("scan summary: %w", err)
		}
		summaries = append(summaries, s)
	}

	return summaries, rows.Err()
}

func (r *EventRepo) GetAppUsageSummariesByDate(ctx context.Context, deviceID string, date string) ([]domain.AppUsageSummary, error) {
	query := `SELECT device_id, app_name, date, total_duration_sec, session_count,
		productive_duration_sec, unproductive_duration_sec, neutral_duration_sec
		FROM app_usage_summaries WHERE device_id = $1 AND date = $2
		ORDER BY total_duration_sec DESC`

	rows, err := r.pgPool.Query(ctx, query, deviceID, date)
	if err != nil {
		return nil, fmt.Errorf("get summaries by date: %w", err)
	}
	defer rows.Close()

	summaries := make([]domain.AppUsageSummary, 0)
	for rows.Next() {
		var s domain.AppUsageSummary
		if err := rows.Scan(&s.DeviceID, &s.AppName, &s.Date, &s.TotalDurationSec, &s.SessionCount,
			&s.ProductiveDuration, &s.UnproductiveDuration, &s.NeutralDuration); err != nil {
			return nil, fmt.Errorf("scan summary by date: %w", err)
		}
		summaries = append(summaries, s)
	}

	return summaries, rows.Err()
}

func (r *EventRepo) GetEventsByDevice(ctx context.Context, deviceID string, limit int, fromTime *time.Time, toTime *time.Time) ([]domain.AppUsageEventMeta, error) {
	query := `SELECT device_id, app_name, window_title, process_name, process_id, 
		start_time, end_time, duration_sec, classification, confidence
		FROM app_usage_events WHERE device_id = $1`

	args := []interface{}{deviceID}
	argIdx := 2

	if fromTime != nil {
		query += fmt.Sprintf(" AND start_time >= $%d", argIdx)
		args = append(args, *fromTime)
		argIdx++
	}
	if toTime != nil {
		query += fmt.Sprintf(" AND start_time <= $%d", argIdx)
		args = append(args, *toTime)
		argIdx++
	}

	query += fmt.Sprintf(" ORDER BY start_time DESC LIMIT $%d", argIdx)
	args = append(args, limit)

	rows, err := r.pgPool.Query(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("get events: %w", err)
	}
	defer rows.Close()

	events := make([]domain.AppUsageEventMeta, 0)
	for rows.Next() {
		var e domain.AppUsageEventMeta
		if err := rows.Scan(&e.DeviceID, &e.AppName, &e.WindowTitle, &e.ProcessName,
			&e.ProcessID, &e.StartTime, &e.EndTime, &e.DurationSec, &e.Classification, &e.Confidence); err != nil {
			return nil, fmt.Errorf("scan event: %w", err)
		}
		events = append(events, e)
	}
	return events, rows.Err()
}

func (r *EventRepo) updateHeartbeat(ctx context.Context, deviceID uuid.UUID) error {
	_, err := r.pgPool.Exec(ctx, `UPDATE devices SET last_heartbeat = NOW(), updated_at = NOW() WHERE id = $1`, deviceID)
	return err
}

func (r *EventRepo) StoreActivitySummaries(ctx context.Context, req domain.BulkActivitySummaryRequest) error {
	deviceUUID, err := uuid.Parse(req.DeviceID)
	if err != nil {
		return fmt.Errorf("invalid device_id: %w", err)
	}

	for _, s := range req.Summaries {
		_, err := r.pgPool.Exec(ctx, `
			INSERT INTO activity_summaries (device_id, window_start, window_end, summary_text, top_apps, screenshot_count, created_at)
			VALUES ($1, $2, $3, $4, $5, $6, NOW())`,
			deviceUUID, s.WindowStart, s.WindowEnd, s.SummaryText, s.TopApps, s.ScreenshotCount,
		)
		if err != nil {
			return fmt.Errorf("store activity summary: %w", err)
		}
	}

	_ = r.updateHeartbeat(ctx, deviceUUID)
	return nil
}

func (r *EventRepo) GetActivitySummaries(ctx context.Context, deviceIDs []string, limit int, fromTime *time.Time, toTime *time.Time) ([]domain.ActivitySummary, error) {
	if len(deviceIDs) == 0 {
		return []domain.ActivitySummary{}, nil
	}

	query := `SELECT id, device_id, window_start, window_end, summary_text, top_apps, screenshot_count, created_at
		FROM activity_summaries WHERE device_id = ANY($1)`

	args := []interface{}{deviceIDs}
	argIdx := 2

	if fromTime != nil {
		query += fmt.Sprintf(" AND window_start >= $%d", argIdx)
		args = append(args, *fromTime)
		argIdx++
	}
	if toTime != nil {
		query += fmt.Sprintf(" AND window_start <= $%d", argIdx)
		args = append(args, *toTime)
		argIdx++
	}

	query += fmt.Sprintf(" ORDER BY window_start DESC LIMIT $%d", argIdx)
	args = append(args, limit)

	rows, err := r.pgPool.Query(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("get activity summaries: %w", err)
	}
	defer rows.Close()

	summaries := make([]domain.ActivitySummary, 0)
	for rows.Next() {
		var s domain.ActivitySummary
		if err := rows.Scan(&s.ID, &s.DeviceID, &s.WindowStart, &s.WindowEnd,
			&s.SummaryText, &s.TopApps, &s.ScreenshotCount, &s.CreatedAt); err != nil {
			return nil, fmt.Errorf("scan activity summary: %w", err)
		}
		summaries = append(summaries, s)
	}

	return summaries, rows.Err()
}

func (r *EventRepo) UpsertAppUsage(ctx context.Context, req domain.AppUsageUpdateRequest) error {
	deviceUUID, err := uuid.Parse(req.DeviceID)
	if err != nil {
		return fmt.Errorf("invalid device_id: %w", err)
	}

	for _, app := range req.Apps {
		var productiveDur, unproductiveDur, neutralDur float64
		switch app.Classification {
		case "productive":
			productiveDur = app.DurationSec
		case "unproductive":
			unproductiveDur = app.DurationSec
		case "neutral":
			neutralDur = app.DurationSec
		}

		_, err := r.pgPool.Exec(ctx, `
			INSERT INTO app_usage_summaries (device_id, app_name, date, total_duration_sec, session_count,
				productive_duration_sec, unproductive_duration_sec, neutral_duration_sec, updated_at)
			VALUES ($1, $2, CURRENT_DATE, $3, $4, $5, $6, $7, NOW())
			ON CONFLICT (device_id, app_name, date) DO UPDATE SET
				total_duration_sec = app_usage_summaries.total_duration_sec + EXCLUDED.total_duration_sec,
				session_count = app_usage_summaries.session_count + EXCLUDED.session_count,
				productive_duration_sec = app_usage_summaries.productive_duration_sec + EXCLUDED.productive_duration_sec,
				unproductive_duration_sec = app_usage_summaries.unproductive_duration_sec + EXCLUDED.unproductive_duration_sec,
				neutral_duration_sec = app_usage_summaries.neutral_duration_sec + EXCLUDED.neutral_duration_sec,
				updated_at = NOW()`,
			deviceUUID, app.AppName, app.DurationSec, app.OpenCount,
			productiveDur, unproductiveDur, neutralDur,
		)
		if err != nil {
			return fmt.Errorf("upsert app usage for %s: %w", app.AppName, err)
		}
	}

	_ = r.updateHeartbeat(ctx, deviceUUID)
	return nil
}
