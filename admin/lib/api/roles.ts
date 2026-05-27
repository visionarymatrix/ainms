import { api } from "./client";

export interface Role {
  id: string;
  company_id: string;
  name: string;
  description: string;
  work_description: string;
  allowed_categories: string[];
  blocked_categories: string[];
  created_at: string;
  updated_at: string;
}

export interface CreateRoleRequest {
  name: string;
  description?: string;
  work_description?: string;
  allowed_categories?: string[];
  blocked_categories?: string[];
}

export interface AppClassification {
  id: string;
  role_id: string;
  app_name: string;
  category: string;
  created_at: string;
}

export interface CreateAppClassificationRequest {
  app_name: string;
  category: string;
}

export interface AlertRule {
  id: string;
  role_id: string;
  category: string;
  threshold_min: number;
  popup_type: string;
  created_at: string;
}

export interface CreateAlertRuleRequest {
  category: string;
  threshold_min: number;
  popup_type: string;
}

export async function listRoles(companyId: string): Promise<Role[]> {
  return api.get<Role[]>(`/v1/companies/${companyId}/roles`);
}

export async function getRole(roleId: string): Promise<Role> {
  return api.get<Role>(`/v1/roles/${roleId}`);
}

export async function createRole(companyId: string, data: CreateRoleRequest): Promise<Role> {
  return api.post<Role>(`/v1/companies/${companyId}/roles`, data);
}

export async function updateRole(roleId: string, data: Partial<CreateRoleRequest>): Promise<Role> {
  return api.put<Role>(`/v1/roles/${roleId}`, data);
}

export async function deleteRole(roleId: string): Promise<void> {
  return api.delete(`/v1/roles/${roleId}`);
}

export async function listAppClassifications(roleId: string): Promise<AppClassification[]> {
  return api.get<AppClassification[]>(`/v1/roles/${roleId}/app-classifications`);
}

export async function createAppClassification(
  roleId: string,
  data: CreateAppClassificationRequest,
): Promise<AppClassification> {
  return api.post<AppClassification>(`/v1/roles/${roleId}/app-classifications`, data);
}

export async function deleteAppClassification(
  roleId: string,
  classificationId: string,
): Promise<void> {
  return api.delete(`/v1/roles/${roleId}/app-classifications/${classificationId}`);
}

export async function listAlertRules(roleId: string): Promise<AlertRule[]> {
  return api.get<AlertRule[]>(`/v1/roles/${roleId}/alert-rules`);
}

export async function createAlertRule(
  roleId: string,
  data: CreateAlertRuleRequest,
): Promise<AlertRule> {
  return api.post<AlertRule>(`/v1/roles/${roleId}/alert-rules`, data);
}

export async function deleteAlertRule(roleId: string, ruleId: string): Promise<void> {
  return api.delete(`/v1/roles/${roleId}/alert-rules/${ruleId}`);
}