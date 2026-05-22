CREATE TABLE app_classifications (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    role_id     UUID NOT NULL,
    app_name    VARCHAR(255) NOT NULL,
    category    VARCHAR(50) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_app_classifications_role ON app_classifications(role_id);