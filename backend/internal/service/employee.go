package service

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/google/uuid"
)

type EmployeeService struct {
	repo *postgres.EmployeeRepo
}

func NewEmployeeService(repo *postgres.EmployeeRepo) *EmployeeService {
	return &EmployeeService{repo: repo}
}

func (s *EmployeeService) Register(ctx context.Context, companyID uuid.UUID, req domain.RegisterEmployeeRequest) (*domain.Employee, error) {
	if req.FirstName == "" {
		return nil, fmt.Errorf("first name is required")
	}
	if req.LastName == "" {
		return nil, fmt.Errorf("last name is required")
	}

	employeeID, err := s.repo.NextEmployeeID(ctx, companyID)
	if err != nil {
		return nil, fmt.Errorf("generate employee id: %w", err)
	}

	emp := &domain.Employee{
		ID:         uuid.New(),
		CompanyID:  companyID,
		EmployeeID: employeeID,
		FirstName:  req.FirstName,
		LastName:   req.LastName,
		Email:      req.Email,
		Status:     "active",
	}

	if req.RoleID != nil {
		roleID, err := uuid.Parse(*req.RoleID)
		if err == nil {
			emp.RoleID = &roleID
		}
	}

	if err := s.repo.Create(ctx, emp); err != nil {
		return nil, fmt.Errorf("create employee: %w", err)
	}

	return emp, nil
}

func (s *EmployeeService) GetByID(ctx context.Context, id uuid.UUID) (*domain.Employee, error) {
	return s.repo.GetByID(ctx, id)
}

func (s *EmployeeService) GetByEmployeeID(ctx context.Context, companyID uuid.UUID, employeeID string) (*domain.Employee, error) {
	return s.repo.GetByEmployeeID(ctx, companyID, employeeID)
}

func (s *EmployeeService) List(ctx context.Context, companyID uuid.UUID) ([]domain.Employee, error) {
	return s.repo.List(ctx, companyID)
}

func (s *EmployeeService) Deactivate(ctx context.Context, id uuid.UUID) error {
	return s.repo.Deactivate(ctx, id)
}