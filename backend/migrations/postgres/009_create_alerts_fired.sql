CREATE TABLE alerts_fired (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id       UUID NOT NULL REFERENCES devices(id),
    alert_rule_id   UUID NOT NULL REFERENCES alert_rules(id),
    employee_id    UUID NOT NULL REFERENCES employees(id),
    app_name        VARCHAR(255) NOT NULL,
    popup_type      VARCHAR(20) NOT NULL,
    explanation     TEXT,
    fired_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alerts_fired_device ON alerts_fired(device_id);
CREATE INDEX idx_alerts_fired_employee ON alerts_fired(employee_id);
CREATE INDEX idx_alerts_fired_fired_at ON alerts_fired(fired_at);