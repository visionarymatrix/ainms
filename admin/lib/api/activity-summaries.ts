import { api } from "./client";

export interface ActivitySummary {
  id: string;
  device_id: string;
  window_start: string;
  window_end: string;
  summary_text: string;
  top_apps: string[];
  screenshot_count: number;
  created_at: string;
}

export interface ActivitySummaryFilters {
  limit?: number;
  from?: string;
  to?: string;
}

export async function listActivitySummaries(
  deviceId: string,
  filters?: ActivitySummaryFilters
): Promise<ActivitySummary[]> {
  const params: Record<string, string> = {};
  if (filters?.limit) params.limit = String(filters.limit);
  if (filters?.from) params.from = filters.from;
  if (filters?.to) params.to = filters.to;
  return api.get<ActivitySummary[]>(
    `/v1/devices/${deviceId}/activity-summaries`,
    Object.keys(params).length > 0 ? params : undefined
  );
}