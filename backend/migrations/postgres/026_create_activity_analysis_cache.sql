-- Cache for AI activity analysis results to avoid repeated Ollama calls
CREATE TABLE IF NOT EXISTS activity_analysis_cache (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    employee_id UUID NOT NULL REFERENCES employees(id) ON DELETE CASCADE,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    employee_name VARCHAR(255) NOT NULL DEFAULT '',
    project_name VARCHAR(255),
    working_apps JSONB NOT NULL DEFAULT '[]',
    total_duration_sec DOUBLE PRECISION NOT NULL DEFAULT 0,
    productive_duration_sec DOUBLE PRECISION NOT NULL DEFAULT 0,
    unproductive_duration_sec DOUBLE PRECISION NOT NULL DEFAULT 0,
    neutral_duration_sec DOUBLE PRECISION NOT NULL DEFAULT 0,
    productive_pct DOUBLE PRECISION NOT NULL DEFAULT 0,
    unproductive_pct DOUBLE PRECISION NOT NULL DEFAULT 0,
    neutral_pct DOUBLE PRECISION NOT NULL DEFAULT 0,
    events JSONB NOT NULL DEFAULT '[]',
    ai_model VARCHAR(255) NOT NULL DEFAULT '',
    analyzed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Fast lookup: find cached analysis for an employee+time range
CREATE UNIQUE INDEX idx_activity_cache_employee_range ON activity_analysis_cache (employee_id, start_time, end_time);
CREATE INDEX idx_activity_cache_employee ON activity_analysis_cache (employee_id);