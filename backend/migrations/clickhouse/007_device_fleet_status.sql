CREATE TABLE IF NOT EXISTS ainms.device_fleet_status (
    device_id       UUID,
    status          String,
    last_heartbeat  DateTime64(3),
    created_at      DateTime64(3) DEFAULT now()
)
ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (device_id, created_at)
TTL created_at + INTERVAL 90 DAY;