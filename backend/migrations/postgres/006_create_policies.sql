CREATE TABLE policies (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID NOT NULL REFERENCES tenants(id),
    upload_interval    INTEGER NOT NULL DEFAULT 300,
    screenshot_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    screenshot_policy  VARCHAR(20) NOT NULL DEFAULT 'metadata_only',
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_policies_tenant ON policies(tenant_id);