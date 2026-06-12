-- Store installed applications per device, uploaded by agents.
-- Enables the admin portal to show what software is installed on each user's machine.
CREATE TABLE IF NOT EXISTS installed_apps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    app_name TEXT NOT NULL,
    display_name TEXT NOT NULL DEFAULT '',
    publisher TEXT NOT NULL DEFAULT '',
    install_path TEXT,
    category TEXT NOT NULL DEFAULT 'neutral',
    confidence DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    source TEXT NOT NULL DEFAULT 'unknown',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(device_id, app_name)
);

CREATE INDEX idx_installed_apps_device_id ON installed_apps(device_id);
CREATE INDEX idx_installed_apps_app_name ON installed_apps(app_name);