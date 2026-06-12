"use client";

import { useEffect, useState, useCallback, useMemo, Fragment } from "react";
import Link from "next/link";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api/client";
import { isSuperAdmin, getCompanyId } from "@/lib/auth/session";
import {
  Monitor,
  User,
  LayoutGrid,
  CheckCircle2,
  XCircle,
  MinusCircle,
} from "lucide-react";
import { toast } from "sonner";
import { listCompanies } from "@/lib/api/companies";
import type { Company } from "@/lib/api/companies";

interface Employee {
  id: string;
  first_name: string;
  last_name: string;
  employee_id: string;
}

interface InstalledAppWithDevice {
  id: string;
  device_id: string;
  app_name: string;
  display_name: string;
  publisher: string;
  install_path: string | null;
  category: string;
  confidence: number;
  source: string;
  created_at: string;
  updated_at: string;
  device_id_str: string;
  device_hostname: string | null;
  employee_id: string;
  employee_name: string;
}

const CLASSIFICATION_COLORS: Record<string, string> = {
  productive: "#22c55e",
  unproductive: "#ef4444",
  neutral: "#6b7280",
};

function formatDate(dateStr: string): string {
  if (!dateStr) return "—";
  const d = new Date(dateStr);
  if (isNaN(d.getTime())) return dateStr;
  return d.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function getCategoryBadgeStyle(category: string): React.CSSProperties {
  const color = CLASSIFICATION_COLORS[category] || CLASSIFICATION_COLORS.neutral;
  return {
    backgroundColor: color,
    color: "#ffffff",
  };
}

function getSourceLabel(source: string): string {
  if (source === "rule") return "Rule";
  if (source === "keyword_fallback") return "Keyword";
  return source;
}

export default function InstalledAppsPage() {
  const isSuper = isSuperAdmin();
  const userCompanyId = getCompanyId();

  const [companies, setCompanies] = useState<Company[]>([]);
  const [selectedCompanyId, setSelectedCompanyId] = useState<string>("");
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [selectedEmployeeId, setSelectedEmployeeId] = useState<string>("");
  const [installedApps, setInstalledApps] = useState<InstalledAppWithDevice[]>([]);

  const [loadingCompanies, setLoadingCompanies] = useState(isSuper);
  const [loadingEmployees, setLoadingEmployees] = useState(false);
  const [loadingApps, setLoadingApps] = useState(false);

  const fetchCompanies = useCallback(async () => {
    if (!isSuper) return;
    setLoadingCompanies(true);
    try {
      const data = await listCompanies();
      setCompanies(data || []);
    } catch {
      setCompanies([]);
    } finally {
      setLoadingCompanies(false);
    }
  }, [isSuper]);

  const fetchEmployees = useCallback(async (companyId: string) => {
    if (!companyId) return;
    setLoadingEmployees(true);
    try {
      const data = await api.get<Employee[]>(`/v1/companies/${companyId}/employees`);
      const sorted = (data || [])
        .slice()
        .sort((a, b) =>
          `${a.first_name} ${a.last_name}`.localeCompare(`${b.first_name} ${b.last_name}`)
        );
      setEmployees(sorted);
    } catch {
      toast.error("Failed to load employees");
      setEmployees([]);
    } finally {
      setLoadingEmployees(false);
    }
  }, []);

  const fetchInstalledApps = useCallback(async (employeeId: string) => {
    if (!employeeId) return;
    setLoadingApps(true);
    try {
      const data = await api.get<InstalledAppWithDevice[]>(
        `/v1/employees/${employeeId}/installed-apps`
      );
      setInstalledApps(data || []);
    } catch {
      toast.error("Failed to load installed applications");
      setInstalledApps([]);
    } finally {
      setLoadingApps(false);
    }
  }, []);

  useEffect(() => {
    if (isSuper) {
      fetchCompanies();
    } else if (userCompanyId) {
      setSelectedCompanyId(userCompanyId);
    }
  }, [isSuper, userCompanyId, fetchCompanies]);

  useEffect(() => {
    if (selectedCompanyId) {
      fetchEmployees(selectedCompanyId);
      setSelectedEmployeeId("");
      setInstalledApps([]);
    }
  }, [selectedCompanyId, fetchEmployees]);

  useEffect(() => {
    if (selectedEmployeeId) {
      fetchInstalledApps(selectedEmployeeId);
    } else {
      setInstalledApps([]);
    }
  }, [selectedEmployeeId, fetchInstalledApps]);

  const stats = useMemo(() => {
    const total = installedApps.length;
    const productive = installedApps.filter((a) => a.category === "productive").length;
    const unproductive = installedApps.filter((a) => a.category === "unproductive").length;
    const neutral = installedApps.filter((a) => a.category === "neutral").length;
    return { total, productive, unproductive, neutral };
  }, [installedApps]);

  const groupedApps = useMemo(() => {
    const map = new Map<string, InstalledAppWithDevice[]>();
    for (const app of installedApps) {
      const key = app.device_id_str || app.device_id;
      const list = map.get(key) || [];
      list.push(app);
      map.set(key, list);
    }
    return Array.from(map.entries()).map(([deviceId, apps]) => ({
      deviceId,
      hostname: apps[0]?.device_hostname || deviceId.slice(0, 8),
      apps,
    }));
  }, [installedApps]);

  const isLoading = loadingCompanies || loadingEmployees;

  if (isLoading && !selectedCompanyId && !userCompanyId) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-semibold tracking-tight">Installed Applications</h1>
        <div className="grid gap-4 md:grid-cols-4">
          {[1, 2, 3, 4].map((i) => (
            <Skeleton key={i} className="h-32" />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Installed Applications</h1>
          <p className="text-muted-foreground">
            View all installed desktop applications per employee.
          </p>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        {isSuper && (
          <div className="space-y-2">
            <label className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
              <Monitor className="h-3.5 w-3.5" />
              Company
            </label>
            <Select value={selectedCompanyId} onValueChange={setSelectedCompanyId}>
              <SelectTrigger className="w-full">
                <SelectValue placeholder="Select company" />
              </SelectTrigger>
              <SelectContent>
                {companies.map((c) => (
                  <SelectItem key={c.id} value={c.id}>
                    {c.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}

        <div className="space-y-2">
          <label className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
            <User className="h-3.5 w-3.5" />
            Employee
          </label>
          <Select
            value={selectedEmployeeId}
            onValueChange={setSelectedEmployeeId}
            disabled={!selectedCompanyId || loadingEmployees}
          >
            <SelectTrigger className="w-full">
              <SelectValue placeholder="Select employee" />
            </SelectTrigger>
            <SelectContent>
              {employees.map((e) => (
                <SelectItem key={e.id} value={e.id}>
                  {e.first_name} {e.last_name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {selectedEmployeeId && (
            <Link
              href={`/employees/${selectedEmployeeId}`}
              className="text-xs text-blue-600 hover:underline"
            >
              View employee profile →
            </Link>
          )}
        </div>
      </div>

      {selectedEmployeeId && (
        <>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                  <LayoutGrid className="h-4 w-4" />
                  Total Apps
                </CardTitle>
              </CardHeader>
              <CardContent>
                {loadingApps ? (
                  <Skeleton className="h-9 w-16" />
                ) : (
                  <div className="text-3xl font-bold">{stats.total}</div>
                )}
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                  <CheckCircle2 className="h-4 w-4" />
                  Productive
                </CardTitle>
              </CardHeader>
              <CardContent>
                {loadingApps ? (
                  <Skeleton className="h-9 w-16" />
                ) : (
                  <div className="text-3xl font-bold">{stats.productive}</div>
                )}
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                  <XCircle className="h-4 w-4" />
                  Unproductive
                </CardTitle>
              </CardHeader>
              <CardContent>
                {loadingApps ? (
                  <Skeleton className="h-9 w-16" />
                ) : (
                  <div className="text-3xl font-bold">{stats.unproductive}</div>
                )}
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                  <MinusCircle className="h-4 w-4" />
                  Neutral
                </CardTitle>
              </CardHeader>
              <CardContent>
                {loadingApps ? (
                  <Skeleton className="h-9 w-16" />
                ) : (
                  <div className="text-3xl font-bold">{stats.neutral}</div>
                )}
              </CardContent>
            </Card>
          </div>

          {loadingApps ? (
            <Card>
              <CardHeader>
                <Skeleton className="h-6 w-48" />
                <Skeleton className="h-4 w-72" />
              </CardHeader>
              <CardContent>
                <Skeleton className="h-64 w-full" />
              </CardContent>
            </Card>
          ) : installedApps.length === 0 ? (
            <Card>
              <CardContent className="py-12 text-center">
                <Monitor className="mx-auto h-10 w-10 text-muted-foreground" />
                <h3 className="mt-4 text-lg font-semibold">No data</h3>
                <p className="mt-1 text-sm text-muted-foreground">
                  No installed applications data available for this employee.
                </p>
              </CardContent>
            </Card>
          ) : (
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <LayoutGrid className="h-5 w-5" />
                  Installed Applications
                </CardTitle>
              </CardHeader>
              <CardContent>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>App Name</TableHead>
                      <TableHead>Publisher</TableHead>
                      <TableHead>Category</TableHead>
                      <TableHead>Confidence</TableHead>
                      <TableHead>Source</TableHead>
                      <TableHead>Last Updated</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {groupedApps.map((group) => (
                      <Fragment key={`group-${group.deviceId}`}>
                        <TableRow className="bg-muted/50">
                          <TableCell
                            colSpan={6}
                            className="font-medium text-sm py-2"
                          >
                            <div className="flex items-center gap-2">
                              <Monitor className="h-4 w-4 text-muted-foreground" />
                              {group.hostname}
                            </div>
                          </TableCell>
                        </TableRow>
                        {group.apps.map((app) => (
                          <TableRow key={app.id}>
                            <TableCell className="font-medium">
                              {app.display_name || app.app_name}
                            </TableCell>
                            <TableCell>{app.publisher}</TableCell>
                            <TableCell>
                              <Badge
                                className="rounded-full"
                                style={getCategoryBadgeStyle(app.category)}
                              >
                                {app.category}
                              </Badge>
                            </TableCell>
                            <TableCell>{(app.confidence * 100).toFixed(0)}%</TableCell>
                            <TableCell>
                              <Badge variant="secondary">{getSourceLabel(app.source)}</Badge>
                            </TableCell>
                            <TableCell>{formatDate(app.updated_at)}</TableCell>
                          </TableRow>
                        ))}
                      </Fragment>
                    ))}
                  </TableBody>
                </Table>
              </CardContent>
            </Card>
          )}
        </>
      )}

      {!selectedEmployeeId && (
        <Card>
          <CardContent className="py-12 text-center">
            <Monitor className="mx-auto h-10 w-10 text-muted-foreground" />
            <h3 className="mt-4 text-lg font-semibold">Select an employee</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Select an employee to view installed applications.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
