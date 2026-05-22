import { api } from "./client";

export interface LoginRequest {
  email: string;
  password: string;
}

export interface RegisterCompanyRequest {
  company_name: string;
  admin_name: string;
  admin_email: string;
  admin_password: string;
  plan?: string;
}

export interface AuthResponse {
  token: string;
  user: {
    id: string;
    email: string;
    name: string;
    role: "super_admin" | "company_admin";
    company_id: string | null;
  };
}

export async function login(req: LoginRequest): Promise<AuthResponse> {
  return api.post<AuthResponse>("/v1/auth/login", req);
}

export async function registerCompany(req: RegisterCompanyRequest): Promise<AuthResponse> {
  return api.post<AuthResponse>("/v1/auth/register", req);
}

export async function getCurrentUser(): Promise<AuthResponse> {
  return api.get<AuthResponse>("/v1/auth/me");
}