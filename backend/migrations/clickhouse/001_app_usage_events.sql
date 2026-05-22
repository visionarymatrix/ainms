CREATE TABLE IF NOT EXISTS ainms.app_usage_events (
    device_id       UUID,
    employee_id     UUID,
    app_name        String,
    window_title    String,
    process_name    String,
    process_id      UInt32,
    start_time      DateTime64(3),
    end_time        DateTime64(3),
    duration_sec    Float64,
    classification  String,
    confidence      Float64,
    role_id         Nullable(UUID),
    created_at      DateTime64(3) DEFAULT now()
)
ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(start_time)
ORDER BY (device_id, start_time)
TTL created_at + INTERVAL 12 MONTH;