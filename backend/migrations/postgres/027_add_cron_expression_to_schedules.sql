-- Add cron_expression column to targeted_screenshot_schedules
-- When cron_expression is set, it overrides the interval_minutes-based scheduling
-- and the scheduler evaluates the cron expression to determine trigger times.
-- Cron format: standard 5-field (minute hour day-of-month month day-of-week)
ALTER TABLE targeted_screenshot_schedules ADD COLUMN IF NOT EXISTS cron_expression TEXT;

-- Index for querying schedules with cron expressions
CREATE INDEX IF NOT EXISTS idx_targeted_schedules_cron ON targeted_screenshot_schedules(cron_expression) WHERE cron_expression IS NOT NULL;