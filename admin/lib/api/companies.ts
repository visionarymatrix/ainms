import { api } from "./client";

export interface Company {
  id: string;
  tenant_id: string;
  name: string;
  plan: string;
  settings: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export async function listCompanies(tenantId?: string): Promise<Company[]> {
  const params: Record<string, string> = {};
  if (tenantId) params.tenant_id = tenantId;
  return api.get<Company[]>("/v1/companies", params);
}

export async function getCompany(id: string): Promise<Company> {
  return api.get<Company>(`/v1/companies/${id}`);
}

export async function createCompany(data: { name: string; plan?: string }): Promise<Company> {
  return api.post<Company>("/v1/companies", data);
}