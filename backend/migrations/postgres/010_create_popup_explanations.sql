CREATE TABLE popup_explanations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id       UUID NOT NULL REFERENCES devices(id),
    employee_id     UUID NOT NULL REFERENCES employees(id),
    app_name        VARCHAR(255) NOT NULL,
    window_title    VARCHAR(255),
    explanation     TEXT NOT NULL,
    popup_type      VARCHAR(20) NOT NULL,
    classification  VARCHAR(50),
    confidence      DOUBLE PRECISION,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_popup_explanations_employee ON popup_explanations(employee_id);
CREATE INDEX idx_popup_explanations_created ON popup_explanations(created_at);