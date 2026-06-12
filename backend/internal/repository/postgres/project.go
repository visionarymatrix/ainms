package postgres

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type ProjectRepo struct {
	pool *pgxpool.Pool
}

func NewProjectRepo(pool *pgxpool.Pool) *ProjectRepo {
	return &ProjectRepo{pool: pool}
}

const projectColumns = `id, company_id, name, description, status, working_apps, created_at, updated_at`

func scanProject(scanner interface{ Scan(...interface{}) error }, p *domain.Project) error {
	var workingAppsJSON []byte
	if err := scanner.Scan(
		&p.ID, &p.CompanyID, &p.Name, &p.Description, &p.Status,
		&workingAppsJSON, &p.CreatedAt, &p.UpdatedAt,
	); err != nil {
		return err
	}
	if len(workingAppsJSON) > 0 {
		if err := json.Unmarshal(workingAppsJSON, &p.WorkingApps); err != nil {
			p.WorkingApps = []string{}
		}
	} else {
		p.WorkingApps = []string{}
	}
	return nil
}

func (r *ProjectRepo) Create(ctx context.Context, project *domain.Project) error {
	workingAppsJSON, err := json.Marshal(project.WorkingApps)
	if err != nil {
		return fmt.Errorf("marshal working_apps: %w", err)
	}

	query := `INSERT INTO projects (company_id, name, description, status, working_apps)
		VALUES ($1, $2, $3, $4, $5)
		RETURNING id, created_at, updated_at`

	return r.pool.QueryRow(ctx, query,
		project.CompanyID, project.Name, project.Description, project.Status, workingAppsJSON,
	).Scan(&project.ID, &project.CreatedAt, &project.UpdatedAt)
}

func (r *ProjectRepo) GetByID(ctx context.Context, id uuid.UUID) (*domain.Project, error) {
	query := `SELECT ` + projectColumns + ` FROM projects WHERE id = $1`

	var p domain.Project
	if err := scanProject(r.pool.QueryRow(ctx, query, id), &p); err != nil {
		return nil, fmt.Errorf("get project: %w", err)
	}
	return &p, nil
}

func (r *ProjectRepo) ListByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.Project, error) {
	query := `SELECT ` + projectColumns + ` FROM projects WHERE company_id = $1 ORDER BY created_at DESC`

	rows, err := r.pool.Query(ctx, query, companyID)
	if err != nil {
		return nil, fmt.Errorf("list projects by company: %w", err)
	}
	defer rows.Close()

	projects := make([]domain.Project, 0)
	for rows.Next() {
		var p domain.Project
		if err := scanProject(rows, &p); err != nil {
			return nil, fmt.Errorf("scan project: %w", err)
		}
		projects = append(projects, p)
	}
	return projects, rows.Err()
}

func (r *ProjectRepo) Update(ctx context.Context, project *domain.Project) error {
	workingAppsJSON, err := json.Marshal(project.WorkingApps)
	if err != nil {
		return fmt.Errorf("marshal working_apps: %w", err)
	}

	query := `UPDATE projects
		SET name = $2, description = $3, status = $4, working_apps = $5, updated_at = NOW()
		WHERE id = $1`

	result, err := r.pool.Exec(ctx, query,
		project.ID, project.Name, project.Description, project.Status, workingAppsJSON,
	)
	if err != nil {
		return fmt.Errorf("update project: %w", err)
	}
	if result.RowsAffected() == 0 {
		return fmt.Errorf("project not found")
	}
	return nil
}

func (r *ProjectRepo) Delete(ctx context.Context, id uuid.UUID) error {
	query := `DELETE FROM projects WHERE id = $1`
	result, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("delete project: %w", err)
	}
	if result.RowsAffected() == 0 {
		return fmt.Errorf("project not found")
	}
	return nil
}

const assignmentColumns = `id, employee_id, project_id, assigned_by, started_at, ended_at, is_primary, created_at, updated_at`

func scanAssignment(scanner interface{ Scan(...interface{}) error }, a *domain.EmployeeProjectAssignment) error {
	return scanner.Scan(
		&a.ID, &a.EmployeeID, &a.ProjectID, &a.AssignedBy,
		&a.StartedAt, &a.EndedAt, &a.IsPrimary, &a.CreatedAt, &a.UpdatedAt,
	)
}

func (r *ProjectRepo) AssignEmployee(ctx context.Context, assignment *domain.EmployeeProjectAssignment) error {
	query := `INSERT INTO employee_project_assignments (employee_id, project_id, assigned_by, is_primary)
		VALUES ($1, $2, $3, $4)
		RETURNING id, started_at, created_at, updated_at`

	return r.pool.QueryRow(ctx, query,
		assignment.EmployeeID, assignment.ProjectID, assignment.AssignedBy, assignment.IsPrimary,
	).Scan(&assignment.ID, &assignment.StartedAt, &assignment.CreatedAt, &assignment.UpdatedAt)
}

func (r *ProjectRepo) UnassignEmployee(ctx context.Context, employeeID, projectID uuid.UUID) error {
	query := `UPDATE employee_project_assignments SET ended_at = NOW(), updated_at = NOW()
		WHERE employee_id = $1 AND project_id = $2 AND ended_at IS NULL`

	result, err := r.pool.Exec(ctx, query, employeeID, projectID)
	if err != nil {
		return fmt.Errorf("unassign employee: %w", err)
	}
	if result.RowsAffected() == 0 {
		return fmt.Errorf("active assignment not found")
	}
	return nil
}

func (r *ProjectRepo) ListByEmployee(ctx context.Context, employeeID uuid.UUID) ([]domain.EmployeeProjectAssignment, error) {
	query := `SELECT ` + assignmentColumns + ` FROM employee_project_assignments
		WHERE employee_id = $1 ORDER BY started_at DESC`

	rows, err := r.pool.Query(ctx, query, employeeID)
	if err != nil {
		return nil, fmt.Errorf("list assignments by employee: %w", err)
	}
	defer rows.Close()

	assignments := make([]domain.EmployeeProjectAssignment, 0)
	for rows.Next() {
		var a domain.EmployeeProjectAssignment
		if err := scanAssignment(rows, &a); err != nil {
			return nil, fmt.Errorf("scan assignment: %w", err)
		}
		assignments = append(assignments, a)
	}
	return assignments, rows.Err()
}

func (r *ProjectRepo) ListActiveByEmployee(ctx context.Context, employeeID uuid.UUID) ([]domain.EmployeeProjectAssignment, error) {
	query := `SELECT ` + assignmentColumns + ` FROM employee_project_assignments
		WHERE employee_id = $1 AND ended_at IS NULL ORDER BY is_primary DESC, started_at DESC`

	rows, err := r.pool.Query(ctx, query, employeeID)
	if err != nil {
		return nil, fmt.Errorf("list active assignments by employee: %w", err)
	}
	defer rows.Close()

	assignments := make([]domain.EmployeeProjectAssignment, 0)
	for rows.Next() {
		var a domain.EmployeeProjectAssignment
		if err := scanAssignment(rows, &a); err != nil {
			return nil, fmt.Errorf("scan assignment: %w", err)
		}
		assignments = append(assignments, a)
	}
	return assignments, rows.Err()
}

func (r *ProjectRepo) ListByProject(ctx context.Context, projectID uuid.UUID) ([]domain.EmployeeProjectAssignment, error) {
	query := `SELECT ` + assignmentColumns + ` FROM employee_project_assignments
		WHERE project_id = $1 ORDER BY started_at DESC`

	rows, err := r.pool.Query(ctx, query, projectID)
	if err != nil {
		return nil, fmt.Errorf("list assignments by project: %w", err)
	}
	defer rows.Close()

	assignments := make([]domain.EmployeeProjectAssignment, 0)
	for rows.Next() {
		var a domain.EmployeeProjectAssignment
		if err := scanAssignment(rows, &a); err != nil {
			return nil, fmt.Errorf("scan assignment: %w", err)
		}
		assignments = append(assignments, a)
	}
	return assignments, rows.Err()
}