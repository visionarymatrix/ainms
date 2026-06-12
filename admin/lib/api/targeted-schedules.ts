import { api } from "./client";

export interface TargetedScreenshotSchedule {
  id: string;
  company_id: string;
  employee_id: string;
  created_by: string;
  name: string;
  interval_minutes: number;
  cron_expression: string | null;
  start_time: string;
  end_time: string;
  start_date: string;
  end_date: string;
  status: string;
  last_triggered_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface TargetedScreenshot {
  id: string;
  device_id: string;
  requested_by: string;
  reason: string;
  policy: string;
  status: string;
  image_path: string | null;
  schedule_id: string | null;
  created_at: string;
  completed_at: string | null;
}

export async function createTargetedSchedule(data: {
  company_id?: string;
  employee_id: string;
  name?: string;
  interval_minutes: number;
  cron_expression?: string;
  start_time?: string;
  end_time?: string;
  start_date?: string;
  end_date?: string;
}): Promise<TargetedScreenshotSchedule> {
  return api.post<TargetedScreenshotSchedule>("/v1/targeted-schedules", data);
}

export async function listTargetedSchedules(companyId?: string): Promise<TargetedScreenshotSchedule[]> {
  const params: Record<string, string> = {};
  if (companyId) params.company_id = companyId;
  return api.get<TargetedScreenshotSchedule[]>("/v1/targeted-schedules", params);
}

export async function listTargetedSchedulesByEmployee(employeeId: string): Promise<TargetedScreenshotSchedule[]> {
  return api.get<TargetedScreenshotSchedule[]>(`/v1/employees/${employeeId}/targeted-schedules`);
}

export async function getTargetedSchedule(scheduleId: string): Promise<TargetedScreenshotSchedule> {
  return api.get<TargetedScreenshotSchedule>(`/v1/targeted-schedules/${scheduleId}`);
}

export async function updateTargetedSchedule(
  scheduleId: string,
  data: Partial<{
    name: string;
    interval_minutes: number;
    cron_expression: string;
    start_time: string;
    end_time: string;
    start_date: string;
    end_date: string;
    status: string;
  }>
): Promise<TargetedScreenshotSchedule> {
  return api.put<TargetedScreenshotSchedule>(`/v1/targeted-schedules/${scheduleId}`, data);
}

export async function deleteTargetedSchedule(scheduleId: string): Promise<void> {
  return api.delete(`/v1/targeted-schedules/${scheduleId}`);
}

export async function getTargetedScreenshots(params?: {
  company_id?: string;
  schedule_id?: string;
  employee_id?: string;
  from?: string;
  to?: string;
}): Promise<TargetedScreenshot[]> {
  return api.get<TargetedScreenshot[]>("/v1/targeted-screenshots", params);
}

export async function getScheduleScreenshots(scheduleId: string): Promise<TargetedScreenshot[]> {
  return api.get<TargetedScreenshot[]>(`/v1/targeted-schedules/${scheduleId}/screenshots`);
}