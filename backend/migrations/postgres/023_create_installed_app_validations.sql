CREATE TABLE IF NOT EXISTS installed_app_validations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    app_name TEXT NOT NULL,
    role_id UUID REFERENCES roles(id) ON DELETE SET NULL,
    display_name TEXT NOT NULL DEFAULT '',
    agent_category TEXT NOT NULL DEFAULT 'neutral',
    validated_category TEXT NOT NULL DEFAULT 'neutral',
    is_compliant BOOLEAN NOT NULL DEFAULT true,
    reason TEXT NOT NULL DEFAULT '',
    validated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(device_id, app_name)
);

CREATE INDEX idx_installed_app_validations_device_id ON installed_app_validations(device_id);
CREATE INDEX idx_installed_app_validations_is_compliant ON installed_app_validations(is_compliant);
CREATE INDEX idx_installed_app_validations_role_id ON installed_app_validations(role_id);