import { api } from "./client";

export interface InstallToken {
  id: string;
  token: string;
  install_cmd: string;
  employee_id: string;
  company_id: string;
  description: string;
  expires_at: string | null;
  created_at: string;
  used_at: string | null;
  revoked_at: string | null;
}

export interface EmployeeInstallToken {
  id: string;
  token: string;
  install_cmd: string;
  windows_cmd: string;
  employee_id: string;
  company_id: string;
  description: string;
  expires_at: string | null;
  created_at: string;
  revoked_at: string | null;
}

export async function getEmployeeInstallToken(employeeId: string): Promise<EmployeeInstallToken> {
  return api.post<EmployeeInstallToken>(`/v1/employees/${employeeId}/install-token`, {});
}

export interface CreateInstallTokenRequest {
  employee_id: string;
  company_id: string;
  description?: string;
  expires_in?: string;
}

export async function generateInstallToken(req: CreateInstallTokenRequest): Promise<InstallToken> {
  return api.post<InstallToken>("/v1/install-tokens", req);
}

export async function listInstallTokens(companyId?: string): Promise<InstallToken[]> {
  const params: Record<string, string> = companyId ? { company_id: companyId } : {};
  return api.get<InstallToken[]>("/v1/install-tokens", params);
}

export async function revokeInstallToken(tokenId: string): Promise<void> {
  await api.delete(`/v1/install-tokens/${tokenId}`);
}
