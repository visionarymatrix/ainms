CREATE TABLE pending_commands (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id   UUID NOT NULL REFERENCES devices(id),
    command_type VARCHAR(50) NOT NULL,
    payload     JSONB NOT NULL DEFAULT '{}',
    status       VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at      TIMESTAMPTZ,
    acked_at     TIMESTAMPTZ
);

CREATE INDEX idx_pending_commands_device ON pending_commands(device_id);
CREATE INDEX idx_pending_commands_status ON pending_commands(status);