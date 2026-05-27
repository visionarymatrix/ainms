<#
.SYNOPSIS
    AINMS Agent Installer / Uninstaller for Windows

.DESCRIPTION
    Installs or uninstalls the AINMS Agent as a Windows service.
    - Install: copies binary to Program Files, creates config, registers service with recovery options, adds firewall rule
    - Uninstall: stops service, removes service, firewall rule, install directory

.PARAMETER Install
    Run in install mode (default)

.PARAMETER Uninstall
    Run in uninstall mode

.PARAMETER ExePath
    Path to the agent-core.exe release binary. Default: looks in ..\target\release\agent-core.exe relative to this script

.PARAMETER EmployeeId
    Employee ID for the config file

.PARAMETER CompanyId
    Company ID (UUID format) for the config file

.PARAMETER Server
    AINMS server URL. Default: http://localhost:8440

.PARAMETER AuthEmail
    Authentication email. Default: superadmin@ainms.io

.PARAMETER AuthPassword
    Authentication password. Default: changeme

.PARAMETER Silent
    Skip interactive prompts, use defaults or provided parameters only

.EXAMPLE
    .\install-ainms-agent.ps1 -EmployeeId "EMP-0042" -CompanyId "550e8400-e29b-41d4-a716-446655440000"

.EXAMPLE
    .\install-ainms-agent.ps1 -Uninstall

.EXAMPLE
    .\install-ainms-agent.ps1 -Silent -EmployeeId "EMP-0042" -CompanyId "550e8400-e29b-41d4-a716-446655440000" -Server "https://ainms.example.com:8440"
#>

[CmdletBinding()]
param(
    [switch]$Install,
    [switch]$Uninstall,
    [string]$ExePath,
    [string]$EmployeeId,
    [string]$CompanyId,
    [string]$Server,
    [string]$AuthEmail,
    [string]$AuthPassword,
    [switch]$Silent
)

# ── Constants ──────────────────────────────────────────────────────────────

$ServiceName    = "AINMSAgent"
$ServiceDisplay  = "AINMS Agent"
$ServiceDesc     = "AINMS workplace accountability agent"
$InstallDir      = "${env:ProgramFiles}\AINMS\Agent"
$ConfigFile      = "$InstallDir\ainms-agent.toml"
$ExeName         = "agent-core.exe"
$FirewallRule    = "AINMS Agent - Outbound"

# Default server/auth values
$DefaultServer       = "http://localhost:8440"
$DefaultAuthEmail    = "superadmin@ainms.io"
$DefaultAuthPassword = "changeme"

# ── Helper Functions ────────────────────────────────────────────────────────

function Write-Status($msg) {
    Write-Host "[AINMS] $msg" -ForegroundColor Cyan
}

function Write-OK($msg) {
    Write-Host "[OK]    $msg" -ForegroundColor Green
}

function Write-Warn($msg) {
    Write-Host "[WARN]  $msg" -ForegroundColor Yellow
}

function Write-Fail($msg) {
    Write-Host "[FAIL]  $msg" -ForegroundColor Red
}

function Test-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-Prompt($Prompt, $Default) {
    if ($Silent) { return $Default }
    $display = if ($Default) { "$Prompt [$Default]" } else { $Prompt }
    $response = Read-Host $display
    if ([string]::IsNullOrWhiteSpace($response)) { $Default } else { $response }
}

# PowerShell 5.1 mangles sc.exe arguments (especially binPath= with spaces).
# Solution: invoke sc.exe via a temporary .bat file where cmd.exe handles quoting natively.
# sc.exe binPath= requires: the entire value after "binPath= " as one token.
# In cmd.exe, nested quotes use \" inside the outer quotes.
function Invoke-ScExe {
    param([string]$Arguments)
    $batPath = Join-Path $env:TEMP "ainms-sc-cmd.bat"
    Set-Content -LiteralPath $batPath -Value "sc.exe $Arguments" -Encoding ASCII
    # Run the bat file and check exit code. Output goes to null to avoid binary garbage
    # from the service process leaking into the pipeline.
    & $batPath | Out-Null
    $exitCode = $LASTEXITCODE
    Remove-Item -LiteralPath $batPath -Force -ErrorAction SilentlyContinue
    # sc.exe returns 0 on success, non-zero on failure
    return $exitCode
}

# ── Admin Check ────────────────────────────────────────────────────────────

if (-not (Test-Admin)) {
    Write-Fail "This script requires Administrator privileges."
    Write-Fail "Right-click PowerShell and select 'Run as Administrator', then re-run this script."
    exit 1
}

# ── Default mode is Install ────────────────────────────────────────────────

if (-not $Install -and -not $Uninstall) {
    $Install = $true
}

# ═══════════════════════════════════════════════════════════════════════════
# UNINSTALL
# ═══════════════════════════════════════════════════════════════════════════

if ($Uninstall) {
    Write-Status "Uninstalling AINMS Agent..."

    # Stop service if running
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if ($svc) {
        if ($svc.Status -ne 'Stopped') {
            Write-Status "Stopping service..."
            Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
            Start-Sleep -Seconds 3
        }

        # Remove service via sc.exe (more reliable than agent-core.exe uninstall)
        Write-Status "Removing service..."
        Invoke-ScExe "delete $ServiceName" | Out-Null
        Write-OK "Service removed"
    } else {
        Write-Warn "Service not found (already removed?)"
    }

    # Remove firewall rule
    $fwRule = Get-NetFirewallRule -DisplayName $FirewallRule -ErrorAction SilentlyContinue
    if ($fwRule) {
        Remove-NetFirewallRule -DisplayName $FirewallRule -ErrorAction SilentlyContinue
        Write-OK "Firewall rule removed"
    }

    # Remove install directory
    if (Test-Path -LiteralPath $InstallDir) {
        Write-Status "Removing install directory: $InstallDir"
        Remove-Item -LiteralPath $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
        if (-not (Test-Path -LiteralPath $InstallDir)) {
            Write-OK "Install directory removed"
        } else {
            Write-Warn "Could not remove install directory (files may be locked). Reboot and delete manually: $InstallDir"
        }
    }

    Write-OK "AINMS Agent uninstalled successfully."
    exit 0
}

# ═══════════════════════════════════════════════════════════════════════════
# INSTALL
# ═══════════════════════════════════════════════════════════════════════════

Write-Status "AINMS Agent Installer v0.2.0"
Write-Status "============================="

# ── Locate binary ────────────────────────────────────────────────────────

if (-not $ExePath) {
    $scriptDir = $PSScriptRoot
    if (-not $scriptDir) { $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition }

    # Try relative to script: ../target/release/agent-core.exe
    $candidate = Join-Path $scriptDir "..\target\release\$ExeName"
    $candidate = [System.IO.Path]::GetFullPath($candidate)
    if (Test-Path -LiteralPath $candidate) {
        $ExePath = $candidate
    } else {
        # Try same directory as script
        $candidate = Join-Path $scriptDir $ExeName
        if (Test-Path -LiteralPath $candidate) {
            $ExePath = $candidate
        }
    }
}

if (-not $ExePath -or -not (Test-Path -LiteralPath $ExePath)) {
    Write-Fail "Cannot find agent-core.exe"
    Write-Fail "Specify path with -ExePath or run from the agent project directory"
    exit 1
}

$ExePath = [System.IO.Path]::GetFullPath($ExePath)
Write-Status "Binary: $ExePath"

# ── Collect configuration ──────────────────────────────────────────────────

Write-Status ""
Write-Status "Configuration"
Write-Status "-------------"

$resolvedEmployeeId = if ($EmployeeId) { $EmployeeId } else { Get-Prompt "Employee ID" "unknown" }
$resolvedCompanyId  = if ($CompanyId)  { $CompanyId  } else { Get-Prompt "Company ID (UUID)" "unknown" }
$resolvedServer     = if ($Server)     { $Server     } else { Get-Prompt "Server URL" $DefaultServer }
$resolvedAuthEmail  = if ($AuthEmail)  { $AuthEmail  } else { Get-Prompt "Auth Email" $DefaultAuthEmail }
$resolvedAuthPassword = if ($AuthPassword) { $AuthPassword } else {
    if ($Silent) { $DefaultAuthPassword } else {
        # Mask password input
        $secPwd = Read-Host "Auth Password" -AsSecureString
        [Runtime.InteropServices.Marshal]::PtrToStringAuto([Runtime.InteropServices.Marshal]::SecureStringToBSTR($secPwd))
    }
}
if (-not $resolvedAuthPassword) { $resolvedAuthPassword = $DefaultAuthPassword }

# Validate company_id is UUID-ish (warn only)
if ($resolvedCompanyId -ne "unknown" -and $resolvedCompanyId -notmatch '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$') {
    Write-Warn "Company ID does not look like a UUID. The server may reject it."
}

# ── Create install directory ───────────────────────────────────────────────

if (-not (Test-Path -LiteralPath $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Write-OK "Created directory: $InstallDir"
}

# ── Copy binary ──────────────────────────────────────────────────────────

$destExe = Join-Path $InstallDir $ExeName
Copy-Item -LiteralPath $ExePath -Destination $destExe -Force
Write-OK "Copied binary to: $destExe"

# ── Write config file ──────────────────────────────────────────────────────

$configContent = @"
# AINMS Agent Configuration
# Generated by installer on $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')

employee_id = "$resolvedEmployeeId"
company_id  = "$resolvedCompanyId"
server      = "$resolvedServer"
auth_email  = "$resolvedAuthEmail"
auth_password = "$resolvedAuthPassword"
"@

Set-Content -LiteralPath $ConfigFile -Value $configContent -Encoding UTF8
Write-OK "Config written to: $ConfigFile"

# ── Remove existing service if present ──────────────────────────────────────

$existingSvc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existingSvc) {
    Write-Status "Existing service found, removing..."
    if ($existingSvc.Status -ne 'Stopped') {
        Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 3
    }
    Invoke-ScExe "delete $ServiceName" | Out-Null
    Start-Sleep -Seconds 2
    Write-OK "Removed existing service"
}

# ── Register service with sc.exe ──────────────────────────────────────────

# Using sc.exe directly gives us full control over recovery options
# which agent-core.exe install (via windows-service crate) doesn't configure

Write-Status "Registering Windows service..."

# sc.exe requires nested quotes around binPath value when it contains spaces.
# For cmd.exe .bat files, \" represents a literal quote inside a quoted parameter.
# PowerShell backtick-quote `"` produces a literal " character in the string.
# So `"\`"$destExe\`"`" produces  "\"C:\path\""  in the .bat file.
$binPathArg = "`"\`"$destExe\`" --run-as-service --config \`"$ConfigFile\`"`""
$exitCode = Invoke-ScExe "create $ServiceName binPath= $binPathArg start= auto DisplayName= `"$ServiceDisplay`""

if ($exitCode -ne 0) {
    Write-Fail "Failed to create service (exit code $exitCode)"
    exit 1
}

# Set description
Invoke-ScExe "description $ServiceName `"$ServiceDesc`"" | Out-Null

# ── Configure service recovery (restart on failure) ───────────────────────

# Recovery policy: restart immediately on first failure, restart after 5s on second,
# restart after 30s on subsequent failures. Reset failure count after 24 hours.
# This matches the architecture spec: SC_ACTION_RESTART with 0-second delay

$recoveryCode = Invoke-ScExe "failure $ServiceName reset= 86400 actions= restart/0/restart/5000/restart/30000"
if ($recoveryCode -ne 0) {
    Write-Warn "Could not configure service recovery (non-fatal)"
}

Write-OK "Service registered with auto-restart recovery policy"

# ── Add firewall rule for outbound HTTPS ──────────────────────────────────

$fwRule = Get-NetFirewallRule -DisplayName $FirewallRule -ErrorAction SilentlyContinue
if (-not $fwRule) {
    # Allow outbound from the agent binary
    New-NetFirewallRule -DisplayName $FirewallRule `
        -Direction Outbound `
        -Action Allow `
        -Program $destExe `
        -Protocol TCP `
        -Profile Any `
        -Enabled True `
        -ErrorAction SilentlyContinue | Out-Null

    Write-OK "Added firewall rule: $FirewallRule"
} else {
    Write-Status "Firewall rule already exists"
}

# ── Start service ──────────────────────────────────────────────────────────

Write-Status "Starting service..."
Start-Service -Name $ServiceName -ErrorAction SilentlyContinue
Start-Sleep -Seconds 3

# Verify
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($svc -and $svc.Status -eq 'Running') {
    Write-OK "Service started successfully"
} else {
    Write-Warn "Service may still be starting. Check with: sc.exe query $ServiceName"
}

# ── Summary ────────────────────────────────────────────────────────────────

Write-Host ""
Write-OK "=== AINMS Agent Installed Successfully ===" 
Write-Host ""
Write-Host "  Install dir : $InstallDir"
Write-Host "  Binary      : $destExe"
Write-Host "  Config      : $ConfigFile"
Write-Host "  Service     : $ServiceName"
Write-Host "  Employee ID : $resolvedEmployeeId"
Write-Host "  Server      : $resolvedServer"
Write-Host ""
Write-Host "  Manage service:"
Write-Host "    Start   : Start-Service $ServiceName"
Write-Host "    Stop    : Stop-Service $ServiceName"
Write-Host "    Status  : Get-Service $ServiceName"
Write-Host "    Remove  : $PSCommandPath -Uninstall"
Write-Host ""