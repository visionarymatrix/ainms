package handler

import (
	"net/http"
)

func InstallShellScript() http.HandlerFunc {
	script := `#!/usr/bin/env bash
set -euo pipefail

TOKEN=""
SERVER="http://173.249.47.143:8440"

for arg in "$@"; do
  case "$arg" in
    --token=*) TOKEN="${arg#*=}" ;;
    --token)   shift; TOKEN="$1" ;;
  esac
done

if [ -z "$TOKEN" ]; then
  echo "Error: --token is required"
  echo "Usage: curl -fsSL SERVER/v1/install.sh | sudo bash -s -- --token YOUR_TOKEN"
  exit 1
fi

echo "=== AINMS Agent Installer ==="
echo "Token: ${TOKEN:0:8}..."

OS_TYPE="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS_TYPE" in
  linux)  OS="linux" ;;
  darwin) OS="macos" ;;
  *)
    echo "Error: Unsupported operating system: $OS_TYPE"
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH="amd64" ;;
  aarch64|arm64) ARCH="arm64" ;;
  *)
    echo "Error: Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

echo "Detected OS: $OS ($ARCH)"

BINARY_NAME="ainms-agent"
INSTALL_DIR="/usr/local/bin"
DOWNLOAD_URL="${SERVER}/v1/agent/download?os=${OS}&arch=${ARCH}"

echo "Downloading agent from ${DOWNLOAD_URL}..."
if command -v curl &>/dev/null; then
  curl -fsSL -o "${INSTALL_DIR}/${BINARY_NAME}" "${DOWNLOAD_URL}" 2>/dev/null || {
    echo "Warning: Binary download not yet available. Skipping binary installation."
    echo "The agent binary will need to be installed manually."
  }
elif command -v wget &>/dev/null; then
  wget -q -O "${INSTALL_DIR}/${BINARY_NAME}" "${DOWNLOAD_URL}" 2>/dev/null || {
    echo "Warning: Binary download not yet available. Skipping binary installation."
    echo "The agent binary will need to be installed manually."
  }
fi

if [ -f "${INSTALL_DIR}/${BINARY_NAME}" ]; then
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
fi

echo "Enrolling device with install token..."
ENROLLResp=$(${BINARY_NAME} --install-token "$TOKEN" --server "$SERVER" install 2>/dev/null) || {
  echo "Falling back to API enrollment..."
  ENROLLResp=$(curl -fsSL -X POST "${SERVER}/v1/enroll/token" \
    -H "Content-Type: application/json" \
    -d "{\"install_token\":\"$TOKEN\",\"os_type\":\"$OS\",\"hostname\":\"$(hostname 2>/dev/null || echo unknown)\"}" 2>/dev/null) || {
    echo "Error: Enrollment failed"
    exit 1
  }
  echo "Device enrolled successfully via API"
}

echo "$ENROLLResp"

echo "Starting AINMS agent service..."
if [ "$OS" = "linux" ]; then
  if command -v systemctl &>/dev/null; then
    cat > /etc/systemd/system/ainms-agent.service <<EOF
[Unit]
Description=AINMS Agent
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/${BINARY_NAME} start
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
    systemctl daemon-reload
    systemctl enable ainms-agent
    systemctl start ainms-agent
    echo "AINMS agent service started via systemd"
  else
    nohup ${INSTALL_DIR}/${BINARY_NAME} start &>/var/log/ainms-agent.log &
    echo "AINMS agent started in background"
  fi
elif [ "$OS" = "macos" ]; then
  if command -v launchctl &>/dev/null; then
    cat > /Library/LaunchDaemons/com.ainms.agent.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.ainms.agent</string>
  <key>ProgramArguments</key>
  <array>
    <string>${INSTALL_DIR}/${BINARY_NAME}</string>
    <string>start</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict>
</plist>
EOF
    launchctl load /Library/LaunchDaemons/com.ainms.agent.plist
    echo "AINMS agent service started via launchd"
  else
    nohup ${INSTALL_DIR}/${BINARY_NAME} start &>/tmp/ainms-agent.log &
    echo "AINMS agent started in background"
  fi
fi

echo "=== Installation Complete ==="
`

	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/x-shellscript; charset=utf-8")
		w.Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(script))
	}
}