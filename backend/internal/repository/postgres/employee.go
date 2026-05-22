package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type EmployeeRepo struct {
	pool *pgxpool.Pool
}

func NewEmployeeRepo(pool *pgxpool.Pool) *EmployeeRepo {
	return &EmployeeRepo{pool: pool}
}

func (r *EmployeeRepo) Create(ctx context.Context, emp *domain.Employee) error {
	query := `INSERT INTO employees (id, company_id, employee_id, first_name, last_name, email, role_id, status, created_at, updated_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
		RETURNING created_at, updated_at`

	if emp.ID == uuid.Nil {
		emp.ID = uuid.New()
	}

	return r.pool.QueryRow(ctx, query,
		emp.ID, emp.CompanyID, emp.EmployeeID, emp.FirstName, emp.LastName,
		emp.Email, emp.RoleID, emp.Status,
	).Scan(&emp.CreatedAt, &emp.UpdatedAt)
}

func (r *EmployeeRepo) GetByID(ctx context.Context, id uuid.UUID) (*domain.Employee, error) {
	query := `SELECT id, company_id, employee_id, first_name, last_name, email, role_id, status, created_at, updated_at
		FROM employees WHERE id = $1`

	var e domain.Employee
	err := r.pool.QueryRow(ctx, query, id).Scan(
		&e.ID, &e.CompanyID, &e.EmployeeID, &e.FirstName, &e.LastName,
		&e.Email, &e.RoleID, &e.Status, &e.CreatedAt, &e.UpdatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("get employee: %w", err)
	}
	return &e, nil
}

func (r *EmployeeRepo) GetByEmployeeID(ctx context.Context, companyID uuid.UUID, employeeID string) (*domain.Employee, error) {
	var query string
	var args []interface{}

	if companyID != uuid.Nil {
		query = `SELECT id, company_id, employee_id, first_name, last_name, email, role_id, status, created_at, updated_at
			FROM employees WHERE company_id = $1 AND employee_id = $2`
		args = []interface{}{companyID, employeeID}
	} else {
		query = `SELECT id, company_id, employee_id, first_name, last_name, email, role_id, status, created_at, updated_at
			FROM employees WHERE employee_id = $1 LIMIT 1`
		args = []interface{}{employeeID}
	}

	var e domain.Employee
	err := r.pool.QueryRow(ctx, query, args...).Scan(
		&e.ID, &e.CompanyID, &e.EmployeeID, &e.FirstName, &e.LastName,
		&e.Email, &e.RoleID, &e.Status, &e.CreatedAt, &e.UpdatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("get employee by employee_id: %w", err)
	}
	return &e, nil
}

func (r *EmployeeRepo) List(ctx context.Context, companyID uuid.UUID) ([]domain.Employee, error) {
	query := `SELECT id, company_id, employee_id, first_name, last_name, email, role_id, status, created_at, updated_at
		FROM employees WHERE company_id = $1 ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query, companyID)
	if err != nil {
		return nil, fmt.Errorf("list employees: %w", err)
	}
	defer rows.Close()

	employees := make([]domain.Employee, 0)
	for rows.Next() {
		var e domain.Employee
		if err := rows.Scan(&e.ID, &e.CompanyID, &e.EmployeeID, &e.FirstName, &e.LastName,
			&e.Email, &e.RoleID, &e.Status, &e.CreatedAt, &e.UpdatedAt); err != nil {
			return nil, fmt.Errorf("scan employee: %w", err)
		}
		employees = append(employees, e)
	}
	return employees, rows.Err()
}

func (r *EmployeeRepo) Update(ctx context.Context, emp *domain.Employee) error {
	query := `UPDATE employees SET first_name = $2, last_name = $3, email = $4, role_id = $5, status = $6, updated_at = NOW()
		WHERE id = $1
		RETURNING updated_at`

	return r.pool.QueryRow(ctx, query,
		emp.ID, emp.FirstName, emp.LastName, emp.Email, emp.RoleID, emp.Status,
	).Scan(&emp.UpdatedAt)
}

func (r *EmployeeRepo) Deactivate(ctx context.Context, id uuid.UUID) error {
	query := `UPDATE employees SET status = 'deactivated', updated_at = NOW() WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("deactivate employee: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("employee not found: %s", id)
	}
	return nil
}

func (r *EmployeeRepo) NextEmployeeID(ctx context.Context, companyID uuid.UUID) (string, error) {
	var maxID *string
	query := `SELECT MAX(employee_id) FROM employees WHERE company_id = $1`
	err := r.pool.QueryRow(ctx, query, companyID).Scan(&maxID)
	if err != nil {
		return "", fmt.Errorf("get next employee id: %w", err)
	}

	if maxID == nil {
		return domain.GenerateEmployeeID(1), nil
	}

	var seq int
	_, err = fmt.Sscanf(*maxID, "EMP-%04d", &seq)
	if err != nil {
		return domain.GenerateEmployeeID(1), nil
	}
	return domain.GenerateEmployeeID(seq + 1), nil
}