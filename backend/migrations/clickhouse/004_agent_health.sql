CREATE TABLE IF NOT EXISTS ainms.agent_health (
    device_id   UUID,
    agent_version String,
    os_type      String,
    cpu_percent  Float64,
    memory_mb    Float64,
    created_at   DateTime64(3) DEFAULT now()
)
ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (device_id, created_at)
TTL created_at + INTERVAL 90 DAY;