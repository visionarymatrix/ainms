package handler

import (
	"net/http"

	"github.com/ainms/gateway/internal/domain"
)

func GetLLMModels() http.HandlerFunc {
	models := []domain.LLMModel{
		{
			ID:          "llama-3.2-1b-instruct-q4_k_m",
			Name:        "Llama 3.2 1B Instruct (Q4_K_M)",
			Description: "Lightweight instruction-tuned Llama 3.2 model, quantised for efficient CPU/GPU inference via llama.cpp.",
			Provider:    "llama-cpp",
			Version:     "1.0.0",
			Files: []domain.LLMModelFile{
				{
					Filename:    "llama-3.2-1b-instruct-Q4_K_M.gguf",
					DownloadURL: "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf",
					FileSizeMB:  769,
					SHA256:      "",
				},
			},
			Parameters: domain.LLMModelParameters{
				NCtx:       2048,
				NThreads:   4,
				NGpuLayers: 99,
			},
		},
		{
			ID:          "llama-3.2-3b-instruct-q4_k_m",
			Name:        "Llama 3.2 3B Instruct (Q4_K_M)",
			Description: "Mid-size instruction-tuned Llama 3.2 model, quantised for balanced accuracy and speed via llama.cpp.",
			Provider:    "llama-cpp",
			Version:     "1.0.0",
			Files: []domain.LLMModelFile{
				{
					Filename:    "llama-3.2-3b-instruct-Q4_K_M.gguf",
					DownloadURL: "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf",
					FileSizeMB:  2012,
					SHA256:      "",
				},
			},
			Parameters: domain.LLMModelParameters{
				NCtx:       2048,
				NThreads:   4,
				NGpuLayers: 99,
			},
		},
		{
			ID:          "phi-3.5-mini-instruct-q4_k_m",
			Name:        "Phi-3.5 Mini Instruct (Q4_K_M)",
			Description: "Compact instruction-tuned Microsoft Phi-3.5 model, quantised for resource-constrained agent deployments.",
			Provider:    "llama-cpp",
			Version:     "1.0.0",
			Files: []domain.LLMModelFile{
				{
					Filename:    "phi-3.5-mini-instruct-Q4_K_M.gguf",
					DownloadURL: "https://huggingface.co/bartowski/Phi-3.5-mini-instruct-GGUF/resolve/main/Phi-3.5-mini-instruct-Q4_K_M.gguf",
					FileSizeMB:  2248,
					SHA256:      "",
				},
			},
			Parameters: domain.LLMModelParameters{
				NCtx:       2048,
				NThreads:   4,
				NGpuLayers: 99,
			},
		},
	}

	resp := domain.LLMModelsResponse{Models: models}

	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, resp)
	}
}