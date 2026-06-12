CREATE TABLE IF NOT EXISTS activity_summaries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID NOT NULL REFERENCES devices(id),
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,
    summary_text TEXT NOT NULL,
    top_apps TEXT[] NOT NULL DEFAULT '{}',
    screenshot_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_activity_summaries_device ON activity_summaries(device_id);
CREATE INDEX IF NOT EXISTS idx_activity_summaries_created ON activity_summaries(created_at);