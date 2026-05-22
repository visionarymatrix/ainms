export interface Company {
  id: string;
  tenant_id: string;
  name: string;
  plan: string;
  settings: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

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
  mtls_cert_sn: string | null;
  status: string;
  last_heartbeat: string | null;
  enrolled_at: string;
  created_at: string;
  updated_at: string;
}

export interface User {
  id: string;
  email: string;
  name: string;
  role: "super_admin" | "company_admin";
  company_id: string | null;
}