CREATE TABLE devices (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    employee_id     UUID NOT NULL REFERENCES employees(id) ON DELETE CASCADE,
    hostname        VARCHAR(255),
    os_type         VARCHAR(20) NOT NULL,
    os_version      VARCHAR(50),
    agent_version   VARCHAR(20),
    mtls_cert_sn    VARCHAR(255) UNIQUE,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    last_heartbeat  TIMESTAMPTZ,
    enrolled_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_devices_employee ON devices(employee_id);
CREATE INDEX idx_devices_status ON devices(status);