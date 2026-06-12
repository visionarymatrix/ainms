import { api } from "./client";

export interface Project {
  id: string;
  company_id: string;
  name: string;
  description: string | null;
  status: string;
  working_apps: string[];
  created_at: string;
  updated_at: string;
}

export interface EmployeeProjectAssignment {
  id: string;
  employee_id: string;
  project_id: string;
  assigned_by: string;
  started_at: string;
  ended_at: string | null;
  is_primary: boolean;
  created_at: string;
  updated_at: string;
}

export async function listProjects(companyId?: string): Promise<Project[]> {
  const params: Record<string, string> = {};
  if (companyId) params.company_id = companyId;
  return api.get<Project[]>("/v1/projects", params);
}

export async function getProject(projectId: string): Promise<Project> {
  return api.get<Project>(`/v1/projects/${projectId}`);
}

export async function createProject(data: {
  company_id?: string;
  name: string;
  description?: string;
  working_apps?: string[];
}): Promise<Project> {
  return api.post<Project>("/v1/projects", data);
}

export async function updateProject(
  projectId: string,
  data: Partial<{
    name: string;
    description: string;
    status: string;
    working_apps: string[];
  }>
): Promise<Project> {
  return api.put<Project>(`/v1/projects/${projectId}`, data);
}

export async function deleteProject(projectId: string): Promise<void> {
  return api.delete(`/v1/projects/${projectId}`);
}

export async function assignEmployeeToProject(
  projectId: string,
  data: {
    employee_id: string;
    is_primary?: boolean;
  }
): Promise<EmployeeProjectAssignment> {
  return api.post<EmployeeProjectAssignment>(`/v1/projects/${projectId}/assignments`, data);
}

export async function unassignEmployeeFromProject(
  projectId: string,
  employeeId: string
): Promise<void> {
  return api.delete(`/v1/projects/${projectId}/assignments/${employeeId}`);
}

export async function listEmployeeProjects(employeeId: string): Promise<EmployeeProjectAssignment[]> {
  return api.get<EmployeeProjectAssignment[]>(`/v1/employees/${employeeId}/projects`);
}