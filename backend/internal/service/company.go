package service

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/google/uuid"
)

type CompanyService struct {
	repo *postgres.CompanyRepo
}

func NewCompanyService(repo *postgres.CompanyRepo) *CompanyService {
	return &CompanyService{repo: repo}
}

func (s *CompanyService) Create(ctx context.Context, company *domain.Company) error {
	if company.ID == uuid.Nil {
		company.ID = uuid.New()
	}
	if company.Plan == "" {
		company.Plan = "starter"
	}
	if company.Settings == nil {
		company.Settings = domain.JSONMap{}
	}
	return s.repo.Create(ctx, company)
}

func (s *CompanyService) GetByID(ctx context.Context, id uuid.UUID) (*domain.Company, error) {
	return s.repo.GetByID(ctx, id)
}

func (s *CompanyService) List(ctx context.Context, tenantID uuid.UUID) ([]domain.Company, error) {
	return s.repo.List(ctx, tenantID)
}

func (s *CompanyService) ListAll(ctx context.Context) ([]domain.Company, error) {
	return s.repo.ListAll(ctx)
}

func (s *CompanyService) Update(ctx context.Context, company *domain.Company) error {
	if company.Name == "" {
		return fmt.Errorf("company name is required")
	}
	return s.repo.Update(ctx, company)
}