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

export async function listEmployees(companyId: string): Promise<Employee[]> {
  return api.get<Employee[]>(`/v1/companies/${companyId}/employees`);
}

export async function getEmployee(id: string): Promise<Employee> {
  return api.get<Employee>(`/v1/employees/${id}`);
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