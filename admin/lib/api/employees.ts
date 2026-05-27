import { api } from "./client";

export interface Employee {
  id: string;
  company_id: string;
  employee_id: string;
  first_name: string;
  last_name: string;
  email: string | null;
  role_id: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface Device {
  id: string;
  employee_id: string;
  hostname: string | null;
  os_type: string;
  os_version: string | null;
  agent_version: string | null;
  status: string;
  connection_status: string;
  last_heartbeat: string | null;
  enrolled_at: string;
  fingerprint: string | null;
  created_at: string;
  updated_at: string;
}

export interface ScreenshotRequest {
  id: string;
  device_id: string;
  requested_by: string;
  reason: string;
  policy: string;
  status: string;
  image_path: string | null;
  created_at: string;
  completed_at: string | null;
}

export async function listEmployees(companyId: string): Promise<Employee[]> {
  return api.get<Employee[]>(`/v1/companies/${companyId}/employees`);
}

export async function getEmployee(id: string): Promise<Employee> {
  return api.get<Employee>(`/v1/employees/${id}`);
}

export async function getEmployeeDevices(employeeId: string): Promise<Device[]> {
  return api.get<Device[]>(`/v1/employees/${employeeId}/devices`);
}

export async function registerEmployee(
  companyId: string,
  data: { first_name: string; last_name: string; email?: string; role_id?: string },
): Promise<Employee> {
  return api.post<Employee>(`/v1/companies/${companyId}/employees`, data);
}

export async function deactivateEmployee(id: string): Promise<void> {
  return api.delete(`/v1/employees/${id}`);
}

export async function requestScreenshot(deviceId: string): Promise<ScreenshotRequest> {
  return api.post<ScreenshotRequest>('/v1/screenshot/request', {
    device_id: deviceId,
    reason: 'On-demand screenshot',
    policy: 'upload_image',
  });
}

export async function getDeviceScreenshots(deviceId: string): Promise<ScreenshotRequest[]> {
  return api.get<ScreenshotRequest[]>(`/v1/devices/${deviceId}/screenshots`);
}

export interface NLQueryResponse {
  query_id: string;
  employee_id: string;
  device_id?: string;
  status: string;
}

export interface AgentReport {
  query_id: string;
  device_id: string;
  query: string;
  timestamp: string;
  summary?: {
    total_events: number;
    classification_breakdown: Record<string, { count: number; duration_sec: number }>;
    top_apps: Array<{ app_name: string; duration_sec: number; classification: string }>;
    network_summary?: {
      total_connections: number;
      unique_domains: string[];
    };
    role?: {
      name: string;
      work_description: string;
    };
  };
  report_text: string;
}

export async function sendNLQuery(employeeId: string, query: string): Promise<NLQueryResponse> {
  return api.post<NLQueryResponse>(`/v1/employees/${employeeId}/nl-query`, { query });
}
