ALTER TABLE app_usage_summaries ADD COLUMN IF NOT EXISTS date DATE NOT NULL DEFAULT CURRENT_DATE;

DROP INDEX IF EXISTS app_usage_summaries_device_id_app_name_key;

ALTER TABLE app_usage_summaries DROP CONSTRAINT IF EXISTS app_usage_summaries_pkey;
ALTER TABLE app_usage_summaries DROP CONSTRAINT IF EXISTS app_usage_summaries_device_id_app_name_key;

ALTER TABLE app_usage_summaries ADD CONSTRAINT app_usage_summaries_pkey PRIMARY KEY (device_id, app_name, date);

CREATE INDEX IF NOT EXISTS idx_app_usage_summaries_date ON app_usage_summaries(date);