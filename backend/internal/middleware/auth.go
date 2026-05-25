package middleware

import (
	"context"
	"net/http"
	"strings"

	"github.com/ainms/gateway/internal/domain"
	"github.com/golang-jwt/jwt/v5"
)

type contextKey string

const (
	UserIDKey     contextKey = "user_id"
	EmailKey      contextKey = "email"
	RoleKey       contextKey = "role"
	CompanyIDKey  contextKey = "company_id"
	EmployeeIDKey contextKey = "employee_id"
)

var JWTSecret = []byte("ainms-dev-secret-change-in-production")

type TokenValidator interface {
	ValidateInstallToken(tokenStr string) (*domain.InstallTokenClaims, error)
}

func Authenticator(validator TokenValidator) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			authHeader := r.Header.Get("Authorization")
			if authHeader == "" {
				http.Error(w, `{"error":"authorization header required"}`, http.StatusUnauthorized)
				return
			}

			tokenStr := strings.TrimPrefix(authHeader, "Bearer ")
			if tokenStr == authHeader {
				http.Error(w, `{"error":"invalid authorization format"}`, http.StatusUnauthorized)
				return
			}

			token, err := jwt.Parse(tokenStr, func(token *jwt.Token) (interface{}, error) {
				if _, ok := token.Method.(*jwt.SigningMethodHMAC); !ok {
					return nil, jwt.ErrSignatureInvalid
				}
				return JWTSecret, nil
			})

			if err == nil && token.Valid {
				claims, ok := token.Claims.(jwt.MapClaims)
				if !ok {
					http.Error(w, `{"error":"invalid token claims"}`, http.StatusUnauthorized)
					return
				}

				userID, _ := claims["user_id"].(string)
				email, _ := claims["email"].(string)
				role, _ := claims["role"].(string)

				var companyID *string
				if cid, ok := claims["company_id"]; ok && cid != nil {
					if cidStr, ok := cid.(string); ok {
						companyID = &cidStr
					}
				}

				ctx := r.Context()
				ctx = context.WithValue(ctx, UserIDKey, userID)
				ctx = context.WithValue(ctx, EmailKey, email)
				ctx = context.WithValue(ctx, RoleKey, role)
				ctx = context.WithValue(ctx, CompanyIDKey, companyID)

				next.ServeHTTP(w, r.WithContext(ctx))
				return
			}

			installClaims, err := validator.ValidateInstallToken(tokenStr)
			if err != nil {
				http.Error(w, `{"error":"invalid or expired token"}`, http.StatusUnauthorized)
				return
			}

			ctx := r.Context()
			ctx = context.WithValue(ctx, UserIDKey, installClaims.EmployeeID)
			ctx = context.WithValue(ctx, RoleKey, installClaims.Role)
			ctx = context.WithValue(ctx, CompanyIDKey, &installClaims.CompanyID)
			ctx = context.WithValue(ctx, EmployeeIDKey, installClaims.EmployeeID)

			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

func RequireRole(roles ...string) func(http.Handler) http.Handler {
	roleSet := make(map[string]bool)
	for _, r := range roles {
		roleSet[r] = true
	}
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			role, _ := r.Context().Value(RoleKey).(string)
			if !roleSet[role] {
				http.Error(w, `{"error":"forbidden"}`, http.StatusForbidden)
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}

func GetUserID(ctx context.Context) string {
	id, _ := ctx.Value(UserIDKey).(string)
	return id
}

func GetEmail(ctx context.Context) string {
	email, _ := ctx.Value(EmailKey).(string)
	return email
}

func GetRole(ctx context.Context) string {
	role, _ := ctx.Value(RoleKey).(string)
	return role
}

func GetCompanyID(ctx context.Context) *string {
	cid, _ := ctx.Value(CompanyIDKey).(*string)
	return cid
}

func GetEmployeeID(ctx context.Context) string {
	id, _ := ctx.Value(EmployeeIDKey).(string)
	return id
}