CREATE TABLE alert_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    role_id         UUID NOT NULL,
    category        VARCHAR(50) NOT NULL,
    threshold_min   INTEGER NOT NULL,
    popup_type      VARCHAR(20) NOT NULL DEFAULT 'toast',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alert_rules_role ON alert_rules(role_id);