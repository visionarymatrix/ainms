CREATE TABLE screenshot_requests (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id       UUID NOT NULL REFERENCES devices(id),
    requested_by    UUID NOT NULL,
    reason          TEXT NOT NULL,
    policy          VARCHAR(20) NOT NULL DEFAULT 'metadata_only',
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX idx_screenshot_requests_device ON screenshot_requests(device_id);
CREATE INDEX idx_screenshot_requests_status ON screenshot_requests(status);