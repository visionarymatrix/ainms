package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"syscall"
	"time"

	"github.com/ainms/gateway/internal/config"
	"github.com/ainms/gateway/internal/handler"
	"github.com/ainms/gateway/internal/middleware"
	"github.com/ainms/gateway/internal/ollama"
	"github.com/ainms/gateway/internal/repository/clickhouse"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/ainms/gateway/internal/service"
	redisstore "github.com/ainms/gateway/internal/store/redis"

	socketio "github.com/zishang520/socket.io/servers/socket/v3"
	"github.com/zishang520/socket.io/v3/pkg/types"

	"github.com/go-chi/chi/v5"
	chimw "github.com/go-chi/chi/v5/middleware"
	"github.com/go-chi/cors"
)

func main() {
	cfg, err := config.Load()
	if err != nil {
		log.Fatalf("failed to load config: %v", err)
	}

	// Resolve upload directory to absolute path so it works regardless of working directory.
	uploadDir, err := filepath.Abs(cfg.UploadDir)
	if err != nil {
		log.Fatalf("failed to resolve upload directory %q: %v", cfg.UploadDir, err)
	}
	log.Printf("upload directory: %s", uploadDir)

	ctx := context.Background()

	pgPool, err := postgres.NewPool(ctx, cfg.Postgres.DSN())
	if err != nil {
		log.Fatalf("failed to connect to PostgreSQL: %v", err)
	}
	defer pgPool.Close()

	chConn, err := clickhouse.NewConn(ctx, cfg.ClickHouse.DSN())
	if err != nil {
		log.Printf("WARNING: ClickHouse unavailable (analytics endpoints disabled): %v", err)
		chConn = nil
	} else {
		defer chConn.Close()
	}

	rdb := redisstore.NewClient(cfg.Redis.Addr(), cfg.Redis.Password, cfg.Redis.DB)
	defer rdb.Close()

	companyRepo := postgres.NewCompanyRepo(pgPool)
	employeeRepo := postgres.NewEmployeeRepo(pgPool)
	deviceRepo := postgres.NewDeviceRepo(pgPool)
	userRepo := postgres.NewUserRepo(pgPool)
	tenantRepo := postgres.NewTenantRepo(pgPool)
	installTokenRepo := postgres.NewInstallTokenRepo(pgPool)
	screenshotRepo := postgres.NewScreenshotRepo(pgPool)
	commandRepo := postgres.NewCommandRepo(pgPool)
	roleRepo := postgres.NewRoleRepo(pgPool)
	appClassificationRepo := postgres.NewAppClassificationRepo(pgPool)
	alertRuleRepo := postgres.NewAlertRuleRepo(pgPool)
	policyRepo := postgres.NewPolicyRepo(pgPool)
	eventRepo := clickhouse.NewEventRepo(pgPool)
	installedAppRepo := postgres.NewInstalledAppRepo(pgPool)
	installedAppValidationRepo := postgres.NewInstalledAppValidationRepo(pgPool)
	targetedScheduleRepo := postgres.NewTargetedScheduleRepo(pgPool)
	projectRepo := postgres.NewProjectRepo(pgPool)
	activityAnalysisRepo := postgres.NewActivityAnalysisRepo(pgPool)

	companySvc := service.NewCompanyService(companyRepo)
	employeeSvc := service.NewEmployeeService(employeeRepo)
	enrollmentSvc := service.NewEnrollmentService(employeeRepo, deviceRepo, roleRepo, appClassificationRepo, alertRuleRepo, policyRepo)
	installTokenSvc := service.NewInstallTokenService(installTokenRepo, employeeRepo)
	authSvc := service.NewAuthService(userRepo, companyRepo, tenantRepo)
	screenshotSvc := service.NewScreenshotService(screenshotRepo, commandRepo, deviceRepo, employeeRepo, uploadDir)
	roleSvc := service.NewRoleService(roleRepo)
	installedAppSvc := service.NewInstalledAppService(installedAppRepo, installedAppValidationRepo, deviceRepo, employeeRepo, appClassificationRepo, roleRepo)

	ollamaClient := ollama.NewClient(cfg.Ollama.BaseURL, cfg.Ollama.APIKey, cfg.Ollama.Model, cfg.Ollama.Timeout)
	complianceAlertRepo := postgres.NewComplianceAlertRepo(pgPool)
	complianceSvc := service.NewComplianceService(complianceAlertRepo, screenshotRepo, screenshotSvc, deviceRepo, employeeRepo, eventRepo, activityAnalysisRepo, ollamaClient, uploadDir)

	socketOpts := socketio.DefaultServerOptions()
	socketOpts.SetCors(&types.Cors{
		Origin:      []string{"http://localhost:3440", "http://localhost:3440"},
		Methods:     []string{"GET", "POST"},
		Credentials: true,
	})
	socketOpts.SetTransports(types.NewSet(socketio.Polling, socketio.WebSocket))

	sio := socketio.NewServer(nil, socketOpts)

	socketHub := service.NewSocketHub(sio)
	targetedScheduleSvc := service.NewTargetedScheduleService(targetedScheduleRepo, employeeRepo, deviceRepo, screenshotSvc, socketHub, screenshotRepo)
	projectSvc := service.NewProjectService(projectRepo)

	socketHandler := handler.NewSocketHandler(socketHub, installTokenSvc, authSvc, enrollmentSvc, screenshotSvc)
	socketHandler.RegisterEvents(sio)

	if err := authSvc.SeedSuperAdmin(ctx); err != nil {
		log.Printf("warning: failed to seed super admin: %v", err)
	} else {
		log.Println("super admin seeded successfully")
	}

	if err := os.MkdirAll(uploadDir, 0755); err != nil {
		log.Printf("warning: failed to create screenshots directory: %v", err)
	}

	r := chi.NewRouter()
	r.Use(chimw.RequestID)
	r.Use(chimw.RealIP)
	r.Use(chimw.Logger)
	r.Use(chimw.Recoverer)
	r.Use(chimw.Timeout(60 * time.Second))

	r.Use(cors.Handler(cors.Options{
		AllowedOrigins:   []string{"http://localhost:3440", "http://localhost:3440", "http://localhost:3000"},
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

	// Public: agent binary download
	r.Get("/v1/agent/download", handler.AgentDownload())

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
			r.Get("/companies/{companyID}/validations", handler.GetCompanyValidations(installedAppSvc))
			r.Get("/companies/{companyID}/non-compliant-apps", handler.GetCompanyNonCompliantApps(installedAppSvc))
			r.Patch("/employees/{employeeID}", handler.UpdateEmployee(employeeSvc))
			r.Delete("/employees/{employeeID}", handler.DeactivateEmployee(employeeSvc))
			r.Post("/employees/{employeeID}/query", handler.PostNLQuery(employeeRepo, deviceRepo, socketHub))

			r.Post("/enroll", handler.EnrollDevice(enrollmentSvc))
			r.Put("/devices/{deviceID}/heartbeat", handler.DeviceHeartbeat(enrollmentSvc))

			r.Post("/install-tokens", handler.GenerateInstallToken(installTokenSvc))
			r.Get("/install-tokens", handler.ListInstallTokens(installTokenSvc))
			r.Delete("/install-tokens/{tokenID}", handler.RevokeInstallToken(installTokenSvc))

			r.Post("/devices/{deviceID}/approve", handler.ApproveDevice(enrollmentSvc))
			r.Post("/devices/{deviceID}/reject", handler.RejectDevice(enrollmentSvc))
			r.Get("/devices/pending", handler.ListPendingDevices(enrollmentSvc))

			r.Get("/rules/sync", handler.SyncRules(appClassificationRepo, alertRuleRepo, policyRepo, deviceRepo, employeeRepo, companyRepo, roleRepo))

			r.Get("/models", handler.GetLLMModels())

			r.Post("/events/bulk", handler.BulkEvents(eventRepo))
			r.Post("/events/app-usage", handler.AppUsage(eventRepo))
			r.Post("/events/priority", handler.PriorityEvent(eventRepo))
			r.Post("/events/popup", handler.PopupEvent(eventRepo))
			r.Post("/events/browser-tabs", handler.BrowserTabsEvent(eventRepo))
			r.Post("/events/network", handler.NetworkTrafficEvent(eventRepo))
			r.Post("/events/activity-summaries", handler.ActivitySummaries(eventRepo))
			r.Post("/events/installed-apps", handler.UploadInstalledApps(installedAppSvc))
			r.Get("/events/installed-apps/{deviceID}", handler.GetDeviceInstalledApps(installedAppRepo))

			r.Get("/devices/{deviceID}/usage-summaries", handler.GetDeviceUsageSummaries(eventRepo))
			r.Get("/devices/{deviceID}/app-usage", handler.GetDeviceAppUsageByDate(eventRepo))
			r.Get("/devices/{deviceID}/events", handler.GetDeviceEvents(eventRepo))
			r.Get("/devices/{deviceID}/activity-summaries", handler.GetDeviceActivitySummaries(eventRepo))
			r.Get("/devices/{deviceID}/installed-apps", handler.GetDeviceInstalledApps(installedAppRepo))
			r.Get("/employees/{employeeID}/installed-apps", handler.GetEmployeeInstalledApps(installedAppRepo))

			// Installed app validation results
			r.Get("/devices/{deviceID}/validations", handler.GetDeviceValidations(installedAppSvc))
			r.Get("/employees/{employeeID}/validations", handler.GetEmployeeValidations(installedAppSvc))

			r.Get("/devices/status", handler.DeviceFleetStatus(enrollmentSvc))

			// Screenshot: admin requests, agent uploads, admin views
			r.Post("/screenshot/request", handler.RequestScreenshot(screenshotSvc, socketHub))
			r.Post("/screenshot/upload", handler.UploadScreenshot(screenshotSvc, socketHub, complianceSvc))
			r.Post("/screenshot/batch-upload", handler.BatchUploadScreenshots(screenshotSvc, socketHub, complianceSvc))

			r.Get("/devices/{deviceID}/alerts", handler.GetDeviceAlerts(complianceSvc))
			r.Post("/alerts/{alertID}/ack", handler.AckAlert(complianceSvc))
			r.Get("/companies/{companyID}/alerts", handler.ListCompanyAlerts(complianceSvc))
			r.Get("/employees/{employeeID}/alerts", handler.ListEmployeeAlerts(complianceSvc))
			r.Get("/devices/{deviceID}/screenshots", handler.GetDeviceScreenshots(screenshotSvc))
			r.Get("/screenshots/{requestID}/image", handler.GetScreenshotImage(screenshotSvc))

			// Targeted screenshot schedules
			r.Post("/targeted-schedules", handler.CreateTargetedSchedule(targetedScheduleSvc))
			r.Get("/targeted-schedules", handler.ListTargetedSchedules(targetedScheduleSvc))
			r.Get("/targeted-schedules/{scheduleID}", handler.GetTargetedSchedule(targetedScheduleSvc))
			r.Put("/targeted-schedules/{scheduleID}", handler.UpdateTargetedSchedule(targetedScheduleSvc))
			r.Delete("/targeted-schedules/{scheduleID}", handler.DeleteTargetedSchedule(targetedScheduleSvc))
			r.Get("/targeted-screenshots", handler.GetTargetedScreenshots(targetedScheduleSvc))
			r.Get("/targeted-schedules/{scheduleID}/screenshots", handler.GetScheduleScreenshots(targetedScheduleSvc))
			r.Get("/employees/{employeeID}/targeted-schedules", handler.ListTargetedSchedulesByEmployee(targetedScheduleSvc))

			// Agent commands: agent polls for pending commands
			r.Get("/devices/{deviceID}/commands", handler.GetPendingCommands(screenshotSvc))
			r.Post("/commands/ack", handler.AcknowledgeCommand(screenshotSvc))

			// Role CRUD
			r.Post("/companies/{companyID}/roles", handler.CreateRole(roleSvc))
			r.Get("/companies/{companyID}/roles", handler.ListRoles(roleSvc))
			r.Get("/roles/{roleID}", handler.GetRole(roleSvc))
			r.Put("/roles/{roleID}", handler.UpdateRole(roleSvc))
			r.Delete("/roles/{roleID}", handler.DeleteRole(roleSvc))

			// AppClassification CRUD (role-scoped)
			r.Get("/roles/{roleID}/app-classifications", handler.ListAppClassifications(appClassificationRepo))
			r.Post("/roles/{roleID}/app-classifications", handler.CreateAppClassification(appClassificationRepo))
			r.Delete("/roles/{roleID}/app-classifications/{classificationID}", handler.DeleteAppClassification(appClassificationRepo))

			// AlertRule CRUD (role-scoped)
			r.Get("/roles/{roleID}/alert-rules", handler.ListAlertRules(alertRuleRepo))
			r.Post("/roles/{roleID}/alert-rules", handler.CreateAlertRule(alertRuleRepo))
			r.Delete("/roles/{roleID}/alert-rules/{ruleID}", handler.DeleteAlertRule(alertRuleRepo))

			// Project CRUD
			r.Post("/projects", handler.CreateProject(projectSvc))
			r.Get("/projects", handler.ListProjects(projectSvc))
			r.Get("/projects/{projectID}", handler.GetProject(projectSvc))
			r.Put("/projects/{projectID}", handler.UpdateProject(projectSvc))
			r.Delete("/projects/{projectID}", handler.DeleteProject(projectSvc))
			r.Post("/projects/{projectID}/assignments", handler.AssignEmployeeToProject(projectSvc))
			r.Delete("/projects/{projectID}/assignments/{employeeID}", handler.UnassignEmployeeFromProject(projectSvc))
			r.Get("/employees/{employeeID}/projects", handler.ListEmployeeProjects(projectSvc))
			r.Get("/employees/{employeeID}/ai-activity", handler.GetAIActivityAnalysis(complianceSvc, projectSvc))
		})
	})

	addr := fmt.Sprintf("%s:%d", cfg.Server.Host, cfg.Server.Port)

	mux := http.NewServeMux()
	mux.Handle("/socketio/", sio.ServeHandler(nil))
	mux.Handle("/", r)

	srv := &http.Server{
		Addr:         addr,
		Handler:      mux,
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

	// Periodic screenshot image cleanup: every 30 minutes, delete image files
	// for screenshots that were analyzed more than 1 hour ago.
		go func() {
		ticker := time.NewTicker(30 * time.Minute)
		defer ticker.Stop()
		for range ticker.C {
			cleaned, err := screenshotSvc.CleanupOldScreenshots(context.Background(), 60)
			if err != nil {
				log.Printf("[cleanup] screenshot cleanup error: %v", err)
			} else if cleaned > 0 {
				log.Printf("[cleanup] removed %d old screenshot images", cleaned)
			}
		}
	}()

	schedulerCancelCtx, schedulerCancel := context.WithCancel(ctx)
	defer schedulerCancel()
	targetedScheduleSvc.StartScheduler(schedulerCancelCtx)

	<-done
	log.Println("shutting down...")

	sio.Close(nil)

	shutdownCtx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	if err := srv.Shutdown(shutdownCtx); err != nil {
		log.Fatalf("server shutdown error: %v", err)
	}

	log.Println("server stopped")
}