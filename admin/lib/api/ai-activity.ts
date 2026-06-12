import { api } from "./client";

export interface AIActivityEvent {
  app_name: string;
  window_title: string;
  duration_sec: number;
  classification: "productive" | "unproductive" | "neutral";
  reason: string;
}

export interface AIActivityAnalysis {
  employee_id: string;
  employee_name: string;
  project_name?: string;
  working_apps: string[];
  total_duration_sec: number;
  productive_duration_sec: number;
  unproductive_duration_sec: number;
  neutral_duration_sec: number;
  productive_pct: number;
  unproductive_pct: number;
  neutral_pct: number;
  events: AIActivityEvent[];
  ai_model: string;
  analyzed_at: string;
}

export async function getAIActivityAnalysis(
  employeeId: string,
  params?: {
    from?: string;
    to?: string;
  }
): Promise<AIActivityAnalysis> {
  const queryParams: Record<string, string> = {};
  if (params?.from) queryParams["start_time"] = params.from;
  if (params?.to) queryParams["end_time"] = params.to;
  return api.get<AIActivityAnalysis>(`/v1/employees/${employeeId}/ai-activity`, queryParams);
}