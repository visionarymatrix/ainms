package service

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/ainms/gateway/internal/domain"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/google/uuid"
)

const serverBaseURL = "http://127.0.0.1:8440"

type InstallTokenService struct {
	tokenRepo    *postgres.InstallTokenRepo
	employeeRepo *postgres.EmployeeRepo
}

func NewInstallTokenService(tokenRepo *postgres.InstallTokenRepo, employeeRepo *postgres.EmployeeRepo) *InstallTokenService {
	return &InstallTokenService{
		tokenRepo:    tokenRepo,
		employeeRepo: employeeRepo,
	}
}

func generateSecureToken() (string, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return "", fmt.Errorf("generate secure token: %w", err)
	}
	return hex.EncodeToString(b), nil
}

func parseExpiresIn(expiresIn string) (time.Duration, error) {
	s := strings.TrimSpace(expiresIn)
	if s == "" {
		return 0, nil
	}

	if len(s) < 2 {
		return 0, fmt.Errorf("invalid expires_in format: %s", expiresIn)
	}

	numStr := s[:len(s)-1]
	unit := strings.ToLower(s[len(s)-1:])

	num, err := strconv.Atoi(numStr)
	if err != nil {
		return 0, fmt.Errorf("invalid expires_in value: %s", expiresIn)
	}

	switch unit {
	case "h":
		return time.Duration(num) * time.Hour, nil
	case "d":
		return time.Duration(num) * 24 * time.Hour, nil
	default:
		return 0, fmt.Errorf("invalid expires_in unit: %s (use h or d)", unit)
	}
}

func (s *InstallTokenService) Generate(ctx context.Context, req domain.CreateInstallTokenRequest, createdBy uuid.UUID) (*domain.InstallTokenResponse, error) {
	employeeID, err := uuid.Parse(req.EmployeeID)
	if err != nil {
		return nil, fmt.Errorf("invalid employee_id: %w", err)
	}
	companyID, err := uuid.Parse(req.CompanyID)
	if err != nil {
		return nil, fmt.Errorf("invalid company_id: %w", err)
	}

	employee, err := s.employeeRepo.GetByID(ctx, employeeID)
	if err != nil {
		return nil, fmt.Errorf("employee not found: %w", err)
	}
	if employee.Status != "active" {
		return nil, fmt.Errorf("employee %s is %s, cannot generate install token", employee.EmployeeID, employee.Status)
	}

	secureToken, err := generateSecureToken()
	if err != nil {
		return nil, err
	}

	var expiresAt *time.Time
	if req.ExpiresIn != nil && *req.ExpiresIn != "" {
		duration, err := parseExpiresIn(*req.ExpiresIn)
		if err != nil {
			return nil, err
		}
		if duration > 0 {
			ea := time.Now().Add(duration)
			expiresAt = &ea
		}
	}

	token := &domain.InstallToken{
		Token:       secureToken,
		EmployeeID:  employeeID,
		CompanyID:   companyID,
		Description: req.Description,
		ExpiresAt:   expiresAt,
		CreatedBy:   createdBy,
	}

	if err := s.tokenRepo.Create(ctx, token); err != nil {
		return nil, fmt.Errorf("create install token: %w", err)
	}

	resp := &domain.InstallTokenResponse{
		ID:          token.ID,
		Token:       token.Token,
		InstallCmd:  fmt.Sprintf(`curl -fsSL %s/v1/install.sh | sudo bash -s -- --token %s`, serverBaseURL, token.Token),
		WindowsCmd:  fmt.Sprintf(`powershell -WindowStyle Hidden -c "iwr '%s/v1/install.ps1?token=%s' -UseBasicParsing | iex"`, serverBaseURL, token.Token),
		EmployeeID:  token.EmployeeID,
		CompanyID:   token.CompanyID,
		Description: token.Description,
		ExpiresAt:   token.ExpiresAt,
		CreatedAt:   token.CreatedAt,
	}

	return resp, nil
}

func (s *InstallTokenService) ListByCompany(ctx context.Context, companyID uuid.UUID) ([]domain.InstallToken, error) {
	return s.tokenRepo.ListByCompany(ctx, companyID)
}

func (s *InstallTokenService) Revoke(ctx context.Context, tokenID uuid.UUID) error {
	return s.tokenRepo.Revoke(ctx, tokenID)
}

func (s *InstallTokenService) Validate(ctx context.Context, tokenString string) (*domain.InstallTokenClaims, error) {
	token, err := s.tokenRepo.GetByTokenWithCompany(ctx, tokenString)
	if err != nil {
		return nil, fmt.Errorf("invalid install token")
	}

	if token.RevokedAt != nil {
		return nil, fmt.Errorf("install token has been revoked")
	}
	if token.ExpiresAt != nil && time.Now().After(*token.ExpiresAt) {
		return nil, fmt.Errorf("install token has expired")
	}

	return &domain.InstallTokenClaims{
		EmployeeID: token.EmployeeID.String(),
		CompanyID:  token.CompanyID.String(),
		Role:       "device",
	}, nil
}

func (s *InstallTokenService) ValidateInstallToken(tokenStr string) (*domain.InstallTokenClaims, error) {
	return s.Validate(context.Background(), tokenStr)
}

func (s *InstallTokenService) GetOrCreateForEmployee(ctx context.Context, employeeID, companyID, createdBy string) (*domain.InstallTokenResponse, error) {
	empUUID, err := uuid.Parse(employeeID)
	if err != nil {
		return nil, fmt.Errorf("invalid employee_id: %w", err)
	}
	_, err = uuid.Parse(companyID)
	if err != nil {
		return nil, fmt.Errorf("invalid company_id: %w", err)
	}
	createdByUUID, err := uuid.Parse(createdBy)
	if err != nil {
		return nil, fmt.Errorf("invalid created_by: %w", err)
	}

	existing, err := s.tokenRepo.GetActiveByEmployee(ctx, empUUID)
	if err == nil && existing != nil {
		resp := &domain.InstallTokenResponse{
			ID:          existing.ID,
			Token:       existing.Token,
			InstallCmd:  fmt.Sprintf(`curl -fsSL %s/v1/install.sh | sudo bash -s -- --token %s`, serverBaseURL, existing.Token),
			WindowsCmd:  fmt.Sprintf(`powershell -WindowStyle Hidden -c "iwr '%s/v1/install.ps1?token=%s' -UseBasicParsing | iex"`, serverBaseURL, existing.Token),
			EmployeeID:  existing.EmployeeID,
			CompanyID:   existing.CompanyID,
			Description: existing.Description,
			ExpiresAt:   existing.ExpiresAt,
			CreatedAt:   existing.CreatedAt,
		}
		return resp, nil
	}

	req := domain.CreateInstallTokenRequest{
		EmployeeID: employeeID,
		CompanyID:  companyID,
	}
	return s.Generate(ctx, req, createdByUUID)
}