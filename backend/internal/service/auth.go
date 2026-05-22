package service

import (
	"context"
	"errors"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	"golang.org/x/crypto/bcrypt"
)

var jwtSecret = []byte("ainms-dev-secret-change-in-production")

type AuthService struct {
	userRepo    *postgres.UserRepo
	companyRepo *postgres.CompanyRepo
	tenantRepo  *postgres.TenantRepo
}

func NewAuthService(userRepo *postgres.UserRepo, companyRepo *postgres.CompanyRepo, tenantRepo *postgres.TenantRepo) *AuthService {
	return &AuthService{userRepo: userRepo, companyRepo: companyRepo, tenantRepo: tenantRepo}
}

func (s *AuthService) Login(ctx context.Context, email, password string) (*domain.LoginResponse, error) {
	user, err := s.userRepo.GetByEmail(ctx, email)
	if err != nil {
		return nil, errors.New("invalid credentials")
	}
	if err := bcrypt.CompareHashAndPassword([]byte(user.PasswordHash), []byte(password)); err != nil {
		return nil, errors.New("invalid credentials")
	}

	token, err := generateToken(user)
	if err != nil {
		return nil, err
	}

	return &domain.LoginResponse{Token: token, User: *user}, nil
}

func (s *AuthService) RegisterCompany(ctx context.Context, req domain.RegisterCompanyRequest) (*domain.LoginResponse, error) {
	if _, err := s.userRepo.GetByEmail(ctx, req.AdminEmail); err == nil {
		return nil, errors.New("email already registered")
	}

	tenant := &domain.Tenant{
		ID:   uuid.New(),
		Name: req.CompanyName,
		Plan: "free",
	}
	if err := s.tenantRepo.Create(ctx, tenant); err != nil {
		return nil, err
	}

	company := &domain.Company{
		ID:       uuid.New(),
		TenantID: tenant.ID,
		Name:     req.CompanyName,
		Plan:     req.Plan,
		Settings: domain.JSONMap{},
	}
	if company.Plan == "" {
		company.Plan = "starter"
	}
	if err := s.companyRepo.Create(ctx, company); err != nil {
		return nil, err
	}

	passwordHash, err := bcrypt.GenerateFromPassword([]byte(req.AdminPassword), bcrypt.DefaultCost)
	if err != nil {
		return nil, err
	}

	companyIDStr := company.ID.String()
	user := &domain.User{
		ID:           uuid.New().String(),
		Email:        req.AdminEmail,
		PasswordHash: string(passwordHash),
		Name:         req.AdminName,
		Role:         "company_admin",
		CompanyID:    &companyIDStr,
	}
	if err := s.userRepo.Create(ctx, user); err != nil {
		return nil, err
	}

	token, err := generateToken(user)
	if err != nil {
		return nil, err
	}

	return &domain.LoginResponse{Token: token, User: *user}, nil
}

func (s *AuthService) SeedSuperAdmin(ctx context.Context) error {
	if _, err := s.userRepo.GetByEmail(ctx, "superadmin@ainms.io"); err == nil {
		return nil
	}

	passwordHash, err := bcrypt.GenerateFromPassword([]byte("changeme"), bcrypt.DefaultCost)
	if err != nil {
		return err
	}

	user := &domain.User{
		ID:           uuid.New().String(),
		Email:        "superadmin@ainms.io",
		PasswordHash: string(passwordHash),
		Name:         "Super Admin",
		Role:         "super_admin",
		CompanyID:    nil,
	}
	return s.userRepo.Create(ctx, user)
}

func (s *AuthService) GetUserByID(ctx context.Context, id string) (*domain.User, error) {
	return s.userRepo.GetByID(ctx, id)
}

func generateToken(user *domain.User) (string, error) {
	claims := jwt.MapClaims{
		"user_id":    user.ID,
		"email":      user.Email,
		"role":       user.Role,
		"company_id": user.CompanyID,
		"exp":        time.Now().Add(72 * time.Hour).Unix(),
	}
	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString(jwtSecret)
}