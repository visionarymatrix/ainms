package service

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/google/uuid"
)

type ProjectService struct {
	projectRepo *postgres.ProjectRepo
}

func NewProjectService(projectRepo *postgres.ProjectRepo) *ProjectService {
	return &ProjectService{
		projectRepo: projectRepo,
	}
}

func (s *ProjectService) CreateProject(ctx context.Context, project *domain.Project) (*domain.Project, error) {
	if project.Status == "" {
		project.Status = "active"
	}
	if project.WorkingApps == nil {
		project.WorkingApps = []string{}
	}
	if err := s.projectRepo.Create(ctx, project); err != nil {
		return nil, fmt.Errorf("create project: %w", err)
	}
	return project, nil
}

func (s *ProjectService) GetProject(ctx context.Context, id uuid.UUID) (*domain.Project, error) {
	return s.projectRepo.GetByID(ctx, id)
}

func (s *ProjectService) ListProjectsByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.Project, error) {
	return s.projectRepo.ListByCompany(ctx, companyID)
}

func (s *ProjectService) UpdateProject(ctx context.Context, project *domain.Project) error {
	return s.projectRepo.Update(ctx, project)
}

func (s *ProjectService) DeleteProject(ctx context.Context, id uuid.UUID) error {
	return s.projectRepo.Delete(ctx, id)
}

func (s *ProjectService) AssignEmployee(ctx context.Context, assignment *domain.EmployeeProjectAssignment) (*domain.EmployeeProjectAssignment, error) {
	if err := s.projectRepo.AssignEmployee(ctx, assignment); err != nil {
		return nil, fmt.Errorf("assign employee to project: %w", err)
	}
	return assignment, nil
}

func (s *ProjectService) UnassignEmployee(ctx context.Context, employeeID, projectID uuid.UUID) error {
	return s.projectRepo.UnassignEmployee(ctx, employeeID, projectID)
}

func (s *ProjectService) ListEmployeeProjects(ctx context.Context, employeeID uuid.UUID) ([]domain.EmployeeProjectAssignment, error) {
	return s.projectRepo.ListByEmployee(ctx, employeeID)
}

func (s *ProjectService) ListActiveEmployeeProjects(ctx context.Context, employeeID uuid.UUID) ([]domain.EmployeeProjectAssignment, error) {
	return s.projectRepo.ListActiveByEmployee(ctx, employeeID)
}

func (s *ProjectService) ListProjectAssignments(ctx context.Context, projectID uuid.UUID) ([]domain.EmployeeProjectAssignment, error) {
	return s.projectRepo.ListByProject(ctx, projectID)
}