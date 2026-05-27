package service

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/google/uuid"
)

type RoleService struct {
	roleRepo *postgres.RoleRepo
}

func NewRoleService(roleRepo *postgres.RoleRepo) *RoleService {
	return &RoleService{roleRepo: roleRepo}
}

func (s *RoleService) Create(ctx context.Context, companyID uuid.UUID, req domain.CreateRoleRequest) (*domain.Role, error) {
	if req.Name == "" {
		return nil, fmt.Errorf("name is required")
	}

	role := &domain.Role{
		ID:                uuid.New(),
		CompanyID:         companyID,
		Name:              req.Name,
		Description:       req.Description,
		WorkDescription:   req.WorkDescription,
		AllowedCategories: req.AllowedCategories,
		BlockedCategories: req.BlockedCategories,
	}

	if err := s.roleRepo.Create(ctx, role); err != nil {
		return nil, fmt.Errorf("create role: %w", err)
	}

	return role, nil
}

func (s *RoleService) GetByID(ctx context.Context, id uuid.UUID) (*domain.Role, error) {
	return s.roleRepo.GetByID(ctx, id)
}

func (s *RoleService) List(ctx context.Context, companyID uuid.UUID) ([]domain.Role, error) {
	return s.roleRepo.List(ctx, companyID)
}

func (s *RoleService) Update(ctx context.Context, id uuid.UUID, req domain.UpdateRoleRequest) (*domain.Role, error) {
	role, err := s.roleRepo.GetByID(ctx, id)
	if err != nil {
		return nil, fmt.Errorf("get role: %w", err)
	}

	if req.Name != nil {
		role.Name = *req.Name
	}
	if req.Description != nil {
		role.Description = *req.Description
	}
	if req.WorkDescription != nil {
		role.WorkDescription = *req.WorkDescription
	}
	if req.AllowedCategories != nil {
		role.AllowedCategories = req.AllowedCategories
	}
	if req.BlockedCategories != nil {
		role.BlockedCategories = req.BlockedCategories
	}

	if err := s.roleRepo.Update(ctx, role); err != nil {
		return nil, fmt.Errorf("update role: %w", err)
	}

	return role, nil
}

func (s *RoleService) Delete(ctx context.Context, id uuid.UUID) error {
	return s.roleRepo.Delete(ctx, id)
}