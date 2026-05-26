package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type DeviceRepo struct {
	pool *pgxpool.Pool
}

func NewDeviceRepo(pool *pgxpool.Pool) *DeviceRepo {
	return &DeviceRepo{pool: pool}
}

const deviceColumns = `id, employee_id, hostname, os_type, os_version, agent_version, mtls_cert_sn, status, last_heartbeat, enrolled_at, fingerprint, cpu_info, ram_info, disk_info, mac_addresses, ip_addresses, approved_by, approved_at, created_at, updated_at`

const deviceColumnsQualified = `d.id, d.employee_id, d.hostname, d.os_type, d.os_version, d.agent_version, d.mtls_cert_sn, d.status, d.last_heartbeat, d.enrolled_at, d.fingerprint, d.cpu_info, d.ram_info, d.disk_info, d.mac_addresses, d.ip_addresses, d.approved_by, d.approved_at, d.created_at, d.updated_at`

func scanDevice(scanner interface{ Scan(...interface{}) error }, d *domain.Device) error {
	return scanner.Scan(
		&d.ID, &d.EmployeeID, &d.Hostname, &d.OSType, &d.OSVersion,
		&d.AgentVersion, &d.MTLSCertSN, &d.Status, &d.LastHeartbeat,
		&d.EnrolledAt, &d.Fingerprint, &d.CPUInfo, &d.RAMInfo, &d.DiskInfo,
		&d.MACAddresses, &d.IPAddresses, &d.ApprovedBy, &d.ApprovedAt,
		&d.CreatedAt, &d.UpdatedAt,
	)
}

func (r *DeviceRepo) Create(ctx context.Context, device *domain.Device) error {
	query := `INSERT INTO devices (id, employee_id, hostname, os_type, os_version, agent_version, mtls_cert_sn, status, fingerprint, cpu_info, ram_info, disk_info, mac_addresses, ip_addresses, enrolled_at, created_at, updated_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW(), NOW(), NOW())
		RETURNING enrolled_at, created_at, updated_at`

	if device.ID == uuid.Nil {
		device.ID = uuid.New()
	}

	return r.pool.QueryRow(ctx, query,
		device.ID, device.EmployeeID, device.Hostname, device.OSType,
		device.OSVersion, device.AgentVersion, device.MTLSCertSN, device.Status,
		device.Fingerprint, device.CPUInfo, device.RAMInfo, device.DiskInfo,
		device.MACAddresses, device.IPAddresses,
	).Scan(&device.EnrolledAt, &device.CreatedAt, &device.UpdatedAt)
}

func (r *DeviceRepo) GetByID(ctx context.Context, id uuid.UUID) (*domain.Device, error) {
	query := `SELECT ` + deviceColumns + ` FROM devices WHERE id = $1`

	var d domain.Device
	if err := scanDevice(r.pool.QueryRow(ctx, query, id), &d); err != nil {
		return nil, fmt.Errorf("get device: %w", err)
	}
	return &d, nil
}

func (r *DeviceRepo) GetByEmployeeID(ctx context.Context, employeeID uuid.UUID) ([]domain.Device, error) {
	query := `SELECT ` + deviceColumns + ` FROM devices WHERE employee_id = $1 ORDER BY enrolled_at DESC`

	rows, err := r.pool.Query(ctx, query, employeeID)
	if err != nil {
		return nil, fmt.Errorf("get devices by employee: %w", err)
	}
	defer rows.Close()

	devices := make([]domain.Device, 0)
	for rows.Next() {
		var d domain.Device
		if err := scanDevice(rows, &d); err != nil {
			return nil, fmt.Errorf("scan device: %w", err)
		}
		devices = append(devices, d)
	}
	return devices, rows.Err()
}

func (r *DeviceRepo) List(ctx context.Context) ([]domain.Device, error) {
	query := `SELECT ` + deviceColumns + ` FROM devices ORDER BY enrolled_at DESC`

	rows, err := r.pool.Query(ctx, query)
	if err != nil {
		return nil, fmt.Errorf("list devices: %w", err)
	}
	defer rows.Close()

	devices := make([]domain.Device, 0)
	for rows.Next() {
		var d domain.Device
		if err := scanDevice(rows, &d); err != nil {
			return nil, fmt.Errorf("scan device: %w", err)
		}
		devices = append(devices, d)
	}
	return devices, rows.Err()
}

func (r *DeviceRepo) UpdateHeartbeat(ctx context.Context, id uuid.UUID) error {
	query := `UPDATE devices SET last_heartbeat = NOW(), updated_at = NOW() WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("update heartbeat: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("device not found: %s", id)
	}
	return nil
}

func (r *DeviceRepo) UpdateAgentVersion(ctx context.Context, id uuid.UUID, version string) error {
	query := `UPDATE devices SET agent_version = $2, updated_at = NOW() WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, id, version)
	if err != nil {
		return fmt.Errorf("update agent_version: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("device not found: %s", id)
	}
	return nil
}

func (r *DeviceRepo) UpdateStatus(ctx context.Context, id uuid.UUID, status string) error {
	query := `UPDATE devices SET status = $2, updated_at = NOW() WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, id, status)
	if err != nil {
		return fmt.Errorf("update device status: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("device not found: %s", id)
	}
	return nil
}

func (r *DeviceRepo) UpdateApproval(ctx context.Context, deviceID uuid.UUID, approvedBy uuid.UUID) error {
	query := `UPDATE devices SET status = 'active', approved_by = $2, approved_at = NOW(), updated_at = NOW() WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, deviceID, approvedBy)
	if err != nil {
		return fmt.Errorf("approve device: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("device not found: %s", deviceID)
	}
	return nil
}

func (r *DeviceRepo) ListByStatus(ctx context.Context, status string) ([]domain.Device, error) {
	query := `SELECT ` + deviceColumns + ` FROM devices WHERE status = $1 ORDER BY enrolled_at DESC`

	rows, err := r.pool.Query(ctx, query, status)
	if err != nil {
		return nil, fmt.Errorf("list devices by status: %w", err)
	}
	defer rows.Close()

	devices := make([]domain.Device, 0)
	for rows.Next() {
		var d domain.Device
		if err := scanDevice(rows, &d); err != nil {
			return nil, fmt.Errorf("scan device: %w", err)
		}
		devices = append(devices, d)
	}
	return devices, rows.Err()
}

func (r *DeviceRepo) ListByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.Device, error) {
	query := `SELECT ` + deviceColumnsQualified + `
		FROM devices d
		JOIN employees e ON d.employee_id = e.id
		WHERE e.company_id = $1
		ORDER BY d.enrolled_at DESC`

	rows, err := r.pool.Query(ctx, query, companyID)
	if err != nil {
		return nil, fmt.Errorf("list devices by company: %w", err)
	}
	defer rows.Close()

	devices := make([]domain.Device, 0)
	for rows.Next() {
		var d domain.Device
		if err := scanDevice(rows, &d); err != nil {
			return nil, fmt.Errorf("scan device: %w", err)
		}
		devices = append(devices, d)
	}
	return devices, rows.Err()
}

func (r *DeviceRepo) ListPendingByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.Device, error) {
	query := `SELECT ` + deviceColumnsQualified + `
		FROM devices d
		JOIN employees e ON d.employee_id = e.id
		WHERE e.company_id = $1 AND d.status = 'pending'
		ORDER BY d.enrolled_at DESC`

	rows, err := r.pool.Query(ctx, query, companyID)
	if err != nil {
		return nil, fmt.Errorf("list pending devices by company: %w", err)
	}
	defer rows.Close()

	devices := make([]domain.Device, 0)
	for rows.Next() {
		var d domain.Device
		if err := scanDevice(rows, &d); err != nil {
			return nil, fmt.Errorf("scan device: %w", err)
		}
		devices = append(devices, d)
	}
	return devices, rows.Err()
}

func (r *DeviceRepo) ListAllPending(ctx context.Context) ([]domain.Device, error) {
	query := `SELECT ` + deviceColumnsQualified + `
		FROM devices d
		WHERE d.status = 'pending'
		ORDER BY d.enrolled_at DESC`

	rows, err := r.pool.Query(ctx, query)
	if err != nil {
		return nil, fmt.Errorf("list all pending devices: %w", err)
	}
	defer rows.Close()

	devices := make([]domain.Device, 0)
	for rows.Next() {
		var d domain.Device
		if err := scanDevice(rows, &d); err != nil {
			return nil, fmt.Errorf("scan device: %w", err)
		}
		devices = append(devices, d)
	}
	return devices, rows.Err()
}

func (r *DeviceRepo) GetDeviceStatus(ctx context.Context, deviceID uuid.UUID) (string, error) {
	query := `SELECT status FROM devices WHERE id = $1`
	var status string
	err := r.pool.QueryRow(ctx, query, deviceID).Scan(&status)
	if err != nil {
		return "", fmt.Errorf("get device status: %w", err)
	}
	return status, nil
}

func (r *DeviceRepo) GetByFingerprint(ctx context.Context, fingerprint string, employeeID uuid.UUID) (*domain.Device, error) {
	query := `SELECT ` + deviceColumns + ` FROM devices WHERE fingerprint = $1 AND employee_id = $2 LIMIT 1`

	var d domain.Device
	if err := scanDevice(r.pool.QueryRow(ctx, query, fingerprint, employeeID), &d); err != nil {
		return nil, fmt.Errorf("get device by fingerprint: %w", err)
	}
	return &d, nil
}

func (r *DeviceRepo) UpdateHardwareInfo(ctx context.Context, id uuid.UUID, updates map[string]interface{}) error {
	if len(updates) == 0 {
		return nil
	}

	setParts := ""
	argNum := 1
	vals := []interface{}{}
	for key, val := range updates {
		if setParts != "" {
			setParts += ", "
		}
		setParts += fmt.Sprintf("%s = $%d", key, argNum)
		vals = append(vals, val)
		argNum++
	}
	setParts += ", updated_at = NOW()"
	vals = append(vals, id)
	query := fmt.Sprintf("UPDATE devices SET %s WHERE id = $%d", setParts, argNum)

	res, err := r.pool.Exec(ctx, query, vals...)
	if err != nil {
		return fmt.Errorf("update hardware info: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("device not found: %s", id)
	}
	return nil
}

func (r *DeviceRepo) RejectDevice(ctx context.Context, deviceID uuid.UUID) error {
	query := `UPDATE devices SET status = 'rejected', updated_at = NOW() WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, deviceID)
	if err != nil {
		return fmt.Errorf("reject device: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("device not found: %s", deviceID)
	}
	return nil
}