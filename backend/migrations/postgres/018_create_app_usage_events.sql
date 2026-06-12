CREATE TABLE IF NOT EXISTS app_usage_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID NOT NULL,
    app_name TEXT NOT NULL,
    window_title TEXT NOT NULL DEFAULT '',
    process_name TEXT NOT NULL DEFAULT '',
    process_id INTEGER NOT NULL DEFAULT 0,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    duration_sec DOUBLE PRECISION NOT NULL DEFAULT 0,
    classification TEXT NOT NULL DEFAULT 'neutral',
    confidence DOUBLE PRECISION NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_app_usage_events_device ON app_usage_events(device_id);
CREATE INDEX IF NOT EXISTS idx_app_usage_events_created ON app_usage_events(created_at);
CREATE INDEX IF NOT EXISTS idx_app_usage_events_app ON app_usage_events(app_name);

CREATE TABLE IF NOT EXISTS app_usage_summaries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID NOT NULL,
    app_name TEXT NOT NULL,
    total_duration_sec DOUBLE PRECISION NOT NULL DEFAULT 0,
    session_count INTEGER NOT NULL DEFAULT 0,
    productive_duration_sec DOUBLE PRECISION NOT NULL DEFAULT 0,
    unproductive_duration_sec DOUBLE PRECISION NOT NULL DEFAULT 0,
    neutral_duration_sec DOUBLE PRECISION NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(device_id, app_name)
);
CREATE INDEX IF NOT EXISTS idx_app_usage_summaries_device ON app_usage_summaries(device_id);

CREATE TABLE IF NOT EXISTS priority_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    app_name TEXT NOT NULL DEFAULT '',
    window_title TEXT NOT NULL DEFAULT '',
    classification TEXT NOT NULL DEFAULT '',
    confidence DOUBLE PRECISION NOT NULL DEFAULT 0,
    event_time TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_priority_events_device ON priority_events(device_id);
CREATE INDEX IF NOT EXISTS idx_priority_events_created ON priority_events(created_at);

CREATE TABLE IF NOT EXISTS popup_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID NOT NULL,
    app_name TEXT NOT NULL DEFAULT '',
    window_title TEXT NOT NULL DEFAULT '',
    explanation TEXT NOT NULL DEFAULT '',
    popup_type TEXT NOT NULL DEFAULT '',
    classification TEXT NOT NULL DEFAULT '',
    confidence DOUBLE PRECISION NOT NULL DEFAULT 0,
    event_time TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_popup_events_device ON popup_events(device_id);