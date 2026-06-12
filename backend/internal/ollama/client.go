package ollama

import (
	"bufio"
	"bytes"
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

const (
	defaultCloudBaseURL = "https://ollama.com/api"
	defaultModel        = "gemma4:31b-cloud"
)

// Client is a custom HTTP client for Ollama Cloud API (not the official SDK,
// because the official SDK uses ED25519 key-signing for ollama.com rather
// than Bearer token auth).
type Client struct {
	baseURL    string
	apiKey     string
	model      string
	httpClient *http.Client
}

// NewClient creates an Ollama Cloud client.
func NewClient(baseURL, apiKey, model string, timeoutSec int) *Client {
	if baseURL == "" {
		baseURL = defaultCloudBaseURL
	}
	if model == "" {
		model = defaultModel
	}
	return &Client{
		baseURL: baseURL,
		apiKey:  apiKey,
		model:   model,
		httpClient: &http.Client{
			Timeout: time.Duration(timeoutSec) * time.Second,
		},
	}
}

// Message is a chat message for the Ollama /api/chat endpoint.
type Message struct {
	Role     string   `json:"role"`
	Content  string   `json:"content"`
	Thinking string   `json:"thinking,omitempty"`
	Images   []string `json:"images,omitempty"`
}

// ChatRequest is the payload for POST /api/chat.
type ChatRequest struct {
	Model    string    `json:"model"`
	Messages []Message `json:"messages"`
	Stream   bool      `json:"stream,omitempty"`
	Options  *Options  `json:"options,omitempty"`
}

// Options controls generation parameters.
type Options struct {
	Temperature float64 `json:"temperature,omitempty"`
	NumCtx      int     `json:"num_ctx,omitempty"`
	NumPredict  int     `json:"num_predict,omitempty"`
}

// ChatResponse is the assembled non-streaming response from /api/chat.
type ChatResponse struct {
	Model      string  `json:"model"`
	CreatedAt  string  `json:"created_at"`
	Message    Message `json:"message"`
	Done       bool    `json:"done"`
	DoneReason string  `json:"done_reason,omitempty"`
	// Usage metrics
	TotalDuration      int64 `json:"total_duration,omitempty"`
	LoadDuration       int64 `json:"load_duration,omitempty"`
	PromptEvalCount    int   `json:"prompt_eval_count,omitempty"`
	PromptEvalDuration int64 `json:"prompt_eval_duration,omitempty"`
	EvalCount          int   `json:"eval_count,omitempty"`
	EvalDuration       int64 `json:"eval_duration,omitempty"`
}

// streamChunk is a single NDJSON chunk from the streaming Ollama Cloud response.
// kimi-k2.6:cloud always streams NDJSON chunks even when stream:false is sent.
type streamChunk struct {
	Model      string  `json:"model"`
	CreatedAt  string  `json:"created_at"`
	Message    Message `json:"message"`
	Done       bool    `json:"done"`
	DoneReason string  `json:"done_reason,omitempty"`
	// Usage metrics (only on final chunk with done=true)
	TotalDuration      int64 `json:"total_duration,omitempty"`
	LoadDuration       int64 `json:"load_duration,omitempty"`
	PromptEvalCount    int   `json:"prompt_eval_count,omitempty"`
	PromptEvalDuration int64 `json:"prompt_eval_duration,omitempty"`
	EvalCount          int   `json:"eval_count,omitempty"`
	EvalDuration       int64 `json:"eval_duration,omitempty"`
}

func (c *Client) Model() string {
	return c.model
}

// Chat sends a chat request to Ollama Cloud and returns the response.
// Ollama Cloud may return either a single JSON object (true non-streaming)
// or NDJSON stream chunks (kimi-k2.6:cloud ignores stream:false).
// This method handles both formats.
func (c *Client) Chat(ctx context.Context, req *ChatRequest) (*ChatResponse, error) {
	if req.Stream {
		return nil, fmt.Errorf("streaming not supported: use ChatStream")
	}

	body, err := json.Marshal(req)
	if err != nil {
		return nil, fmt.Errorf("marshal request: %w", err)
	}

	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, c.baseURL+"/chat", bytes.NewReader(body))
	if err != nil {
		return nil, fmt.Errorf("create request: %w", err)
	}
	httpReq.Header.Set("Content-Type", "application/json")
	httpReq.Header.Set("Authorization", "Bearer "+c.apiKey)

	resp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return nil, fmt.Errorf("do request: %w", err)
	}
	defer resp.Body.Close()

	switch resp.StatusCode {
	case http.StatusUnauthorized:
		return nil, fmt.Errorf("unauthorized: check OLLAMA_API_KEY")
	case http.StatusPaymentRequired:
		return nil, fmt.Errorf("payment required: model needs Pro subscription")
	case http.StatusForbidden:
		return nil, fmt.Errorf("forbidden: model requires subscription")
	case http.StatusTooManyRequests:
		return nil, fmt.Errorf("rate limited: quota exceeded")
	}
	if resp.StatusCode != http.StatusOK {
		respBody, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("unexpected status %d: %s", resp.StatusCode, string(respBody))
	}

	return decodeChatResponse(resp.Body)
}

// decodeChatResponse handles both single-JSON and NDJSON streaming responses
// from Ollama Cloud. kimi-k2.6:cloud returns NDJSON chunks even with stream:false.
func decodeChatResponse(body io.Reader) (*ChatResponse, error) {
	// Peek at the first bytes to detect format.
	// Single JSON: starts with '{' and the entire body is one object.
	// NDJSON stream: multiple JSON objects separated by newlines.
	br := bufio.NewReader(body)
	firstLine, err := br.ReadBytes('\n')
	if err != nil && err != io.EOF {
		return nil, fmt.Errorf("read first line: %w", err)
	}
	firstLine = bytes.TrimSpace(firstLine)

	// Try decoding as a single complete JSON response first.
	var single ChatResponse
	if json.Unmarshal(firstLine, &single) == nil && single.Done {
		more, _ := br.Peek(1)
		if len(more) == 0 {
			return &single, nil
		}
	}

	// It's an NDJSON stream — accumulate all chunks.
	var contentBuf strings.Builder
	var thinkingBuf strings.Builder
	var lastChunk streamChunk

	// Process first line as a chunk.
	var firstChunk streamChunk
	if err := json.Unmarshal(firstLine, &firstChunk); err != nil {
		return nil, fmt.Errorf("decode first chunk: %w", err)
	}
	contentBuf.WriteString(firstChunk.Message.Content)
	thinkingBuf.WriteString(firstChunk.Message.Thinking)
	lastChunk = firstChunk

	// Read remaining lines.
	scanner := bufio.NewScanner(br)
	for scanner.Scan() {
		line := bytes.TrimSpace(scanner.Bytes())
		if len(line) == 0 {
			continue
		}
		var chunk streamChunk
		if err := json.Unmarshal(line, &chunk); err != nil {
			continue // skip malformed chunks
		}
		contentBuf.WriteString(chunk.Message.Content)
		thinkingBuf.WriteString(chunk.Message.Thinking)
		lastChunk = chunk
		if chunk.Done {
			break
		}
	}

	// Assemble the final ChatResponse from the accumulated content.
	return &ChatResponse{
		Model:      lastChunk.Model,
		CreatedAt:  lastChunk.CreatedAt,
		Message: Message{
			Role:     "assistant",
			Content:  contentBuf.String(),
			Thinking: thinkingBuf.String(),
		},
		Done:               lastChunk.Done,
		DoneReason:         lastChunk.DoneReason,
		TotalDuration:      lastChunk.TotalDuration,
		LoadDuration:       lastChunk.LoadDuration,
		PromptEvalCount:    lastChunk.PromptEvalCount,
		PromptEvalDuration: lastChunk.PromptEvalDuration,
		EvalCount:          lastChunk.EvalCount,
		EvalDuration:       lastChunk.EvalDuration,
	}, nil
}

// AnalyzeScreenshot sends a vision+text compliance analysis request.
// systemPrompt sets the monitoring instructions; userPrompt describes the context;
// imageData is the raw PNG/JPEG bytes (base64-encoded internally).
func (c *Client) AnalyzeScreenshot(ctx context.Context, systemPrompt, userPrompt string, imageData []byte) (*ChatResponse, error) {
	messages := []Message{}
	if systemPrompt != "" {
		messages = append(messages, Message{Role: "system", Content: systemPrompt})
	}
	messages = append(messages, Message{
		Role:    "user",
		Content: userPrompt,
		Images:  []string{base64.StdEncoding.EncodeToString(imageData)},
	})

	return c.Chat(ctx, &ChatRequest{
		Model:    c.model,
		Messages: messages,
		Stream:   false,
		Options: &Options{
			Temperature: 0.3,
			NumCtx:      8192,
			NumPredict:  512,
		},
	})
}

// AnalyzeScreenshots sends a multi-image vision+text compliance analysis request.
// All images are sent in a single chat message so the model can cross-reference them.
// imagesData is a slice of raw PNG/JPEG byte slices.
func (c *Client) AnalyzeScreenshots(ctx context.Context, systemPrompt, userPrompt string, imagesData [][]byte) (*ChatResponse, error) {
	images := make([]string, len(imagesData))
	for i, data := range imagesData {
		images[i] = base64.StdEncoding.EncodeToString(data)
	}

	messages := []Message{}
	if systemPrompt != "" {
		messages = append(messages, Message{Role: "system", Content: systemPrompt})
	}
	messages = append(messages, Message{
		Role:    "user",
		Content: userPrompt,
		Images:  images,
	})

	return c.Chat(ctx, &ChatRequest{
		Model:    c.model,
		Messages: messages,
		Stream:   false,
		Options: &Options{
			Temperature: 0.3,
			NumCtx:      16384,
			NumPredict:  1024,
		},
	})
}