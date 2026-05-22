CREATE TABLE IF NOT EXISTS ainms.app_usage_summary (
    device_id               UUID,
    employee_id             UUID,
    app_name                String,
    total_duration_sec      Float64,
    session_count           UInt32,
    productive_duration_sec Float64,
    unproductive_duration_sec Float64,
    neutral_duration_sec    Float64,
    summary_date            Date,
    created_at              DateTime64(3) DEFAULT now()
)
ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(summary_date)
ORDER BY (employee_id, app_name, summary_date)
TTL created_at + INTERVAL 24 MONTH;