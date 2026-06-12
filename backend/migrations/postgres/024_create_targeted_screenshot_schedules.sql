-- Targeted screenshot schedules: admin-defined separate screenshot windows for specific employees
CREATE TABLE IF NOT EXISTS targeted_screenshot_schedules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id      UUID NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    employee_id     UUID NOT NULL REFERENCES employees(id) ON DELETE CASCADE,
    created_by      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            TEXT NOT NULL DEFAULT '',
    interval_minutes INT NOT NULL DEFAULT 5 CHECK (interval_minutes >= 1),
    start_time      TIME NOT NULL DEFAULT '09:00:00',
    end_time        TIME NOT NULL DEFAULT '17:00:00',
    start_date      DATE NOT NULL DEFAULT CURRENT_DATE,
    end_date        DATE NOT NULL DEFAULT CURRENT_DATE + INTERVAL '30 days',
    status          VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'expired')),
    last_triggered_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_targeted_schedules_company ON targeted_screenshot_schedules(company_id);
CREATE INDEX idx_targeted_schedules_employee ON targeted_screenshot_schedules(employee_id);
CREATE INDEX idx_targeted_schedules_status ON targeted_screenshot_schedules(status);
CREATE INDEX idx_targeted_schedules_active ON targeted_screenshot_schedules(status, start_date, end_date) WHERE status = 'active';

-- Add schedule_id to screenshot_requests to link targeted screenshots back to their schedule
ALTER TABLE screenshot_requests ADD COLUMN IF NOT EXISTS schedule_id UUID REFERENCES targeted_screenshot_schedules(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_screenshot_requests_schedule ON screenshot_requests(schedule_id) WHERE schedule_id IS NOT NULL;