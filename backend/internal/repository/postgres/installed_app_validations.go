package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type InstalledAppValidationRepo struct {
	pool *pgxpool.Pool
}

func NewInstalledAppValidationRepo(pool *pgxpool.Pool) *InstalledAppValidationRepo {
	return &InstalledAppValidationRepo{pool: pool}
}

func (r *InstalledAppValidationRepo) UpsertValidations(ctx context.Context, validations []domain.InstalledAppValidation) error {
	for _, v := range validations {
		_, err := r.pool.Exec(ctx, `
			INSERT INTO installed_app_validations (id, device_id, app_name, role_id, display_name, agent_category, validated_category, is_compliant, reason, validated_at, created_at, updated_at)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
			ON CONFLICT (device_id, app_name) DO UPDATE SET
				role_id = EXCLUDED.role_id,
				display_name = EXCLUDED.display_name,
				agent_category = EXCLUDED.agent_category,
				validated_category = EXCLUDED.validated_category,
				is_compliant = EXCLUDED.is_compliant,
				reason = EXCLUDED.reason,
				validated_at = EXCLUDED.validated_at,
				updated_at = NOW()`,
			v.ID, v.DeviceID, v.AppName, v.RoleID, v.DisplayName, v.AgentCategory, v.ValidatedCategory, v.IsCompliant, v.Reason, v.ValidatedAt,
		)
		if err != nil {
			return fmt.Errorf("upsert installed app validation %s: %w", v.AppName, err)
		}
	}
	return nil
}

const validationColumns = `id, device_id, app_name, role_id, display_name, agent_category, validated_category, is_compliant, reason, validated_at, created_at, updated_at`

func scanInstalledAppValidation(scanner interface {
	Scan(dest ...interface{}) error
}, v *domain.InstalledAppValidation) error {
	return scanner.Scan(&v.ID, &v.DeviceID, &v.AppName, &v.RoleID, &v.DisplayName, &v.AgentCategory, &v.ValidatedCategory, &v.IsCompliant, &v.Reason, &v.ValidatedAt, &v.CreatedAt, &v.UpdatedAt)
}

func (r *InstalledAppValidationRepo) GetByDeviceID(ctx context.Context, deviceID uuid.UUID) ([]domain.InstalledAppValidation, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT `+validationColumns+` FROM installed_app_validations
		WHERE device_id = $1 ORDER BY app_name`, deviceID)
	if err != nil {
		return nil, fmt.Errorf("get validations by device: %w", err)
	}
	defer rows.Close()

	results := make([]domain.InstalledAppValidation, 0)
	for rows.Next() {
		var v domain.InstalledAppValidation
		if err := scanInstalledAppValidation(rows, &v); err != nil {
			return nil, fmt.Errorf("scan validation: %w", err)
		}
		results = append(results, v)
	}
	return results, rows.Err()
}

func (r *InstalledAppValidationRepo) GetByEmployeeID(ctx context.Context, employeeID uuid.UUID) ([]domain.InstalledAppValidationWithDetails, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT v.id, v.device_id, v.app_name, v.role_id, v.display_name, v.agent_category, v.validated_category,
			v.is_compliant, v.reason, v.validated_at, v.created_at, v.updated_at,
			COALESCE(d.hostname, ''), e.id::text, CONCAT(e.first_name, ' ', e.last_name), COALESCE(r.name, '')
		FROM installed_app_validations v
		JOIN devices d ON d.id = v.device_id
		JOIN employees e ON e.id = d.employee_id
		LEFT JOIN roles r ON r.id = v.role_id
		WHERE d.employee_id = $1
		ORDER BY v.app_name`, employeeID)
	if err != nil {
		return nil, fmt.Errorf("get validations by employee: %w", err)
	}
	defer rows.Close()

	results := make([]domain.InstalledAppValidationWithDetails, 0)
	for rows.Next() {
		var v domain.InstalledAppValidationWithDetails
		if err := rows.Scan(
			&v.ID, &v.DeviceID, &v.AppName, &v.RoleID, &v.DisplayName, &v.AgentCategory, &v.ValidatedCategory,
			&v.IsCompliant, &v.Reason, &v.ValidatedAt, &v.CreatedAt, &v.UpdatedAt,
			&v.DeviceHostname, &v.EmployeeID, &v.EmployeeName, &v.RoleName,
		); err != nil {
			return nil, fmt.Errorf("scan validation with details: %w", err)
		}
		results = append(results, v)
	}
	return results, rows.Err()
}

func (r *InstalledAppValidationRepo) GetAllByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.InstalledAppValidationWithDetails, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT v.id, v.device_id, v.app_name, v.role_id, v.display_name, v.agent_category, v.validated_category,
			v.is_compliant, v.reason, v.validated_at, v.created_at, v.updated_at,
			COALESCE(d.hostname, ''), e.id::text, CONCAT(e.first_name, ' ', e.last_name), COALESCE(r.name, '')
		FROM installed_app_validations v
		JOIN devices d ON d.id = v.device_id
		JOIN employees e ON e.id = d.employee_id
		LEFT JOIN roles r ON r.id = v.role_id
		WHERE e.company_id = $1
		ORDER BY v.app_name`, companyID)
	if err != nil {
		return nil, fmt.Errorf("get all validations by company: %w", err)
	}
	defer rows.Close()

	results := make([]domain.InstalledAppValidationWithDetails, 0)
	for rows.Next() {
		var v domain.InstalledAppValidationWithDetails
		if err := rows.Scan(
			&v.ID, &v.DeviceID, &v.AppName, &v.RoleID, &v.DisplayName, &v.AgentCategory, &v.ValidatedCategory,
			&v.IsCompliant, &v.Reason, &v.ValidatedAt, &v.CreatedAt, &v.UpdatedAt,
			&v.DeviceHostname, &v.EmployeeID, &v.EmployeeName, &v.RoleName,
		); err != nil {
			return nil, fmt.Errorf("scan validation with details: %w", err)
		}
		results = append(results, v)
	}
	return results, rows.Err()
}

func (r *InstalledAppValidationRepo) GetNonCompliantByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.InstalledAppValidationWithDetails, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT v.id, v.device_id, v.app_name, v.role_id, v.display_name, v.agent_category, v.validated_category,
			v.is_compliant, v.reason, v.validated_at, v.created_at, v.updated_at,
			COALESCE(d.hostname, ''), e.id::text, CONCAT(e.first_name, ' ', e.last_name), COALESCE(r.name, '')
		FROM installed_app_validations v
		JOIN devices d ON d.id = v.device_id
		JOIN employees e ON e.id = d.employee_id
		LEFT JOIN roles r ON r.id = v.role_id
		WHERE e.company_id = $1 AND v.is_compliant = false
		ORDER BY v.app_name`, companyID)
	if err != nil {
		return nil, fmt.Errorf("get non-compliant by company: %w", err)
	}
	defer rows.Close()

	results := make([]domain.InstalledAppValidationWithDetails, 0)
	for rows.Next() {
		var v domain.InstalledAppValidationWithDetails
		if err := rows.Scan(
			&v.ID, &v.DeviceID, &v.AppName, &v.RoleID, &v.DisplayName, &v.AgentCategory, &v.ValidatedCategory,
			&v.IsCompliant, &v.Reason, &v.ValidatedAt, &v.CreatedAt, &v.UpdatedAt,
			&v.DeviceHostname, &v.EmployeeID, &v.EmployeeName, &v.RoleName,
		); err != nil {
			return nil, fmt.Errorf("scan validation with details: %w", err)
		}
		results = append(results, v)
	}
	return results, rows.Err()
}

func (r *InstalledAppValidationRepo) DeleteByDeviceID(ctx context.Context, deviceID uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `DELETE FROM installed_app_validations WHERE device_id = $1`, deviceID)
	if err != nil {
		return fmt.Errorf("delete validations by device: %w", err)
	}
	return nil
}