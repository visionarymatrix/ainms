package handler

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/go-chi/chi/v5"
)

func TestPostNLQuery_BadRequests(t *testing.T) {
	tests := []struct {
		name       string
		employeeID string
		body       interface{}
		wantCode   int
		wantErrSub string // substring expected in the error response
	}{
		{
			name:       "invalid employee ID",
			employeeID: "not-a-uuid",
			body:       map[string]string{"query": "what is user doing"},
			wantCode:   http.StatusBadRequest,
			wantErrSub: "invalid employee_id",
		},
		{
			name:       "empty query",
			employeeID: "550e8400-e29b-41d4-a716-446655440000",
			body:       map[string]string{"query": ""},
			wantCode:   http.StatusBadRequest,
			wantErrSub: "query is required",
		},
		{
			name:       "missing query field",
			employeeID: "550e8400-e29b-41d4-a716-446655440000",
			body:       map[string]string{},
			wantCode:   http.StatusBadRequest,
			wantErrSub: "query is required",
		},
		{
			name:       "invalid JSON body",
			employeeID: "550e8400-e29b-41d4-a716-446655440000",
			body:       "not-json",
			wantCode:   http.StatusBadRequest,
			wantErrSub: "invalid request body",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := chi.NewRouter()
			r.Post("/v1/employees/{employeeID}/nl-query", PostNLQuery(nil, nil, nil))

			var bodyBytes []byte
			if s, ok := tt.body.(string); ok {
				bodyBytes = []byte(s)
			} else {
				bodyBytes, _ = json.Marshal(tt.body)
			}

			req := httptest.NewRequest(http.MethodPost, "/v1/employees/"+tt.employeeID+"/nl-query", bytes.NewReader(bodyBytes))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()

			r.ServeHTTP(w, req)

			if w.Code != tt.wantCode {
				t.Errorf("expected status %d, got %d; body: %s", tt.wantCode, w.Code, w.Body.String())
			}

			var resp map[string]interface{}
			if err := json.Unmarshal(w.Body.Bytes(), &resp); err == nil {
				if errMsg, ok := resp["error"].(string); ok {
					if tt.wantErrSub != "" && !containsSubstring(errMsg, tt.wantErrSub) {
						t.Errorf("expected error to contain %q, got %q", tt.wantErrSub, errMsg)
					}
				}
			}
		})
	}
}

func containsSubstring(s, sub string) bool {
	return len(s) >= len(sub) && (s == sub || len(sub) == 0 || searchSubstring(s, sub))
}

func searchSubstring(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}