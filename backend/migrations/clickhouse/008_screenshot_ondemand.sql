CREATE TABLE IF NOT EXISTS ainms.screenshot_ondemand (
    request_id      UUID,
    device_id       UUID,
    requested_by    UUID,
    category        String,
    confidence      Float64,
    reason          String,
    status          String,
    created_at      DateTime64(3) DEFAULT now()
)
ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (device_id, created_at)
TTL created_at + INTERVAL 90 DAY;