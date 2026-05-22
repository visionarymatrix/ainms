CREATE TABLE IF NOT EXISTS ainms.popup_events (
    device_id       UUID,
    employee_id    UUID,
    app_name        String,
    window_title    String,
    explanation     String,
    popup_type     String,
    classification String,
    confidence      Float64,
    created_at      DateTime64(3) DEFAULT now()
)
ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (employee_id, created_at)
TTL created_at + INTERVAL 12 MONTH;