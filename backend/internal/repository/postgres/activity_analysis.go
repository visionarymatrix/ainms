package postgres

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type ActivityAnalysisRepo struct {
	pool *pgxpool.Pool
}

func NewActivityAnalysisRepo(pool *pgxpool.Pool) *ActivityAnalysisRepo {
	return &ActivityAnalysisRepo{pool: pool}
}

func (r *ActivityAnalysisRepo) GetCached(ctx context.Context, employeeID uuid.UUID, startTime, endTime time.Time) (*domain.ActivityAnalysisResult, error) {
	query := `SELECT employee_id, employee_name, project_name, working_apps,
		total_duration_sec, productive_duration_sec, unproductive_duration_sec, neutral_duration_sec,
		productive_pct, unproductive_pct, neutral_pct,
		events, ai_model, analyzed_at, start_time, end_time
		FROM activity_analysis_cache
		WHERE employee_id = $1 AND start_time = $2 AND end_time = $3`

	var result domain.ActivityAnalysisResult
	var workingAppsJSON []byte
	var eventsJSON []byte
	var projectName *string
	var employeeIDStr string

	if err := r.pool.QueryRow(ctx, query, employeeID, startTime, endTime).Scan(
		&employeeIDStr, &result.EmployeeName, &projectName, &workingAppsJSON,
		&result.TotalDurationSec, &result.ProductiveDuration, &result.UnproductiveDuration, &result.NeutralDuration,
		&result.ProductivePct, &result.UnproductivePct, &result.NeutralPct,
		&eventsJSON, &result.AIModel, &result.AnalyzedAt, &result.StartTime, &result.EndTime,
	); err != nil {
		return nil, fmt.Errorf("activity analysis cache lookup: %w", err)
	}

	result.EmployeeID = employeeIDStr
	if projectName != nil {
		result.ProjectName = *projectName
	}
	if len(workingAppsJSON) > 0 {
		_ = json.Unmarshal(workingAppsJSON, &result.WorkingApps)
	}
	if len(eventsJSON) > 0 {
		_ = json.Unmarshal(eventsJSON, &result.Events)
	}

	return &result, nil
}

func (r *ActivityAnalysisRepo) Save(ctx context.Context, result *domain.ActivityAnalysisResult) error {
	workingAppsJSON, err := json.Marshal(result.WorkingApps)
	if err != nil {
		return fmt.Errorf("marshal working_apps: %w", err)
	}
	eventsJSON, err := json.Marshal(result.Events)
	if err != nil {
		return fmt.Errorf("marshal events: %w", err)
	}

	employeeID, err := uuid.Parse(result.EmployeeID)
	if err != nil {
		return fmt.Errorf("parse employee_id: %w", err)
	}

	var projectNamePtr *string
	if result.ProjectName != "" {
		projectNamePtr = &result.ProjectName
	}

	query := `INSERT INTO activity_analysis_cache (
		employee_id, start_time, end_time, employee_name, project_name, working_apps,
		total_duration_sec, productive_duration_sec, unproductive_duration_sec, neutral_duration_sec,
		productive_pct, unproductive_pct, neutral_pct,
		events, ai_model, analyzed_at
	) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
	ON CONFLICT (employee_id, start_time, end_time) DO UPDATE SET
		employee_name = EXCLUDED.employee_name,
		project_name = EXCLUDED.project_name,
		working_apps = EXCLUDED.working_apps,
		total_duration_sec = EXCLUDED.total_duration_sec,
		productive_duration_sec = EXCLUDED.productive_duration_sec,
		unproductive_duration_sec = EXCLUDED.unproductive_duration_sec,
		neutral_duration_sec = EXCLUDED.neutral_duration_sec,
		productive_pct = EXCLUDED.productive_pct,
		unproductive_pct = EXCLUDED.unproductive_pct,
		neutral_pct = EXCLUDED.neutral_pct,
		events = EXCLUDED.events,
		ai_model = EXCLUDED.ai_model,
		analyzed_at = EXCLUDED.analyzed_at`

	_, err = r.pool.Exec(ctx, query,
		employeeID, result.StartTime, result.EndTime, result.EmployeeName, projectNamePtr, workingAppsJSON,
		result.TotalDurationSec, result.ProductiveDuration, result.UnproductiveDuration, result.NeutralDuration,
		result.ProductivePct, result.UnproductivePct, result.NeutralPct,
		eventsJSON, result.AIModel, result.AnalyzedAt,
	)
	if err != nil {
		return fmt.Errorf("save activity analysis cache: %w", err)
	}
	return nil
}