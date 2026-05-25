package handler

import (
	"net/http"
	"os"
	"path/filepath"
	"strconv"
)

func AgentDownload() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		osType := r.URL.Query().Get("os")
		arch := r.URL.Query().Get("arch")

		if osType == "" || arch == "" {
			w.Header().Set("Content-Type", "text/plain; charset=utf-8")
			w.WriteHeader(http.StatusBadRequest)
			w.Write([]byte("Error: 'os' and 'arch' query parameters are required\n"))
			return
		}

		filename := ""
		switch osType {
		case "windows":
			filename = "ainms-agent_windows_amd64.exe"
		case "linux":
			filename = "ainms-agent_linux_amd64"
		case "macos":
			filename = "ainms-agent_macos_amd64"
		default:
			w.Header().Set("Content-Type", "text/plain; charset=utf-8")
			w.WriteHeader(http.StatusBadRequest)
			w.Write([]byte("Error: unsupported os type. Supported: windows, linux, macos\n"))
			return
		}

		binaryPath := filepath.Join("public", "agents", filename)
		stat, err := os.Stat(binaryPath)
		if err != nil {
			w.Header().Set("Content-Type", "text/plain; charset=utf-8")
			w.WriteHeader(http.StatusNotFound)
			w.Write([]byte("Error: agent binary not found for " + osType + "/" + arch + "\n"))
			return
		}

		w.Header().Set("Content-Type", "application/octet-stream")
		w.Header().Set("Content-Length", strconv.FormatInt(stat.Size(), 10))
		w.Header().Set("Content-Disposition", "attachment; filename=\""+filename+"\"")
		w.Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
		http.ServeFile(w, r, binaryPath)
	}
}
