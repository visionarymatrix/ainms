CREATE TABLE IF NOT EXISTS ainms.tamper_events (
    device_id       UUID,
    employee_id    UUID,
    event_type      String,
    details         String,
    created_at      DateTime64(3) DEFAULT now()
)
ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (device_id, created_at)
TTL created_at + INTERVAL 24 MONTH;