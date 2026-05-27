package handler

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/go-chi/chi/v5"
)

// TestCreateRole_InvalidCompanyID tests that an invalid company UUID returns 400
// before the service layer is called (nil service is safe for this path).
func TestCreateRole_InvalidCompanyID(t *testing.T) {
	r := chi.NewRouter()
	r.Post("/v1/companies/{companyID}/roles", CreateRole(nil))

	body, _ := json.Marshal(map[string]string{"name": "Engineer"})
	req := httptest.NewRequest(http.MethodPost, "/v1/companies/not-a-uuid/roles", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// TestCreateRole_InvalidJSON tests that a non-JSON body returns 400.
func TestCreateRole_InvalidJSON(t *testing.T) {
	r := chi.NewRouter()
	r.Post("/v1/companies/{companyID}/roles", CreateRole(nil))

	req := httptest.NewRequest(http.MethodPost, "/v1/companies/550e8400-e29b-41d4-a716-446655440000/roles", bytes.NewReader([]byte("not-json")))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err == nil {
		if errMsg, ok := resp["error"].(string); ok {
			if errMsg != "invalid request body" {
				t.Errorf("expected error 'invalid request body', got %q", errMsg)
			}
		}
	}
}

// TestGetRole_InvalidRoleID tests that an invalid role UUID returns 400.
func TestGetRole_InvalidRoleID(t *testing.T) {
	r := chi.NewRouter()
	r.Get("/v1/roles/{roleID}", GetRole(nil))

	req := httptest.NewRequest(http.MethodGet, "/v1/roles/not-a-uuid", nil)
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// TestListRoles_InvalidCompanyID tests that an invalid company UUID returns 400.
func TestListRoles_InvalidCompanyID(t *testing.T) {
	r := chi.NewRouter()
	r.Get("/v1/companies/{companyID}/roles", ListRoles(nil))

	req := httptest.NewRequest(http.MethodGet, "/v1/companies/not-a-uuid/roles", nil)
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// TestUpdateRole_InvalidRoleID tests that an invalid role UUID returns 400.
func TestUpdateRole_InvalidRoleID(t *testing.T) {
	r := chi.NewRouter()
	r.Put("/v1/roles/{roleID}", UpdateRole(nil))

	body, _ := json.Marshal(map[string]string{"name": "Updated"})
	req := httptest.NewRequest(http.MethodPut, "/v1/roles/not-a-uuid", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// TestUpdateRole_InvalidJSON tests that invalid JSON body returns 400.
func TestUpdateRole_InvalidJSON(t *testing.T) {
	r := chi.NewRouter()
	r.Put("/v1/roles/{roleID}", UpdateRole(nil))

	req := httptest.NewRequest(http.MethodPut, "/v1/roles/550e8400-e29b-41d4-a716-446655440000", bytes.NewReader([]byte("{bad")))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// TestDeleteRole_InvalidRoleID tests that an invalid role UUID returns 400.
func TestDeleteRole_InvalidRoleID(t *testing.T) {
	r := chi.NewRouter()
	r.Delete("/v1/roles/{roleID}", DeleteRole(nil))

	req := httptest.NewRequest(http.MethodDelete, "/v1/roles/not-a-uuid", nil)
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// ── AppClassification handler validation tests ──

// TestCreateAppClassification_InvalidRoleID tests that an invalid role UUID returns 400.
func TestCreateAppClassification_InvalidRoleID(t *testing.T) {
	r := chi.NewRouter()
	r.Post("/v1/roles/{roleID}/app-classifications", CreateAppClassification(nil))

	body, _ := json.Marshal(map[string]string{"app_name": "slack", "category": "productive"})
	req := httptest.NewRequest(http.MethodPost, "/v1/roles/not-a-uuid/app-classifications", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// TestCreateAppClassification_EmptyAppName tests that empty app_name returns 400.
// This validation runs before the repo call, so nil repo is safe.
func TestCreateAppClassification_EmptyAppName(t *testing.T) {
	r := chi.NewRouter()
	r.Post("/v1/roles/{roleID}/app-classifications", CreateAppClassification(nil))

	body, _ := json.Marshal(map[string]string{"app_name": "", "category": "productive"})
	req := httptest.NewRequest(http.MethodPost, "/v1/roles/550e8400-e29b-41d4-a716-446655440000/app-classifications", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err == nil {
		if errMsg, ok := resp["error"].(string); ok {
			if errMsg != "app_name is required" {
				t.Errorf("expected error 'app_name is required', got %q", errMsg)
			}
		}
	}
}

// TestCreateAppClassification_EmptyCategory tests that empty category returns 400.
func TestCreateAppClassification_EmptyCategory(t *testing.T) {
	r := chi.NewRouter()
	r.Post("/v1/roles/{roleID}/app-classifications", CreateAppClassification(nil))

	body, _ := json.Marshal(map[string]string{"app_name": "slack", "category": ""})
	req := httptest.NewRequest(http.MethodPost, "/v1/roles/550e8400-e29b-41d4-a716-446655440000/app-classifications", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err == nil {
		if errMsg, ok := resp["error"].(string); ok {
			if errMsg != "category is required" {
				t.Errorf("expected error 'category is required', got %q", errMsg)
			}
		}
	}
}

// TestCreateAppClassification_InvalidJSON tests that invalid JSON body returns 400.
func TestCreateAppClassification_InvalidJSON(t *testing.T) {
	r := chi.NewRouter()
	r.Post("/v1/roles/{roleID}/app-classifications", CreateAppClassification(nil))

	req := httptest.NewRequest(http.MethodPost, "/v1/roles/550e8400-e29b-41d4-a716-446655440000/app-classifications", bytes.NewReader([]byte("not-json")))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// TestDeleteAppClassification_InvalidID tests that an invalid classification UUID returns 400.
func TestDeleteAppClassification_InvalidID(t *testing.T) {
	r := chi.NewRouter()
	r.Delete("/v1/app-classifications/{classificationID}", DeleteAppClassification(nil))

	req := httptest.NewRequest(http.MethodDelete, "/v1/app-classifications/not-a-uuid", nil)
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// ── AlertRule handler validation tests ──

// TestCreateAlertRule_InvalidRoleID tests that an invalid role UUID returns 400.
func TestCreateAlertRule_InvalidRoleID(t *testing.T) {
	r := chi.NewRouter()
	r.Post("/v1/roles/{roleID}/alert-rules", CreateAlertRule(nil))

	body, _ := json.Marshal(map[string]interface{}{"category": "unproductive", "threshold_min": 10, "popup_type": "toast"})
	req := httptest.NewRequest(http.MethodPost, "/v1/roles/not-a-uuid/alert-rules", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// TestCreateAlertRule_EmptyCategory tests that empty category returns 400.
func TestCreateAlertRule_EmptyCategory(t *testing.T) {
	r := chi.NewRouter()
	r.Post("/v1/roles/{roleID}/alert-rules", CreateAlertRule(nil))

	body, _ := json.Marshal(map[string]interface{}{"category": "", "threshold_min": 10, "popup_type": "toast"})
	req := httptest.NewRequest(http.MethodPost, "/v1/roles/550e8400-e29b-41d4-a716-446655440000/alert-rules", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err == nil {
		if errMsg, ok := resp["error"].(string); ok {
			if errMsg != "category is required" {
				t.Errorf("expected error 'category is required', got %q", errMsg)
			}
		}
	}
}

// TestCreateAlertRule_InvalidJSON tests that invalid JSON body returns 400.
func TestCreateAlertRule_InvalidJSON(t *testing.T) {
	r := chi.NewRouter()
	r.Post("/v1/roles/{roleID}/alert-rules", CreateAlertRule(nil))

	req := httptest.NewRequest(http.MethodPost, "/v1/roles/550e8400-e29b-41d4-a716-446655440000/alert-rules", bytes.NewReader([]byte("{bad")))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// TestDeleteAlertRule_InvalidRuleID tests that an invalid rule UUID returns 400.
func TestDeleteAlertRule_InvalidRuleID(t *testing.T) {
	r := chi.NewRouter()
	r.Delete("/v1/alert-rules/{ruleID}", DeleteAlertRule(nil))

	req := httptest.NewRequest(http.MethodDelete, "/v1/alert-rules/not-a-uuid", nil)
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// TestListAppClassifications_InvalidRoleID tests that an invalid role UUID returns 400.
func TestListAppClassifications_InvalidRoleID(t *testing.T) {
	r := chi.NewRouter()
	r.Get("/v1/roles/{roleID}/app-classifications", ListAppClassifications(nil))

	req := httptest.NewRequest(http.MethodGet, "/v1/roles/not-a-uuid/app-classifications", nil)
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}

// TestListAlertRules_InvalidRoleID tests that an invalid role UUID returns 400.
func TestListAlertRules_InvalidRoleID(t *testing.T) {
	r := chi.NewRouter()
	r.Get("/v1/roles/{roleID}/alert-rules", ListAlertRules(nil))

	req := httptest.NewRequest(http.MethodGet, "/v1/roles/not-a-uuid/alert-rules", nil)
	w := httptest.NewRecorder()

	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status %d, got %d; body: %s", http.StatusBadRequest, w.Code, w.Body.String())
	}
}