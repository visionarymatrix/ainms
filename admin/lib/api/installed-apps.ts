import { api } from "./client";

export interface InstalledAppValidation {
  id: string;
  device_id: string;
  app_name: string;
  role_id: string | null;
  display_name: string;
  agent_category: string;
  validated_category: string;
  is_compliant: boolean;
  reason: string;
  validated_at: string;
  created_at: string;
  updated_at: string;
}

export interface InstalledAppValidationWithDetails extends InstalledAppValidation {
  device_hostname: string;
  employee_id: string;
  employee_name: string;
  role_name: string;
}

export async function getDeviceValidations(deviceId: string): Promise<InstalledAppValidation[]> {
  return api.get<InstalledAppValidation[]>(`/v1/devices/${deviceId}/validations`);
}

export async function getEmployeeValidations(employeeId: string): Promise<InstalledAppValidationWithDetails[]> {
  return api.get<InstalledAppValidationWithDetails[]>(`/v1/employees/${employeeId}/validations`);
}

export async function getCompanyValidations(companyId: string): Promise<InstalledAppValidationWithDetails[]> {
  return api.get<InstalledAppValidationWithDetails[]>(`/v1/companies/${companyId}/validations`);
}

export async function getCompanyNonCompliantApps(companyId: string): Promise<InstalledAppValidationWithDetails[]> {
  return api.get<InstalledAppValidationWithDetails[]>(`/v1/companies/${companyId}/non-compliant-apps`);
}