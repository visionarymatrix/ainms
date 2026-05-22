export interface User {
  id: string;
  email: string;
  name: string;
  role: "super_admin" | "company_admin";
  company_id: string | null;
}

export interface AuthState {
  user: User | null;
  token: string | null;
  isLoading: boolean;
}

const TOKEN_KEY = "ainms_token";
const USER_KEY = "ainms_user";

export function getToken(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem(TOKEN_KEY);
}

export function getUser(): User | null {
  if (typeof window === "undefined") return null;
  const raw = localStorage.getItem(USER_KEY);
  if (!raw) return null;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

export function setAuth(token: string, user: User): void {
  localStorage.setItem(TOKEN_KEY, token);
  localStorage.setItem(USER_KEY, JSON.stringify(user));
  document.cookie = `ainms_session=${token}; path=/; max-age=${60 * 60 * 24 * 7}; SameSite=Lax`;
}

export function clearAuth(): void {
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(USER_KEY);
  document.cookie = "ainms_session=; path=/; max-age=0";
}

export function isAuthenticated(): boolean {
  if (typeof window === "undefined") return false;
  return !!getToken();
}

export function isSuperAdmin(): boolean {
  const user = getUser();
  return user?.role === "super_admin";
}

export function getCompanyId(): string | null {
  const user = getUser();
  return user?.company_id ?? null;
}