CREATE TABLE IF NOT EXISTS compliance_alerts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id       UUID NOT NULL REFERENCES devices(id),
    employee_id     UUID NOT NULL REFERENCES employees(id),
    screenshot_id   UUID REFERENCES screenshot_requests(id),
    decision        VARCHAR(20) NOT NULL CHECK (decision IN ('violation','shoutout','productive')),
    message         TEXT NOT NULL,
    model_used      VARCHAR(100) NOT NULL DEFAULT 'kimi-k2.6:cloud',
    raw_response    TEXT,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','delivered','acked')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at    TIMESTAMPTZ,
    acked_at        TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_compliance_alerts_device ON compliance_alerts(device_id);
CREATE INDEX IF NOT EXISTS idx_compliance_alerts_status ON compliance_alerts(status);
CREATE INDEX IF NOT EXISTS idx_compliance_alerts_created ON compliance_alerts(created_at);
