import { api } from "./client";

export interface PopupAnswer {
  id: string;
  device_id: string;
  alert_id?: string;
  decision: string;
  app_name: string;
  window_title: string;
  explanation: string;
  popup_type: string;
  classification: string;
  confidence: number;
  event_time: string;
  created_at: string;
}

export interface ComplianceAlert {
  id: string;
  device_id: string;
  employee_id: string;
  screenshot_id?: string;
  decision: string;
  message: string;
  model_used: string;
  raw_response?: string;
  status: string;
  created_at: string;
  delivered_at?: string;
  acked_at?: string;
  employee_name?: string;
  device_hostname?: string;
  popup_answer?: PopupAnswer;
}

export async function listCompanyAlerts(
  companyId: string,
  status?: string,
): Promise<ComplianceAlert[]> {
  const params: Record<string, string> = {};
  if (status) params.status = status;
  return api.get<ComplianceAlert[]>(
    `/v1/companies/${companyId}/alerts`,
    params,
  );
}

export async function listEmployeeAlerts(
  employeeId: string,
  status?: string,
): Promise<ComplianceAlert[]> {
  const params: Record<string, string> = {};
  if (status) params.status = status;
  return api.get<ComplianceAlert[]>(
    `/v1/employees/${employeeId}/alerts`,
    params,
  );
}

export async function ackAlert(alertId: string): Promise<void> {
  await api.post(`/v1/alerts/${alertId}/ack`, {});
}