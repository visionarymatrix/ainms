package domain

import (
	"fmt"
	"time"

	"github.com/google/uuid"
)

type Tenant struct {
	ID        uuid.UUID  `json:"id" db:"id"`
	Name      string     `json:"name" db:"name"`
	Plan      string     `json:"plan" db:"plan"`
	Settings  JSONMap     `json:"settings" db:"settings"`
	CreatedAt time.Time   `json:"created_at" db:"created_at"`
	UpdatedAt time.Time   `json:"updated_at" db:"updated_at"`
}

type Company struct {
	ID        uuid.UUID  `json:"id" db:"id"`
	TenantID  uuid.UUID  `json:"tenant_id" db:"tenant_id"`
	Name      string     `json:"name" db:"name"`
	Plan      string     `json:"plan" db:"plan"`
	Settings  JSONMap     `json:"settings" db:"settings"`
	CreatedAt time.Time   `json:"created_at" db:"created_at"`
	UpdatedAt time.Time   `json:"updated_at" db:"updated_at"`
}

type CreateCompanyRequest struct {
	TenantID string `json:"tenant_id" validate:"required,uuid"`
	Name     string `json:"name" validate:"required,min=1,max=255"`
	Plan     string `json:"plan" validate:"omitempty,oneof=starter professional enterprise"`
}

type Employee struct {
	ID         uuid.UUID `json:"id" db:"id"`
	CompanyID  uuid.UUID `json:"company_id" db:"company_id"`
	EmployeeID string    `json:"employee_id" db:"employee_id"`
	FirstName  string    `json:"first_name" db:"first_name"`
	LastName   string    `json:"last_name" db:"last_name"`
	Email      *string   `json:"email" db:"email"`
	RoleID     *uuid.UUID `json:"role_id" db:"role_id"`
	Status     string    `json:"status" db:"status"`
	CreatedAt  time.Time `json:"created_at" db:"created_at"`
	UpdatedAt  time.Time `json:"updated_at" db:"updated_at"`
}

type RegisterEmployeeRequest struct {
	FirstName string  `json:"first_name" validate:"required,min=1,max=255"`
	LastName  string  `json:"last_name" validate:"required,min=1,max=255"`
	Email     *string `json:"email" validate:"omitempty,email"`
	RoleID    *string `json:"role_id" validate:"omitempty,uuid"`
}

type Device struct {
	ID               uuid.UUID  `json:"id" db:"id"`
	EmployeeID       uuid.UUID  `json:"employee_id" db:"employee_id"`
	Hostname         *string    `json:"hostname" db:"hostname"`
	OSType           string     `json:"os_type" db:"os_type"`
	OSVersion        *string    `json:"os_version" db:"os_version"`
	AgentVersion     *string    `json:"agent_version" db:"agent_version"`
	MTLSCertSN       *string    `json:"mtls_cert_sn" db:"mtls_cert_sn"`
	Status           string     `json:"status" db:"status"`
	LastHeartbeat    *time.Time `json:"last_heartbeat" db:"last_heartbeat"`
	EnrolledAt       time.Time  `json:"enrolled_at" db:"enrolled_at"`
	Fingerprint      *string    `json:"fingerprint" db:"fingerprint"`
	CPUInfo          *string    `json:"cpu_info" db:"cpu_info"`
	RAMInfo          *string    `json:"ram_info" db:"ram_info"`
	DiskInfo         *string    `json:"disk_info" db:"disk_info"`
	MACAddresses     *string    `json:"mac_addresses" db:"mac_addresses"`
	IPAddresses      *string    `json:"ip_addresses" db:"ip_addresses"`
	ApprovedBy       *uuid.UUID `json:"approved_by" db:"approved_by"`
	ApprovedAt       *time.Time `json:"approved_at" db:"approved_at"`
	CreatedAt        time.Time  `json:"created_at" db:"created_at"`
	UpdatedAt        time.Time  `json:"updated_at" db:"updated_at"`
	ConnectionStatus string     `json:"connection_status" db:"-"`
}

type DeviceSession struct {
	ID            uuid.UUID  `json:"id" db:"id"`
	DeviceID      uuid.UUID  `json:"device_id" db:"device_id"`
	ConnectedAt   time.Time  `json:"connected_at" db:"connected_at"`
	DisconnectedAt *time.Time `json:"disconnected_at" db:"disconnected_at"`
	Status        string     `json:"status" db:"status"`
}

type EnrollmentRequest struct {
	EmployeeID   string `json:"employee_id" validate:"required"`
	CompanyID    string `json:"company_id" validate:"required,uuid"`
	Hostname     string `json:"hostname" validate:"omitempty"`
	OSType       string `json:"os_type" validate:"required,oneof=windows macos linux"`
	OSVersion    string `json:"os_version" validate:"omitempty"`
	Fingerprint  string `json:"fingerprint" validate:"required"`
	CPUInfo      string `json:"cpu_info" validate:"omitempty"`
	RAMInfo      string `json:"ram_info" validate:"omitempty"`
	DiskInfo     string `json:"disk_info" validate:"omitempty"`
	MACAddresses string `json:"mac_addresses" validate:"omitempty"`
	IPAddresses  string `json:"ip_addresses" validate:"omitempty"`
}

type EnrollmentResponse struct {
	DeviceID    uuid.UUID       `json:"device_id"`
	EmployeeID  string          `json:"employee_id"`
	Employee    Employee        `json:"employee"`
	DeviceToken string          `json:"device_token"`
	Status      string          `json:"status"`
	Rules       RuleSetResponse `json:"rules"`
}

type AppClassification struct {
	ID        uuid.UUID `json:"id" db:"id"`
	RoleID    uuid.UUID `json:"role_id" db:"role_id"`
	AppName   string    `json:"app_name" db:"app_name"`
	Category  string    `json:"category" db:"category"`
	CreatedAt time.Time `json:"created_at" db:"created_at"`
}

type AlertRule struct {
	ID           uuid.UUID `json:"id" db:"id"`
	RoleID       uuid.UUID `json:"role_id" db:"role_id"`
	Category     string    `json:"category" db:"category"`
	ThresholdMin int       `json:"threshold_min" db:"threshold_min"`
	PopupType   string    `json:"popup_type" db:"popup_type"`
	CreatedAt    time.Time `json:"created_at" db:"created_at"`
}

type Policy struct {
	ID                uuid.UUID `json:"id" db:"id"`
	TenantID          uuid.UUID `json:"tenant_id" db:"tenant_id"`
	UploadInterval    int       `json:"upload_interval" db:"upload_interval"`
	ScreenshotEnabled bool      `json:"screenshot_enabled" db:"screenshot_enabled"`
	ScreenshotPolicy  string    `json:"screenshot_policy" db:"screenshot_policy"`
	CreatedAt         time.Time `json:"created_at" db:"created_at"`
	UpdatedAt         time.Time `json:"updated_at" db:"updated_at"`
}

type RuleSetResponse struct {
	AppClassifications []AppClassification `json:"app_classifications"`
	AlertRules        []AlertRule          `json:"alert_rules"`
	Policy            Policy               `json:"policy"`
}

type BulkEventRequest struct {
	DeviceID string              `json:"device_id" validate:"required,uuid"`
	Summary  AppUsageSummary      `json:"summary"`
	Metadata []AppUsageEventMeta `json:"metadata" validate:"min=1"`
}

type AppUsageSummary struct {
	DeviceID            string  `json:"device_id"`
	AppName             string  `json:"app_name"`
	TotalDurationSec   float64 `json:"total_duration_sec"`
	SessionCount        int     `json:"session_count"`
	ProductiveDuration  float64 `json:"productive_duration_sec"`
	UnproductiveDuration float64 `json:"unproductive_duration_sec"`
	NeutralDuration     float64 `json:"neutral_duration_sec"`
}

type AppUsageEventMeta struct {
	AppName      string            `json:"app_name"`
	WindowTitle  string            `json:"window_title"`
	ProcessName  string            `json:"process_name"`
	ProcessID    int               `json:"process_id"`
	StartTime    time.Time         `json:"start_time"`
	EndTime      time.Time         `json:"end_time"`
	DurationSec  float64           `json:"duration_sec"`
	Classification string         `json:"classification"`
	Confidence   float64           `json:"confidence"`
	RoleID       *uuid.UUID        `json:"role_id"`
	DeviceID     uuid.UUID         `json:"device_id"`
}

type PriorityEventRequest struct {
	DeviceID      uuid.UUID `json:"device_id" validate:"required"`
	EventType     string    `json:"event_type" validate:"required"`
	AppName       string    `json:"app_name"`
	WindowTitle   string    `json:"window_title"`
	Classification string  `json:"classification"`
	Confidence    float64   `json:"confidence"`
	Timestamp     time.Time `json:"timestamp"`
}

type PopupEvent struct {
	DeviceID    uuid.UUID `json:"device_id" validate:"required"`
	AppName     string    `json:"app_name" validate:"required"`
	WindowTitle string    `json:"window_title"`
	Explanation string    `json:"explanation" validate:"required"`
	PopupType   string    `json:"popup_type" validate:"required,oneof=toast modal soft_block"`
	Classification string `json:"classification"`
	Confidence  float64   `json:"confidence"`
	Timestamp   time.Time `json:"timestamp"`
}

type ScreenshotRequest struct {
	DeviceID    uuid.UUID `json:"device_id" validate:"required,uuid"`
	RequestedBy uuid.UUID `json:"requested_by" validate:"required,uuid"`
	Reason      string    `json:"reason" validate:"required,min=10"`
	Policy      string    `json:"policy" validate:"required,oneof=upload_image metadata_only"`
}

type ScreenshotUpload struct {
	RequestID   uuid.UUID `json:"request_id" validate:"required,uuid"`
	DeviceID    uuid.UUID `json:"device_id" validate:"required,uuid"`
	Category    string    `json:"category"`
	Confidence  float64   `json:"confidence"`
	ImageURL    *string   `json:"image_url"`
	UploadedAt  time.Time `json:"uploaded_at"`
}

// JSONMap is a helper type for JSONB columns.
type JSONMap map[string]interface{}

// GenerateEmployeeID creates a sequential employee ID in the format EMP-XXXX.
func GenerateEmployeeID(sequence int) string {
	return fmt.Sprintf("EMP-%04d", sequence)
}