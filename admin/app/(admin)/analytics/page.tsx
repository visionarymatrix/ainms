"use client";

import { useEffect, useState, useCallback, useMemo } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api/client";
import { isSuperAdmin, getCompanyId } from "@/lib/auth/session";
import { BarChart3, Clock, Monitor, User } from "lucide-react";
import { toast } from "sonner";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  Legend,
} from "recharts";
import { listCompanies } from "@/lib/api/companies";
import type { Company } from "@/lib/api/companies";
import { formatDuration } from "@/lib/utils/format";

interface Employee {
  id: string;
  first_name: string;
  last_name: string;
  employee_id: string;
}

interface Device {
  id: string;
  hostname: string | null;
  employee_id: string;
  company_id: string;
  status: string;
}

interface AppUsageSummary {
  device_id: string;
  app_name: string;
  date: string;
  total_duration_sec: number;
  session_count: number;
  productive_duration_sec: number;
  unproductive_duration_sec: number;
  neutral_duration_sec: number;
}

function toDateInputValue(date: Date): string {
  const tzOffset = date.getTimezoneOffset() * 60000;
  const localISOTime = new Date(date.getTime() - tzOffset).toISOString().slice(0, 10);
  return localISOTime;
}

const CLASSIFICATION_COLORS: Record<string, string> = {
  productive: "#22c55e",
  unproductive: "#ef4444",
  neutral: "#6b7280",
};

export default function AnalyticsPage() {
  const isSuper = isSuperAdmin();
  const userCompanyId = getCompanyId();

  const [companies, setCompanies] = useState<Company[]>([]);
  const [selectedCompanyId, setSelectedCompanyId] = useState<string>("");
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [selectedEmployeeId, setSelectedEmployeeId] = useState<string>("");
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>("");
  const [date, setDate] = useState<string>(toDateInputValue(new Date()));
  const [usageData, setUsageData] = useState<AppUsageSummary[]>([]);

  const [loadingCompanies, setLoadingCompanies] = useState(isSuper);
  const [loadingEmployees, setLoadingEmployees] = useState(false);
  const [loadingDevices, setLoadingDevices] = useState(true);
  const [loadingUsage, setLoadingUsage] = useState(false);

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
      const sorted = (data || []).slice().sort((a, b) => `${a.first_name} ${a.last_name}`.localeCompare(`${b.first_name} ${b.last_name}`));
      setEmployees(sorted);
    } catch {
      toast.error("Failed to load employees");
      setEmployees([]);
    } finally {
      setLoadingEmployees(false);
    }
  }, []);

  const fetchDevices = useCallback(async () => {
    setLoadingDevices(true);
    try {
      const data = await api.get<Device[]>("/v1/devices/status");
      const active = (data || []).filter((d) => d.status === "active");
      setDevices(active);
    } catch {
      toast.error("Failed to load devices");
      setDevices([]);
    } finally {
      setLoadingDevices(false);
    }
  }, []);

  const fetchUsage = useCallback(async (deviceId: string, dateStr: string) => {
    if (!deviceId || !dateStr) return;
    setLoadingUsage(true);
    try {
      const data = await api.get<AppUsageSummary[]>(`/v1/devices/${deviceId}/app-usage?date=${dateStr}`);
      const sorted = (data || []).sort((a, b) => b.total_duration_sec - a.total_duration_sec);
      setUsageData(sorted);
    } catch {
      toast.error("Failed to load app usage data");
      setUsageData([]);
    } finally {
      setLoadingUsage(false);
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
    fetchDevices();
  }, [fetchDevices]);

  useEffect(() => {
    if (selectedCompanyId) {
      fetchEmployees(selectedCompanyId);
      setSelectedEmployeeId("");
      setSelectedDeviceId("");
      setUsageData([]);
    }
  }, [selectedCompanyId, fetchEmployees]);

  useEffect(() => {
    if (selectedEmployeeId) {
      const empDevices = devices.filter((d) => d.employee_id === selectedEmployeeId);
      if (empDevices.length > 0) {
        setSelectedDeviceId(empDevices[0].id);
      } else {
        setSelectedDeviceId("");
      }
      setUsageData([]);
    }
  }, [selectedEmployeeId, devices]);

  useEffect(() => {
    if (selectedDeviceId && date) {
      fetchUsage(selectedDeviceId, date);
    }
  }, [selectedDeviceId, date, fetchUsage]);

  const employeeDevices = useMemo(() => {
    if (!selectedEmployeeId) return [];
    return devices.filter((d) => d.employee_id === selectedEmployeeId);
  }, [selectedEmployeeId, devices]);

  const barData = useMemo(() => {
    return usageData.map((u) => ({
      name: u.app_name,
      hours: Number((u.total_duration_sec / 3600).toFixed(2)),
      ...u,
    }));
  }, [usageData]);

  const classificationData = useMemo(() => {
    const productive = usageData.reduce((sum, u) => sum + u.productive_duration_sec, 0);
    const unproductive = usageData.reduce((sum, u) => sum + u.unproductive_duration_sec, 0);
    const neutral = usageData.reduce((sum, u) => sum + u.neutral_duration_sec, 0);
    return [
      { name: "Productive", value: productive, color: CLASSIFICATION_COLORS.productive },
      { name: "Unproductive", value: unproductive, color: CLASSIFICATION_COLORS.unproductive },
      { name: "Neutral", value: neutral, color: CLASSIFICATION_COLORS.neutral },
    ].filter((d) => d.value > 0);
  }, [usageData]);

  const totalHours = useMemo(() => {
    const totalSec = usageData.reduce((sum, u) => sum + u.total_duration_sec, 0);
    return totalSec / 3600;
  }, [usageData]);

  const mostUsedApp = useMemo(() => {
    if (usageData.length === 0) return "—";
    return usageData[0].app_name;
  }, [usageData]);

  const appCount = usageData.length;

  const isLoading = loadingCompanies || loadingEmployees || loadingDevices;

  if (isLoading && !selectedCompanyId && !userCompanyId) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-semibold tracking-tight">App Hours Analytics</h1>
        <div className="grid gap-4 md:grid-cols-3">
          {[1, 2, 3].map((i) => (
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
          <h1 className="text-2xl font-semibold tracking-tight">App Hours Analytics</h1>
          <p className="text-muted-foreground">
            Per-employee, per-device app usage breakdown by day.
          </p>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
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
          <Select value={selectedEmployeeId} onValueChange={setSelectedEmployeeId} disabled={!selectedCompanyId || loadingEmployees}>
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
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
            <Monitor className="h-3.5 w-3.5" />
            Device
          </label>
          <Select value={selectedDeviceId} onValueChange={setSelectedDeviceId} disabled={employeeDevices.length === 0}>
            <SelectTrigger className="w-full">
              <SelectValue placeholder="Select device" />
            </SelectTrigger>
            <SelectContent>
              {employeeDevices.map((d) => (
                <SelectItem key={d.id} value={d.id}>
                  {d.hostname || d.id.slice(0, 8)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
            <Clock className="h-3.5 w-3.5" />
            Date
          </label>
          <Input
            type="date"
            value={date}
            onChange={(e) => setDate(e.target.value)}
            className="w-full"
          />
        </div>
      </div>

      {selectedDeviceId && (
        <>
          <div className="grid gap-4 md:grid-cols-3">
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                  <Clock className="h-4 w-4" />
                  Total Hours
                </CardTitle>
              </CardHeader>
              <CardContent>
                {loadingUsage ? (
                  <Skeleton className="h-9 w-24" />
                ) : (
                  <div className="text-3xl font-bold">{totalHours.toFixed(1)}h</div>
                )}
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                  <BarChart3 className="h-4 w-4" />
                  Most Used App
                </CardTitle>
              </CardHeader>
              <CardContent>
                {loadingUsage ? (
                  <Skeleton className="h-9 w-40" />
                ) : (
                  <div className="text-xl font-bold truncate" title={mostUsedApp}>
                    {mostUsedApp}
                  </div>
                )}
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                  <Monitor className="h-4 w-4" />
                  Apps Tracked
                </CardTitle>
              </CardHeader>
              <CardContent>
                {loadingUsage ? (
                  <Skeleton className="h-9 w-16" />
                ) : (
                  <div className="text-3xl font-bold">{appCount}</div>
                )}
              </CardContent>
            </Card>
          </div>

          {loadingUsage ? (
            <Card>
              <CardHeader>
                <Skeleton className="h-6 w-48" />
                <Skeleton className="h-4 w-72" />
              </CardHeader>
              <CardContent>
                <Skeleton className="h-64 w-full" />
              </CardContent>
            </Card>
          ) : usageData.length === 0 ? (
            <Card>
              <CardContent className="py-12 text-center">
                <BarChart3 className="mx-auto h-10 w-10 text-muted-foreground" />
                <h3 className="mt-4 text-lg font-semibold">No data</h3>
                <p className="mt-1 text-sm text-muted-foreground">
                  No app usage recorded for this device on the selected date.
                </p>
              </CardContent>
            </Card>
          ) : (
            <div className="grid gap-4 lg:grid-cols-3">
              <Card className="lg:col-span-2">
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <BarChart3 className="h-5 w-5" />
                    App Usage by Hours
                  </CardTitle>
                  <CardDescription>
                    Sorted by total time spent, descending
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="h-80">
                    <ResponsiveContainer width="100%" height="100%">
                      <BarChart data={barData} margin={{ top: 8, right: 8, bottom: 8, left: 8 }}>
                        <CartesianGrid strokeDasharray="3 3" vertical={false} />
                        <XAxis dataKey="name" tick={{ fontSize: 12 }} angle={-30} textAnchor="end" height={60} interval={0} />
                        <YAxis tick={{ fontSize: 12 }} label={{ value: "Hours", angle: -90, position: "insideLeft", style: { fontSize: 12 } }} />
                        <RechartsTooltip
                          formatter={(value: any) => [`${Number(value).toFixed(2)}h`, "Hours"]}
                          contentStyle={{ borderRadius: "6px", fontSize: 12 }}
                        />
                        <Bar dataKey="hours" fill="hsl(var(--primary))" radius={[4, 4, 0, 0]} />
                      </BarChart>
                    </ResponsiveContainer>
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <BarChart3 className="h-5 w-5" />
                    Classification
                  </CardTitle>
                  <CardDescription>
                    Productive vs unproductive vs neutral time
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="h-80">
                    <ResponsiveContainer width="100%" height="100%">
                      <PieChart>
                        <Pie
                          data={classificationData}
                          dataKey="value"
                          nameKey="name"
                          cx="50%"
                          cy="50%"
                          outerRadius={100}
                          label={(props: any) => `${props.name} ${(props.percent * 100).toFixed(0)}%`}
                          labelLine
                        >
                          {classificationData.map((entry, index) => (
                            <Cell key={`cell-${index}`} fill={entry.color} />
                          ))}
                        </Pie>
                        <RechartsTooltip
                          formatter={(value: any) => formatDuration(value as number)}
                          contentStyle={{ borderRadius: "6px", fontSize: 12 }}
                        />
                        <Legend verticalAlign="bottom" height={36} wrapperStyle={{ fontSize: 12 }} />
                      </PieChart>
                    </ResponsiveContainer>
                  </div>
                </CardContent>
              </Card>
            </div>
          )}
        </>
      )}

      {!selectedDeviceId && (
        <Card>
          <CardContent className="py-12 text-center">
            <Monitor className="mx-auto h-10 w-10 text-muted-foreground" />
            <h3 className="mt-4 text-lg font-semibold">Select a device</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Choose a company, employee, and device to view app usage analytics.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
