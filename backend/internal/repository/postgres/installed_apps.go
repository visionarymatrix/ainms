package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type InstalledAppRepo struct {
	pool *pgxpool.Pool
}

func NewInstalledAppRepo(pool *pgxpool.Pool) *InstalledAppRepo {
	return &InstalledAppRepo{pool: pool}
}

func (r *InstalledAppRepo) UpsertInstalledApps(ctx context.Context, deviceID uuid.UUID, apps []domain.InstalledAppEntry) error {
	for _, app := range apps {
		_, err := r.pool.Exec(ctx, `
			INSERT INTO installed_apps (device_id, app_name, display_name, publisher, install_path, category, confidence, source, created_at, updated_at)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
			ON CONFLICT (device_id, app_name) DO UPDATE SET
				display_name = EXCLUDED.display_name,
				publisher = EXCLUDED.publisher,
				install_path = EXCLUDED.install_path,
				category = EXCLUDED.category,
				confidence = EXCLUDED.confidence,
				source = EXCLUDED.source,
				updated_at = NOW()`,
			deviceID, app.AppName, app.DisplayName, app.Publisher, app.InstallPath, app.Category, app.Confidence, app.Source,
		)
		if err != nil {
			return fmt.Errorf("upsert installed app %s: %w", app.AppName, err)
		}
	}
	return nil
}

func (r *InstalledAppRepo) GetByDeviceID(ctx context.Context, deviceID uuid.UUID) ([]domain.InstalledApp, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT id, device_id, app_name, display_name, publisher, install_path, category, confidence, source, created_at, updated_at
		FROM installed_apps WHERE device_id = $1
		ORDER BY display_name`,
		deviceID,
	)
	if err != nil {
		return nil, fmt.Errorf("get installed apps: %w", err)
	}
	defer rows.Close()

	apps := make([]domain.InstalledApp, 0)
	for rows.Next() {
		var a domain.InstalledApp
		if err := rows.Scan(&a.ID, &a.DeviceID, &a.AppName, &a.DisplayName, &a.Publisher, &a.InstallPath, &a.Category, &a.Confidence, &a.Source, &a.CreatedAt, &a.UpdatedAt); err != nil {
			return nil, fmt.Errorf("scan installed app: %w", err)
		}
		apps = append(apps, a)
	}
	return apps, rows.Err()
}

func (r *InstalledAppRepo) GetByEmployeeID(ctx context.Context, employeeID uuid.UUID) ([]domain.InstalledAppWithDevice, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT ia.id, ia.device_id, ia.app_name, ia.display_name, ia.publisher, ia.install_path,
			ia.category, ia.confidence, ia.source, ia.created_at, ia.updated_at,
			ia.device_id, d.hostname, e.employee_id, CONCAT(e.first_name, ' ', e.last_name)
		FROM installed_apps ia
		JOIN devices d ON d.id = ia.device_id
		JOIN employees e ON e.id = d.employee_id
		WHERE d.employee_id = $1
		ORDER BY ia.display_name`,
		employeeID,
	)
	if err != nil {
		return nil, fmt.Errorf("get installed apps by employee: %w", err)
	}
	defer rows.Close()

	apps := make([]domain.InstalledAppWithDevice, 0)
	for rows.Next() {
		var a domain.InstalledAppWithDevice
		if err := rows.Scan(
			&a.ID, &a.DeviceID, &a.AppName, &a.DisplayName, &a.Publisher, &a.InstallPath,
			&a.Category, &a.Confidence, &a.Source, &a.CreatedAt, &a.UpdatedAt,
			&a.DeviceIDStr, &a.DeviceHostname, &a.EmployeeID, &a.EmployeeName,
		); err != nil {
			return nil, fmt.Errorf("scan installed app with device: %w", err)
		}
		apps = append(apps, a)
	}
	return apps, rows.Err()
}

func (r *InstalledAppRepo) DeleteByDeviceID(ctx context.Context, deviceID uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `DELETE FROM installed_apps WHERE device_id = $1`, deviceID)
	if err != nil {
		return fmt.Errorf("delete installed apps: %w", err)
	}
	return nil
}