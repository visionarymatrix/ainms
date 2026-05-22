CREATE TABLE IF NOT EXISTS ainms.screenshot_metadata (
    device_id       UUID,
    category        String,
    confidence      Float64,
    uploaded        UInt8,
    created_at      DateTime64(3) DEFAULT now()
)
ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (device_id, created_at)
TTL created_at + INTERVAL 30 DAY;