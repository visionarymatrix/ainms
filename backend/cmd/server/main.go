package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/ainms/gateway/internal/config"
	"github.com/ainms/gateway/internal/handler"
	"github.com/ainms/gateway/internal/middleware"
	"github.com/ainms/gateway/internal/repository/clickhouse"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/ainms/gateway/internal/service"
	redisstore "github.com/ainms/gateway/internal/store/redis"

	"github.com/go-chi/chi/v5"
	chimw "github.com/go-chi/chi/v5/middleware"
	"github.com/go-chi/cors"
)

func main() {
	cfg, err := config.Load()
	if err != nil {
		log.Fatalf("failed to load config: %v", err)
	}

	ctx := context.Background()

	pgPool, err := postgres.NewPool(ctx, cfg.Postgres.DSN())
	if err != nil {
		log.Fatalf("failed to connect to PostgreSQL: %v", err)
	}
	defer pgPool.Close()

	chConn, err := clickhouse.NewConn(ctx, cfg.ClickHouse.DSN())
	if err != nil {
		log.Fatalf("failed to connect to ClickHouse: %v", err)
	}
	defer chConn.Close()

	rdb := redisstore.NewClient(cfg.Redis.Addr(), cfg.Redis.Password, cfg.Redis.DB)
	defer rdb.Close()

	companyRepo := postgres.NewCompanyRepo(pgPool)
	employeeRepo := postgres.NewEmployeeRepo(pgPool)
	deviceRepo := postgres.NewDeviceRepo(pgPool)
	userRepo := postgres.NewUserRepo(pgPool)
	tenantRepo := postgres.NewTenantRepo(pgPool)
	installTokenRepo := postgres.NewInstallTokenRepo(pgPool)
	eventRepo := clickhouse.NewEventRepo(pgPool)

	companySvc := service.NewCompanyService(companyRepo)
	employeeSvc := service.NewEmployeeService(employeeRepo)
	enrollmentSvc := service.NewEnrollmentService(employeeRepo, deviceRepo)
	installTokenSvc := service.NewInstallTokenService(installTokenRepo, employeeRepo)
	authSvc := service.NewAuthService(userRepo, companyRepo, tenantRepo)

	if err := authSvc.SeedSuperAdmin(ctx); err != nil {
		log.Printf("warning: failed to seed super admin: %v", err)
	} else {
		log.Println("super admin seeded successfully")
	}

	r := chi.NewRouter()
	r.Use(chimw.RequestID)
	r.Use(chimw.RealIP)
	r.Use(chimw.Logger)
	r.Use(chimw.Recoverer)
	r.Use(chimw.Timeout(60 * time.Second))

	r.Use(cors.Handler(cors.Options{
		AllowedOrigins:   []string{"http://173.249.47.143:3440", "http://localhost:3440", "http://localhost:3000"},
		AllowedMethods:   []string{"GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"},
		AllowedHeaders:   []string{"Accept", "Authorization", "Content-Type"},
		ExposedHeaders:   []string{"Content-Length"},
		AllowCredentials:  true,
		MaxAge:           300,
	}))

	r.Get("/v1/health", handler.Health(pgPool, chConn, rdb))

	// Public auth routes (no auth required)
	r.Post("/v1/auth/login", handler.Login(authSvc))
	r.Post("/v1/auth/register", handler.RegisterCompany(authSvc))

	// Public: token-based enrollment (install_token is the auth)
	r.Post("/v1/enroll/token", handler.EnrollWithToken(enrollmentSvc, installTokenSvc))

	// Public: install scripts for curl|sh and powershell
	r.Get("/v1/install.sh", handler.InstallShellScript())
	r.Get("/v1/install.ps1", handler.InstallPSScript())

	// Public device status endpoint (agent polls without auth)
	r.Get("/v1/devices/{deviceID}/status", handler.GetDeviceStatus(enrollmentSvc))

	// Protected routes
	r.Group(func(r chi.Router) {
		r.Use(middleware.Authenticator(installTokenSvc))

		r.Get("/v1/auth/me", handler.GetCurrentUser(authSvc))

		r.Route("/v1", func(r chi.Router) {
			r.Post("/companies", handler.CreateCompany(companySvc))
			r.Get("/companies", handler.ListCompanies(companySvc))
			r.Get("/companies/{companyID}", handler.GetCompany(companySvc))

			r.Post("/companies/{companyID}/employees", handler.RegisterEmployee(employeeSvc))
			r.Get("/employees/{employeeID}", handler.GetEmployee(employeeSvc))
			r.Get("/employees/{employeeID}/devices", handler.GetEmployeeDevices(deviceRepo))
			r.Post("/employees/{employeeID}/install-token", handler.GenerateEmployeeInstallToken(installTokenSvc, employeeSvc))
			r.Get("/companies/{companyID}/employees", handler.ListEmployees(employeeSvc))
			r.Patch("/employees/{employeeID}", handler.UpdateEmployee(employeeSvc))
			r.Delete("/employees/{employeeID}", handler.DeactivateEmployee(employeeSvc))

			r.Post("/enroll", handler.EnrollDevice(enrollmentSvc))
			r.Put("/devices/{deviceID}/heartbeat", handler.DeviceHeartbeat(enrollmentSvc))

			r.Post("/install-tokens", handler.GenerateInstallToken(installTokenSvc))
			r.Get("/install-tokens", handler.ListInstallTokens(installTokenSvc))
			r.Delete("/install-tokens/{tokenID}", handler.RevokeInstallToken(installTokenSvc))

			r.Post("/devices/{deviceID}/approve", handler.ApproveDevice(enrollmentSvc))
			r.Post("/devices/{deviceID}/reject", handler.RejectDevice(enrollmentSvc))
			r.Get("/devices/pending", handler.ListPendingDevices(enrollmentSvc))

			r.Get("/rules/sync", handler.SyncRules(nil))
			r.Get("/models/latest", handler.GetLatestModel(nil))

			r.Post("/events/bulk", handler.BulkEvents(eventRepo))
			r.Post("/events/priority", handler.PriorityEvent(eventRepo))
			r.Post("/events/popup", handler.PopupEvent(eventRepo))

			r.Get("/devices/{deviceID}/usage-summaries", handler.GetDeviceUsageSummaries(eventRepo))
			r.Get("/devices/{deviceID}/events", handler.GetDeviceEvents(eventRepo))

			r.Get("/devices/status", handler.DeviceFleetStatus(enrollmentSvc))
			r.Post("/screenshot/request", handler.RequestScreenshot(nil))
			r.Post("/screenshot/upload", handler.UploadScreenshot(nil))

			r.Get("/commands", handler.WebSocketCommands(nil))
		})
	})

	addr := fmt.Sprintf("%s:%d", cfg.Server.Host, cfg.Server.Port)
	srv := &http.Server{
		Addr:         addr,
		Handler:      r,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 60 * time.Second,
		IdleTimeout:  120 * time.Second,
	}

	done := make(chan os.Signal, 1)
	signal.Notify(done, os.Interrupt, syscall.SIGTERM)

	go func() {
		log.Printf("starting AINMS gateway on %s", addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("server error: %v", err)
		}
	}()

	<-done
	log.Println("shutting down...")

	shutdownCtx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	if err := srv.Shutdown(shutdownCtx); err != nil {
		log.Fatalf("server shutdown error: %v", err)
	}

	log.Println("server stopped")
}