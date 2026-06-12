"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Separator } from "@/components/ui/separator";
import { Input } from "@/components/ui/input";
import {
  ArrowLeft,
  Camera,
  Copy,
  Eye,
  EyeOff,
  Laptop,
  Monitor,
  RefreshCw,
  Send,
  Terminal,
  CloudOff,
  MessageSquare,
  Mail,
  Shield,
  Clock,
  Hash,
  CheckCircle2,
  XCircle,
  AlertCircle,
  Package,
  BarChart3,
  KeyRound,
  Cpu,
  ImageOff,
  CalendarDays,
  ChevronDown,
  ChevronUp,
  Plus,
  Trash2,
  Pause,
  Play,
} from "lucide-react";
import { toast } from "sonner";
import { getToken, getCompanyId } from "@/lib/auth/session";
import { useSocket } from "@/lib/socket";
import {
  getEmployee,
  getEmployeeDevices,
  requestScreenshot,
  getDeviceScreenshots,
  sendNLQuery,
  type Employee,
  type Device,
  type ScreenshotRequest,
  type NLQueryResponse,
  type AgentReport,
} from "@/lib/api/employees";
import {
  listTargetedSchedulesByEmployee,
  createTargetedSchedule,
  updateTargetedSchedule,
  deleteTargetedSchedule,
  type TargetedScreenshotSchedule,
} from "@/lib/api/targeted-schedules";
import { getEmployeeInstallToken, type EmployeeInstallToken } from "@/lib/api/install-tokens";
import { getRole, type Role } from "@/lib/api/roles";
import { getEmployeeValidations, type InstalledAppValidationWithDetails } from "@/lib/api/installed-apps";
import { listActivitySummaries, type ActivitySummary } from "@/lib/api/activity-summaries";
import { api } from "@/lib/api/client";
import { timeAgo, formatDate, formatDuration, maskToken, copyToClipboard } from "@/lib/utils/format";
import {
  DeviceStatusBadge,
  ConnectionStatusBadge,
  EmployeeStatusBadge,
  ScreenshotStatusBadge,
  TokenStatusBadge,
  ClassificationBadge,
} from "@/components/shared/status-badge";
import { ConnectionStatusDot } from "@/components/shared/connection-status-dot";
import { SocketIndicator } from "@/components/shared/socket-indicator";

interface AppUsageEvent {
  device_id: string;
  app_name: string;
  window_title: string;
  process_name: string;
  process_id: number;
  start_time: string;
  end_time: string;
  duration_sec: number;
  classification: string;
  confidence: number;
}

export default function EmployeeDetailPage() {
  const params = useParams();
  const employeeId = params.id as string;

  // ── Core data ────────────────────────────────────────────────
  const [employee, setEmployee] = useState<Employee | null>(null);
  const [employeeLoading, setEmployeeLoading] = useState(true);
  const [employeeError, setEmployeeError] = useState<string | null>(null);

  const [role, setRole] = useState<Role | null>(null);
  const [roleLoading, setRoleLoading] = useState(false);

  const [devices, setDevices] = useState<Device[]>([]);
  const [devicesLoading, setDevicesLoading] = useState(true);
  const [devicesError, setDevicesError] = useState<string | null>(null);

  // ── Tab state ────────────────────────────────────────────────
  const [activeTab, setActiveTab] = useState("overview");
  const [fetchedTabs, setFetchedTabs] = useState<Set<string>>(new Set());

  // ── Install token ─────────────────────────────────────────────
  const [installToken, setInstallToken] = useState<EmployeeInstallToken | null>(null);
  const [tokenLoading, setTokenLoading] = useState(false);
  const [tokenError, setTokenError] = useState<string | null>(null);
  const [showFullToken, setShowFullToken] = useState(false);
  const [installCmdTab, setInstallCmdTab] = useState("linux");

  // ── Screenshots ───────────────────────────────────────────────
  const [screenshots, setScreenshots] = useState<ScreenshotRequest[]>([]);
  const [screenshotsLoading, setScreenshotsLoading] = useState(false);
  const [screenshotsError, setScreenshotsError] = useState<string | null>(null);
  const [screenshotRequesting, setScreenshotRequesting] = useState<string | null>(null);
  const [viewingScreenshot, setViewingScreenshot] = useState<ScreenshotRequest | null>(null);
  const [screenshotFilter, setScreenshotFilter] = useState<"all" | "images">("images");

  // ── Activity ──────────────────────────────────────────────────
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>("");
  const [activitySummaries, setActivitySummaries] = useState<ActivitySummary[]>([]);
  const [activityEvents, setActivityEvents] = useState<AppUsageEvent[]>([]);
  const [activityLoading, setActivityLoading] = useState(false);
  const [activityError, setActivityError] = useState<string | null>(null);

  // ── Validations ───────────────────────────────────────────────
  const [validations, setValidations] = useState<InstalledAppValidationWithDetails[]>([]);
  const [validationsLoading, setValidationsLoading] = useState(false);
  const [validationsError, setValidationsError] = useState<string | null>(null);
  const [complianceFilter, setComplianceFilter] = useState<"all" | "compliant" | "noncompliant">("all");

  // ── Schedules ───────────────────────────────────────────────
  const [schedules, setSchedules] = useState<TargetedScreenshotSchedule[]>([]);
  const [schedulesLoading, setSchedulesLoading] = useState(false);
  const [schedulesError, setSchedulesError] = useState<string | null>(null);
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [creatingSchedule, setCreatingSchedule] = useState(false);

  // Create schedule form state
  const [newScheduleName, setNewScheduleName] = useState("");
  const [newScheduleInterval, setNewScheduleInterval] = useState(5);
  const [newScheduleStartTime, setNewScheduleStartTime] = useState("09:00");
  const [newScheduleEndTime, setNewScheduleEndTime] = useState("17:00");
  const [newScheduleStartDate, setNewScheduleStartDate] = useState<string>("");
  const [newScheduleEndDate, setNewScheduleEndDate] = useState<string>("");
  const [newScheduleCron, setNewScheduleCron] = useState("");
  const [showAdvancedCron, setShowAdvancedCron] = useState(false);

  // ── NL Query ─────────────────────────────────────────────────
  const [nlQuery, setNlQuery] = useState("");
  const [nlQueryLoading, setNlQueryLoading] = useState(false);
  const [nlReports, setNlReports] = useState<Array<{ query: string; query_id: string; report: AgentReport | null; timestamp: string }>>([]);

  // ── Socket ────────────────────────────────────────────────────
  const token = getToken();
  const { isConnected, on } = useSocket(token);
  const pendingRequestIds = useRef<Record<string, string>>({});

  // ── Fetch employee ───────────────────────────────────────────
  const fetchEmployee = useCallback(async () => {
    setEmployeeLoading(true);
    setEmployeeError(null);
    try {
      const data = await getEmployee(employeeId);
      setEmployee(data);
    } catch {
      setEmployeeError("Failed to load employee");
    } finally {
      setEmployeeLoading(false);
    }
  }, [employeeId]);

  // ── Fetch role ───────────────────────────────────────────────
  useEffect(() => {
    if (employee?.role_id) {
      setRoleLoading(true);
      getRole(employee.role_id)
        .then(setRole)
        .catch(() => setRole(null))
        .finally(() => setRoleLoading(false));
    } else {
      setRole(null);
    }
  }, [employee?.role_id]);

  // ── Fetch devices ─────────────────────────────────────────────
  const fetchDevices = useCallback(async () => {
    setDevicesLoading(true);
    setDevicesError(null);
    try {
      const data = await getEmployeeDevices(employeeId);
      setDevices(data || []);
    } catch {
      setDevicesError("Failed to load devices");
      setDevices([]);
    } finally {
      setDevicesLoading(false);
    }
  }, [employeeId]);

  // ── Fetch install token ───────────────────────────────────────
  const fetchInstallToken = useCallback(async () => {
    setTokenLoading(true);
    setTokenError(null);
    try {
      const data = await getEmployeeInstallToken(employeeId);
      setInstallToken(data);
    } catch {
      setInstallToken(null);
    } finally {
      setTokenLoading(false);
    }
  }, [employeeId]);

  // ── Fetch screenshots ────────────────────────────────────────
  const fetchScreenshots = useCallback(async () => {
    if (devices.length === 0) {
      setScreenshots([]);
      return;
    }
    setScreenshotsLoading(true);
    setScreenshotsError(null);
    try {
      const allScreenshots: ScreenshotRequest[] = [];
      await Promise.all(
        devices.map(async (device) => {
          const data = await getDeviceScreenshots(device.id);
          if (data && data.length > 0) {
            allScreenshots.push(...data);
          }
        })
      );
      allScreenshots.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
      setScreenshots(allScreenshots);
    } catch {
      setScreenshotsError("Failed to load screenshots");
      setScreenshots([]);
    } finally {
      setScreenshotsLoading(false);
    }
  }, [devices]);

  // ── Fetch activity ────────────────────────────────────────────
  const fetchActivity = useCallback(async (deviceId: string) => {
    if (!deviceId) return;
    setActivityLoading(true);
    setActivityError(null);
    try {
      const [summariesData, eventsData] = await Promise.all([
        listActivitySummaries(deviceId, { limit: 10 }),
        api.get<AppUsageEvent[]>(`/v1/devices/${deviceId}/events`),
      ]);
      setActivitySummaries(summariesData || []);
      setActivityEvents((eventsData as AppUsageEvent[]) || []);
    } catch {
      setActivityError("Failed to load activity data");
      setActivitySummaries([]);
      setActivityEvents([]);
    } finally {
      setActivityLoading(false);
    }
  }, []);

  // ── Fetch validations ────────────────────────────────────────
  const fetchValidations = useCallback(async () => {
    setValidationsLoading(true);
    setValidationsError(null);
    try {
      const data = await getEmployeeValidations(employeeId);
      setValidations(data || []);
    } catch {
      setValidationsError("Failed to load app validations");
      setValidations([]);
    } finally {
      setValidationsLoading(false);
    }
  }, [employeeId]);

  // ── Fetch schedules ────────────────────────────────────────
  const fetchSchedules = useCallback(async () => {
    setSchedulesLoading(true);
    setSchedulesError(null);
    try {
      const data = await listTargetedSchedulesByEmployee(employeeId);
      setSchedules(data || []);
    } catch {
      setSchedulesError("Failed to load schedules");
      setSchedules([]);
    } finally {
      setSchedulesLoading(false);
    }
  }, [employeeId]);

  // ── Initial load ─────────────────────────────────────────────
  useEffect(() => {
    fetchEmployee();
    fetchDevices();
  }, [fetchEmployee, fetchDevices]);

  // ── Lazy tab loading ──────────────────────────────────────────
  useEffect(() => {
    if (fetchedTabs.has(activeTab)) return;

    const loadTab = async () => {
      setFetchedTabs((prev) => new Set(prev).add(activeTab));
      switch (activeTab) {
        case "overview":
          fetchInstallToken();
          break;
        case "devices":
          // Already loaded
          break;
        case "activity":
          // Will load when device is selected
          break;
        case "screenshots":
          fetchScreenshots();
          break;
        case "installed-apps":
        case "app-validations":
          fetchValidations();
          break;
        case "install-tokens":
          fetchInstallToken();
          break;
        case "schedule":
          fetchSchedules();
          break;
      }
    };
    loadTab();
  }, [activeTab, fetchedTabs, fetchInstallToken, fetchScreenshots, fetchValidations, fetchSchedules]);

  // ── Screenshot request handler ────────────────────────────────
  const handleTakeScreenshot = useCallback(async (deviceId: string) => {
    setScreenshotRequesting(deviceId);
    try {
      const req = await requestScreenshot(deviceId);
      const requestId = req.id;
      pendingRequestIds.current[deviceId] = requestId;
      toast.success("Screenshot requested");

      await new Promise<void>((resolve) => {
        const timeout = setTimeout(async () => {
          try {
            const data = await getDeviceScreenshots(deviceId);
            if (data && data.length > 0) {
              const completed = data.find((s) => s.id === requestId && s.status === "completed" && s.image_path);
              if (completed) {
                setScreenshots((prev) => {
                  const filtered = prev.filter((s) => s.id !== completed.id);
                  const updated = [completed, ...filtered];
                  updated.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
                  return updated;
                });
                toast.success("Screenshot captured!");
              }
            }
          } catch {
            toast.warning("Screenshot timeout. Agent may not have responded.");
          } finally {
            resolve();
          }
        }, 60000);

        const checkInterval = setInterval(() => {
          if (!pendingRequestIds.current[deviceId]) {
            clearTimeout(timeout);
            clearInterval(checkInterval);
            resolve();
          }
        }, 500);
      });
    } catch {
      toast.error("Failed to request screenshot");
    } finally {
      delete pendingRequestIds.current[deviceId];
      setScreenshotRequesting(null);
    }
  }, []);

  // ── NL Query handler ─────────────────────────────────────────
  const handleSendNLQuery = useCallback(async () => {
    if (!nlQuery.trim()) return;
    setNlQueryLoading(true);
    const queryText = nlQuery.trim();
    setNlQuery("");
    try {
      const resp = await sendNLQuery(employeeId, queryText);
      setNlReports((prev) => [...prev, { query: queryText, query_id: resp.query_id, report: null, timestamp: new Date().toISOString() }]);
    } catch {
      toast.error("Failed to send query to agent");
    } finally {
      setNlQueryLoading(false);
    }
  }, [employeeId, nlQuery]);

  // ── Socket events ────────────────────────────────────────────
  useEffect(() => {
    const offOnline = on("device_online", (data: { device_id: string }) => {
      setDevices((prev) =>
        prev.map((d) =>
          d.id === data.device_id
            ? { ...d, connection_status: "online" as const, last_heartbeat: new Date().toISOString() }
            : d
        )
      );
    });
    const offOffline = on("device_offline", (data: { device_id: string }) => {
      setDevices((prev) =>
        prev.map((d) =>
          d.id === data.device_id ? { ...d, connection_status: "offline" as const } : d
        )
      );
    });
    const offScreenshotReady = on("screenshot_ready", (data: { request_id: string; device_id: string; status: string; image_path: string }) => {
      delete pendingRequestIds.current[data.device_id];
      if (fetchedTabs.has("screenshots")) {
        fetchScreenshots();
      }
    });
    const offAgentReport = on("agent_report", (data: AgentReport) => {
      if (data.query_id) {
        setNlReports((prev) =>
          prev.map((r) =>
            r.query_id === data.query_id ? { ...r, report: data } : r
          )
        );
      }
    });

    return () => {
      offOnline();
      offOffline();
      offScreenshotReady();
      offAgentReport();
    };
  }, [on, fetchScreenshots, fetchedTabs]);

  // ── Auto-select device for activity tab ──────────────────────
  useEffect(() => {
    if (devices.length > 0 && !selectedDeviceId) {
      setSelectedDeviceId(devices[0].id);
    }
  }, [devices, selectedDeviceId]);

  // ── Load activity when device changes ─────────────────────────
  useEffect(() => {
    if (selectedDeviceId && activeTab === "activity") {
      fetchActivity(selectedDeviceId);
    }
  }, [selectedDeviceId, fetchActivity, activeTab]);

  // ── Schedule handlers ───────────────────────────────────────
  const handleCreateSchedule = useCallback(async () => {
    const companyId = getCompanyId();
    if (!companyId) {
      toast.error("Company ID not found");
      return;
    }
    setCreatingSchedule(true);
    try {
      const data = await createTargetedSchedule({
        company_id: companyId,
        employee_id: employeeId,
        name: newScheduleName || undefined,
        interval_minutes: newScheduleCron ? 5 : newScheduleInterval,
        cron_expression: newScheduleCron || undefined,
        start_time: newScheduleStartTime || undefined,
        end_time: newScheduleEndTime || undefined,
        start_date: newScheduleStartDate || undefined,
        end_date: newScheduleEndDate || undefined,
      });
      setSchedules((prev) => [data, ...prev]);
      toast.success("Schedule created");
      setCreateDialogOpen(false);
      setNewScheduleName("");
      setNewScheduleInterval(5);
      setNewScheduleStartTime("09:00");
      setNewScheduleEndTime("17:00");
      setNewScheduleStartDate("");
      setNewScheduleEndDate("");
      setNewScheduleCron("");
      setShowAdvancedCron(false);
    } catch {
      toast.error("Failed to create schedule");
    } finally {
      setCreatingSchedule(false);
    }
  }, [employeeId, newScheduleName, newScheduleInterval, newScheduleStartTime, newScheduleEndTime, newScheduleStartDate, newScheduleEndDate, newScheduleCron]);

  const handleToggleScheduleStatus = useCallback(async (schedule: TargetedScreenshotSchedule) => {
    const nextStatus = schedule.status === "active" ? "paused" : "active";
    try {
      const updated = await updateTargetedSchedule(schedule.id, { status: nextStatus });
      setSchedules((prev) => prev.map((s) => (s.id === updated.id ? updated : s)));
      toast.success(`Schedule ${nextStatus}`);
    } catch {
      toast.error("Failed to update schedule");
    }
  }, []);

  const handleDeleteSchedule = useCallback(async (scheduleId: string) => {
    try {
      await deleteTargetedSchedule(scheduleId);
      setSchedules((prev) => prev.filter((s) => s.id !== scheduleId));
      toast.success("Schedule deleted");
    } catch {
      toast.error("Failed to delete schedule");
    }
  }, []);

  const applyPreset = useCallback((preset: { name: string; interval_minutes: number; start_time: string; end_time: string }) => {
    setNewScheduleName(preset.name);
    setNewScheduleInterval(preset.interval_minutes);
    setNewScheduleStartTime(preset.start_time);
    setNewScheduleEndTime(preset.end_time);
    setNewScheduleCron("");
    setShowAdvancedCron(false);
  }, []);

  // ── Computed values ───────────────────────────────────────────
  const linuxCommand = installToken?.install_cmd || "";
  const windowsCommand = installToken?.windows_cmd || "";

  const filteredValidations = validations.filter((v) => {
    if (complianceFilter === "compliant") return v.is_compliant;
    if (complianceFilter === "noncompliant") return !v.is_compliant;
    return true;
  });

  const compliantCount = validations.filter((v) => v.is_compliant).length;
  const nonCompliantCount = validations.filter((v) => !v.is_compliant).length;
  const uniqueApps = new Set(validations.map((v) => v.app_name)).size;

  // ── Loading state ─────────────────────────────────────────────
  if (employeeLoading) {
    return (
      <div className="space-y-6 p-6">
        <Skeleton className="h-6 w-32" />
        <Skeleton className="h-10 w-64" />
        <Skeleton className="h-[500px] w-full" />
      </div>
    );
  }

  if (employeeError || !employee) {
    return (
      <div className="space-y-6 p-6">
        <Link href="/employees" className="inline-flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors">
          <ArrowLeft className="h-4 w-4" />
          Back to Employees
        </Link>
        <Card>
          <CardContent className="py-12 text-center">
            <p className="text-muted-foreground">{employeeError || "Employee not found"}</p>
            <Button variant="outline" className="mt-4" onClick={fetchEmployee}>
              Retry
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  // ── Render ────────────────────────────────────────────────────
  return (
    <TooltipProvider delayDuration={200}>
      <div className="space-y-6 p-6">
        {/* Breadcrumb */}
        <Link href="/employees" className="inline-flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors">
          <ArrowLeft className="h-4 w-4" />
          Back to Employees
        </Link>

        {/* Header */}
        <div className="flex items-start justify-between">
          <div className="space-y-1">
            <div className="flex items-center gap-3">
              <div className="h-14 w-14 rounded-full bg-primary/10 flex items-center justify-center text-primary font-bold text-lg shrink-0">
                {employee.first_name[0]}{employee.last_name[0]}
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <h1 className="text-2xl font-semibold tracking-tight">
                    {employee.first_name} {employee.last_name}
                  </h1>
                  <EmployeeStatusBadge status={employee.status} />
                </div>
                <div className="flex items-center gap-3 text-sm text-muted-foreground mt-1">
                  <span className="flex items-center gap-1">
                    <Hash className="h-3.5 w-3.5" />
                    <code className="font-mono text-xs bg-muted px-1.5 py-0.5 rounded">{employee.employee_id}</code>
                  </span>
                  {employee.email && (
                    <span className="flex items-center gap-1">
                      <Mail className="h-3.5 w-3.5" />
                      {employee.email}
                    </span>
                  )}
                  <span className="flex items-center gap-1">
                    <Clock className="h-3.5 w-3.5" />
                    Joined {formatDate(employee.created_at)}
                  </span>
                </div>
              </div>
            </div>
          </div>
          <SocketIndicator isConnected={isConnected} />
        </div>

        {/* Role info */}
        {employee.role_id && (
          <div className="mt-2">
            {roleLoading ? (
              <Skeleton className="h-8 w-48" />
            ) : role ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Shield className="h-4 w-4" />
                <span className="font-medium text-foreground">{role.name}</span>
                {role.work_description && (
                  <>
                    <span className="text-border">—</span>
                    <span>{role.work_description}</span>
                  </>
                )}
              </div>
            ) : null}
          </div>
        )}

        {/* Tabs */}
        <Tabs value={activeTab} onValueChange={setActiveTab}>
          <TabsList className="grid grid-cols-8 w-full">
            <TabsTrigger value="overview" className="text-xs">Overview</TabsTrigger>
            <TabsTrigger value="devices" className="text-xs">Devices</TabsTrigger>
            <TabsTrigger value="activity" className="text-xs">Activity</TabsTrigger>
            <TabsTrigger value="screenshots" className="text-xs">Screenshots</TabsTrigger>
            <TabsTrigger value="installed-apps" className="text-xs">Apps</TabsTrigger>
            <TabsTrigger value="app-validations" className="text-xs">Validations</TabsTrigger>
            <TabsTrigger value="install-tokens" className="text-xs">Tokens</TabsTrigger>
            <TabsTrigger value="schedule" className="text-xs">Schedule</TabsTrigger>
          </TabsList>

          {/* ─── Overview Tab ─────────────────────────────────────── */}
          <TabsContent value="overview" className="space-y-6 mt-4">
            {/* Info cards */}
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
              <Card>
                <CardHeader className="pb-2">
                  <CardDescription>Employee ID</CardDescription>
                </CardHeader>
                <CardContent>
                  <code className="font-mono text-sm">{employee.employee_id}</code>
                </CardContent>
              </Card>
              <Card>
                <CardHeader className="pb-2">
                  <CardDescription>Status</CardDescription>
                </CardHeader>
                <CardContent>
                  <EmployeeStatusBadge status={employee.status} />
                </CardContent>
              </Card>
              <Card>
                <CardHeader className="pb-2">
                  <CardDescription>Devices</CardDescription>
                </CardHeader>
                <CardContent>
                  <span className="text-2xl font-semibold">{devices.length}</span>
                </CardContent>
              </Card>
              <Card>
                <CardHeader className="pb-2">
                  <CardDescription>Screenshots</CardDescription>
                </CardHeader>
                <CardContent>
                  <span className="text-2xl font-semibold">{screenshots.length}</span>
                </CardContent>
              </Card>
            </div>

            {/* Role card */}
            {employee.role_id && role && (
              <Card>
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm flex items-center gap-2">
                    <Shield className="h-4 w-4 text-muted-foreground" />
                    Role
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="flex items-center gap-2">
                    <Badge variant="secondary">{role.name}</Badge>
                    {role.work_description && (
                      <span className="text-sm text-muted-foreground">{role.work_description}</span>
                    )}
                  </div>
                </CardContent>
              </Card>
            )}

            {/* Install token section */}
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm flex items-center gap-2">
                  <Terminal className="h-4 w-4 text-muted-foreground" />
                  Install Token
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                {tokenLoading ? (
                  <div className="space-y-2">
                    <Skeleton className="h-10 w-3/4 rounded-lg" />
                    <Skeleton className="h-28 w-full rounded-lg" />
                  </div>
                ) : installToken ? (
                  <div className="space-y-3">
                    <div className="flex items-center gap-2 text-sm bg-muted/50 border rounded-lg px-3 py-2">
                      <span className="text-muted-foreground shrink-0">Token</span>
                      <code className="font-mono text-xs flex-1 truncate select-all">
                        {showFullToken ? installToken.token : maskToken(installToken.token)}
                      </code>
                      <div className="flex items-center gap-1 shrink-0">
                        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => setShowFullToken(!showFullToken)}>
                          {showFullToken ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
                        </Button>
                        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => copyToClipboard(installToken.token, "Token")}>
                          <Copy className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </div>

                    <Tabs value={installCmdTab} onValueChange={setInstallCmdTab}>
                      <TabsList className="h-8 w-full grid grid-cols-2">
                        <TabsTrigger value="linux" className="text-xs">Linux / macOS</TabsTrigger>
                        <TabsTrigger value="windows" className="text-xs">Windows</TabsTrigger>
                      </TabsList>
                      <TabsContent value="linux" className="mt-2">
                        <div className="relative group">
                          <pre className="bg-zinc-950 text-zinc-100 p-4 rounded-lg text-xs font-mono leading-relaxed overflow-x-auto pr-24">
                            <code>{linuxCommand}</code>
                          </pre>
                          <Button
                            size="sm"
                            className="absolute top-2 right-2 h-7 text-xs gap-1.5 opacity-80 group-hover:opacity-100 transition-opacity"
                            onClick={() => copyToClipboard(linuxCommand, "Linux install command")}
                          >
                            <Copy className="h-3 w-3" />
                            Copy
                          </Button>
                        </div>
                      </TabsContent>
                      <TabsContent value="windows" className="mt-2">
                        <div className="relative group">
                          <pre className="bg-zinc-950 text-zinc-100 p-4 rounded-lg text-xs font-mono leading-relaxed overflow-x-auto pr-24">
                            <code>{windowsCommand}</code>
                          </pre>
                          <Button
                            size="sm"
                            className="absolute top-2 right-2 h-7 text-xs gap-1.5 opacity-80 group-hover:opacity-100 transition-opacity"
                            onClick={() => copyToClipboard(windowsCommand, "Windows install command")}
                          >
                            <Copy className="h-3 w-3" />
                            Copy
                          </Button>
                        </div>
                      </TabsContent>
                    </Tabs>

                    <p className="text-xs text-muted-foreground">
                      {installToken.expires_at
                        ? `Token expires ${formatDate(installToken.expires_at)}`
                        : "This token never expires"
                      }
                    </p>
                  </div>
                ) : (
                  <div className="flex flex-col items-center gap-3 py-6 border border-dashed rounded-lg bg-muted/30">
                    <p className="text-sm text-muted-foreground">No install token yet</p>
                    <Button size="sm" onClick={fetchInstallToken} disabled={tokenLoading}>
                      {tokenLoading ? "Generating..." : "Generate Install Token"}
                    </Button>
                  </div>
                )}
              </CardContent>
            </Card>

            {/* NL Query section */}
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm flex items-center gap-2">
                  <MessageSquare className="h-4 w-4 text-muted-foreground" />
                  Query Agent
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                {nlReports.length > 0 && (
                  <div className="space-y-3 max-h-80 overflow-y-auto">
                    {nlReports.map((entry, idx) => (
                      <div key={idx} className="space-y-2">
                        <div className="flex justify-end">
                          <div className="rounded-lg bg-primary text-primary-foreground px-3 py-2 text-sm max-w-[80%]">
                            {entry.query}
                          </div>
                        </div>
                        {entry.report ? (
                          <div className="flex justify-start">
                            <div className="rounded-lg border bg-card px-3 py-2 text-sm max-w-[80%] space-y-2">
                              <p>{entry.report.report_text}</p>
                              {entry.report.summary && (
                                <div className="text-xs text-muted-foreground space-y-1">
                                  <p>Events: {entry.report.summary.total_events}</p>
                                  {entry.report.summary.top_apps.length > 0 && (
                                    <p>Top apps: {entry.report.summary.top_apps.slice(0, 3).map((a) => `${a.app_name} (${Math.round(a.duration_sec / 60)}min)`).join(", ")}</p>
                                  )}
                                </div>
                              )}
                            </div>
                          </div>
                        ) : (
                          <div className="flex justify-start">
                            <div className="rounded-lg border bg-muted px-3 py-2 text-sm text-muted-foreground flex items-center gap-2">
                              <RefreshCw className="h-3 w-3 animate-spin" />
                              Waiting for agent response...
                            </div>
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                )}

                <div className="flex gap-2">
                  <Input
                    placeholder="Ask about this employee's activity..."
                    value={nlQuery}
                    onChange={(e) => setNlQuery(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && !e.shiftKey && !nlQueryLoading) {
                        e.preventDefault();
                        handleSendNLQuery();
                      }
                    }}
                    disabled={nlQueryLoading || devices.length === 0}
                    className="text-sm"
                  />
                  <Button size="icon" onClick={handleSendNLQuery} disabled={nlQueryLoading || !nlQuery.trim() || devices.length === 0}>
                    <Send className="h-4 w-4" />
                  </Button>
                </div>
                {devices.length === 0 && (
                  <p className="text-xs text-muted-foreground">No devices connected. Connect a device to query the agent.</p>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          {/* ─── Devices Tab ─────────────────────────────────────── */}
          <TabsContent value="devices" className="space-y-4 mt-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold flex items-center gap-2">
                <Laptop className="h-4 w-4 text-muted-foreground" />
                Devices
                {devices.length > 0 && <Badge variant="secondary" className="text-xs">{devices.length}</Badge>}
              </h3>
              <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={fetchDevices} disabled={devicesLoading}>
                <RefreshCw className={`h-3 w-3 mr-1 ${devicesLoading ? "animate-spin" : ""}`} />
                Refresh
              </Button>
            </div>

            {devicesError ? (
              <Card>
                <CardContent className="py-8 text-center">
                  <p className="text-sm text-muted-foreground">{devicesError}</p>
                  <Button variant="outline" size="sm" className="mt-2" onClick={fetchDevices}>
                    Retry
                  </Button>
                </CardContent>
              </Card>
            ) : devicesLoading ? (
              <div className="space-y-2">
                {[1, 2].map((i) => (
                  <Skeleton key={i} className="h-16 w-full rounded-lg" />
                ))}
              </div>
            ) : devices.length === 0 ? (
              <div className="flex items-center gap-3 py-8 text-muted-foreground">
                <CloudOff className="h-8 w-8 shrink-0" />
                <div>
                  <p className="text-sm font-medium">No devices enrolled yet</p>
                  <p className="text-xs">Share the install command from the Overview or Install Tokens tab to enroll a device.</p>
                </div>
              </div>
            ) : (
              <Card>
                <CardContent className="p-0">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Hostname</TableHead>
                        <TableHead>OS</TableHead>
                        <TableHead>Agent Version</TableHead>
                        <TableHead>Connection</TableHead>
                        <TableHead>Last Heartbeat</TableHead>
                        <TableHead>Status</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {devices.map((device) => (
                        <TableRow key={device.id}>
                          <TableCell className="font-medium">
                            <div className="flex items-center gap-2">
                              <Monitor className="h-4 w-4 text-muted-foreground shrink-0" />
                              {device.hostname || "Unnamed Device"}
                            </div>
                          </TableCell>
                          <TableCell>
                            <span className="text-sm">
                              {device.os_type === "linux" ? "Linux" : device.os_type === "macos" ? "macOS" : device.os_type === "windows" ? "Windows" : device.os_type}
                              {device.os_version ? ` ${device.os_version}` : ""}
                            </span>
                          </TableCell>
                          <TableCell className="font-mono text-xs">
                            {device.agent_version || "—"}
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center gap-2">
                              <ConnectionStatusDot status={device.connection_status} />
                              <ConnectionStatusBadge status={device.connection_status as "online" | "idle" | "offline"} />
                            </div>
                          </TableCell>
                          <TableCell className="text-sm text-muted-foreground">
                            {timeAgo(device.last_heartbeat)}
                          </TableCell>
                          <TableCell>
                            <DeviceStatusBadge status={device.status} />
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </CardContent>
              </Card>
            )}
          </TabsContent>

          {/* ─── Activity Tab ────────────────────────────────────── */}
          <TabsContent value="activity" className="space-y-4 mt-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold flex items-center gap-2">
                <BarChart3 className="h-4 w-4 text-muted-foreground" />
                Activity
              </h3>
            </div>

            {devices.length === 0 ? (
              <div className="flex items-center gap-3 py-8 text-muted-foreground">
                <CloudOff className="h-8 w-8 shrink-0" />
                <p className="text-sm">No devices to show activity for.</p>
              </div>
            ) : (
              <>
                <Select value={selectedDeviceId} onValueChange={setSelectedDeviceId}>
                  <SelectTrigger className="w-[300px]">
                    <SelectValue placeholder="Select a device" />
                  </SelectTrigger>
                  <SelectContent>
                    {devices.map((d) => (
                      <SelectItem key={d.id} value={d.id}>
                        {d.hostname || "Unnamed Device"} ({d.os_type})
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>

                {activityError ? (
                  <Card>
                    <CardContent className="py-8 text-center">
                      <p className="text-sm text-muted-foreground">{activityError}</p>
                      <Button variant="outline" size="sm" className="mt-2" onClick={() => selectedDeviceId && fetchActivity(selectedDeviceId)}>
                        Retry
                      </Button>
                    </CardContent>
                  </Card>
                ) : activityLoading ? (
                  <div className="space-y-3">
                    <div className="grid gap-4 md:grid-cols-2">
                      <Skeleton className="h-24 rounded-lg" />
                      <Skeleton className="h-24 rounded-lg" />
                    </div>
                    <Skeleton className="h-[300px] rounded-lg" />
                  </div>
                ) : (
                  <>
                    {/* Stats cards */}
                    <div className="grid gap-4 md:grid-cols-2">
                      <Card>
                        <CardHeader className="pb-2">
                          <CardDescription>Activity Summaries</CardDescription>
                        </CardHeader>
                        <CardContent>
                          <span className="text-2xl font-semibold">{activitySummaries.length}</span>
                        </CardContent>
                      </Card>
                      <Card>
                        <CardHeader className="pb-2">
                          <CardDescription>Unique Apps</CardDescription>
                        </CardHeader>
                        <CardContent>
                          <span className="text-2xl font-semibold">
                            {new Set(activityEvents.map((e) => e.app_name)).size}
                          </span>
                        </CardContent>
                      </Card>
                    </div>

                    {/* Activity summaries */}
                    {activitySummaries.length > 0 && (
                      <Card>
                        <CardHeader className="pb-2">
                          <CardTitle className="text-sm">Recent Summaries</CardTitle>
                        </CardHeader>
                        <CardContent className="space-y-3">
                          {activitySummaries.map((summary) => (
                            <div key={summary.id} className="border rounded-lg p-3 space-y-2">
                              <div className="flex items-center justify-between">
                                <span className="text-xs text-muted-foreground">
                                  {formatDate(summary.window_start)} — {timeAgo(summary.window_end)}
                                </span>
                                <Badge variant="secondary" className="text-xs">
                                  {summary.screenshot_count} screenshot{summary.screenshot_count !== 1 ? "s" : ""}
                                </Badge>
                              </div>
                              <p className="text-sm">{summary.summary_text}</p>
                              {summary.top_apps.length > 0 && (
                                <div className="flex flex-wrap gap-1">
                                  {summary.top_apps.map((app, idx) => (
                                    <Badge key={idx} variant="outline" className="text-xs">
                                      {app}
                                    </Badge>
                                  ))}
                                </div>
                              )}
                            </div>
                          ))}
                        </CardContent>
                      </Card>
                    )}

                    {/* Recent events */}
                    {activityEvents.length > 0 && (
                      <Card>
                        <CardHeader className="pb-2">
                          <CardTitle className="text-sm">Recent Events</CardTitle>
                        </CardHeader>
                        <CardContent className="p-0">
                          <Table>
                            <TableHeader>
                              <TableRow>
                                <TableHead>App</TableHead>
                                <TableHead>Window Title</TableHead>
                                <TableHead>Duration</TableHead>
                                <TableHead>Classification</TableHead>
                                <TableHead>Time</TableHead>
                              </TableRow>
                            </TableHeader>
                            <TableBody>
                              {activityEvents.slice(0, 20).map((event, idx) => (
                                <TableRow key={idx}>
                                  <TableCell className="font-medium text-sm">{event.app_name}</TableCell>
                                  <TableCell className="text-sm text-muted-foreground max-w-[200px] truncate">{event.window_title}</TableCell>
                                  <TableCell className="text-sm">{formatDuration(event.duration_sec)}</TableCell>
                                  <TableCell>
                                    <ClassificationBadge classification={event.classification} />
                                  </TableCell>
                                  <TableCell className="text-xs text-muted-foreground">{timeAgo(event.start_time)}</TableCell>
                                </TableRow>
                              ))}
                            </TableBody>
                          </Table>
                        </CardContent>
                      </Card>
                    )}

                    {activitySummaries.length === 0 && activityEvents.length === 0 && (
                      <div className="py-8 text-center text-muted-foreground">
                        <p className="text-sm">No activity data for this device.</p>
                      </div>
                    )}
                  </>
                )}
              </>
            )}
          </TabsContent>

          {/* ─── Screenshots Tab ────────────────────────────────── */}
          <TabsContent value="screenshots" className="space-y-4 mt-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold flex items-center gap-2">
                <Camera className="h-4 w-4 text-muted-foreground" />
                Screenshots
                {screenshots.length > 0 && (
                  <Badge variant="secondary" className="text-xs">
                    {screenshotFilter === "images"
                      ? screenshots.filter((s) => s.image_path).length
                      : screenshots.length}
                  </Badge>
                )}
              </h3>
              <div className="flex items-center gap-2">
                <Select value={screenshotFilter} onValueChange={(v: "all" | "images") => setScreenshotFilter(v)}>
                  <SelectTrigger className="h-7 text-xs w-[120px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="images">With images</SelectItem>
                    <SelectItem value="all">All screenshots</SelectItem>
                  </SelectContent>
                </Select>
                <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={fetchScreenshots} disabled={screenshotsLoading}>
                  <RefreshCw className={`h-3 w-3 mr-1 ${screenshotsLoading ? "animate-spin" : ""}`} />
                  Refresh
                </Button>
              </div>
            </div>

            {screenshotsError ? (
              <Card>
                <CardContent className="py-8 text-center">
                  <p className="text-sm text-muted-foreground">{screenshotsError}</p>
                  <Button variant="outline" size="sm" className="mt-2" onClick={fetchScreenshots}>
                    Retry
                  </Button>
                </CardContent>
              </Card>
            ) : screenshotsLoading && screenshots.length === 0 ? (
              <div className="space-y-2">
                <Skeleton className="h-20 w-full" />
                <Skeleton className="h-20 w-full" />
              </div>
            ) : (
              <>
                {/* Device screenshot buttons */}
                {devices.filter((d) => d.connection_status === "online" || d.connection_status === "idle").length > 0 && (
                  <div className="flex flex-wrap gap-2">
                    {devices
                      .filter((d) => d.connection_status === "online" || d.connection_status === "idle")
                      .map((d) => (
                        <Button
                          key={d.id}
                          variant="outline"
                          size="sm"
                          className="text-xs gap-1.5"
                          disabled={screenshotRequesting === d.id}
                          onClick={() => handleTakeScreenshot(d.id)}
                        >
                          {screenshotRequesting === d.id ? (
                            <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                          ) : (
                            <Camera className="h-3.5 w-3.5" />
                          )}
                          {d.hostname || "Unnamed Device"}
                        </Button>
                      ))}
                  </div>
                )}

                {screenshots.length === 0 ? (
                  <div className="flex items-center gap-3 py-8 text-muted-foreground">
                    <Camera className="h-8 w-8 shrink-0" />
                    <div>
                      <p className="text-sm font-medium">No screenshots yet</p>
                      <p className="text-xs">Take a screenshot from an online device above.</p>
                    </div>
                  </div>
                ) : (
                  <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
                    {screenshots
                      .filter((ss) => screenshotFilter === "all" || ss.image_path)
                      .map((ss) => {
                      const device = devices.find((d) => d.id === ss.device_id);
                      return (
                        <div key={ss.id} className="relative group rounded-lg border overflow-hidden bg-muted/30">
                          {ss.status === "pending" || (ss.status === "completed" && !ss.image_path && ss.policy === "upload_image") ? (
                            <div className="flex items-center justify-center h-40">
                              <div className="flex flex-col items-center gap-1">
                                <RefreshCw className={`h-5 w-5 text-muted-foreground${ss.status === "pending" ? " animate-spin" : ""}`} />
                                <span className="text-xs text-muted-foreground">
                                  {ss.status === "pending" ? "Capturing..." : "Awaiting image..."}
                                </span>
                              </div>
                            </div>
                          ) : ss.status === "completed" && ss.image_path ? (
                            <img
                              src={`/api/screenshots/${ss.id}?token=${token}`}
                              alt={`Screenshot ${ss.id}`}
                              className="w-full h-40 object-cover cursor-pointer hover:opacity-90 transition-opacity"
                              onClick={() => setViewingScreenshot(ss)}
                            />
                          ) : (
                            <div className="flex items-center justify-center h-40 text-muted-foreground text-xs">
                              <div className="flex flex-col items-center gap-1.5">
                                <ImageOff className="h-4 w-4" />
                                <span>{ss.policy === "metadata_only" ? "Metadata only" : ss.status}</span>
                              </div>
                            </div>
                          )}
                          <div className="absolute bottom-0 inset-x-0 bg-black/60 px-2 py-1.5 flex items-center justify-between">
                            <div className="flex items-center gap-1.5 min-w-0">
                              <Monitor className="h-3 w-3 text-white shrink-0" />
                              <span className="text-white text-[10px] truncate">
                                {device?.hostname || "Unknown"}
                              </span>
                            </div>
                            <div className="flex items-center gap-1.5 shrink-0">
                              <span className="text-white/70 text-[10px]">{timeAgo(ss.created_at)}</span>
                              <ScreenshotStatusBadge status={ss.status} />
                            </div>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </>
            )}

            {/* Full-size screenshot dialog */}
            {viewingScreenshot && (
              <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80" onClick={() => setViewingScreenshot(null)}>
                <div className="max-w-5xl w-full mx-4" onClick={(e) => e.stopPropagation()}>
                  <div className="bg-background rounded-lg overflow-hidden shadow-2xl">
                    <div className="flex items-center justify-between px-6 pt-4 pb-2">
                      <div>
                        <h3 className="font-semibold flex items-center gap-2">
                          Screenshot
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => copyToClipboard(`${window.location.origin}/api/screenshots/${viewingScreenshot.id}?token=${token}`, "Image URL")}
                          >
                            <Copy className="h-4 w-4" />
                          </Button>
                        </h3>
                        <p className="text-sm text-muted-foreground">
                          Taken {new Date(viewingScreenshot.created_at).toLocaleString()}
                        </p>
                      </div>
                      <Button variant="ghost" size="icon" onClick={() => setViewingScreenshot(null)}>
                        ✕
                      </Button>
                    </div>
                    <div className="px-6 pb-6">
                      <img
                        src={`/api/screenshots/${viewingScreenshot.id}?token=${token}`}
                        alt="Full screenshot"
                        className="w-full rounded-lg border"
                      />
                    </div>
                  </div>
                </div>
              </div>
            )}
          </TabsContent>

          {/* ─── Installed Apps Tab ────────────────────────────────── */}
          <TabsContent value="installed-apps" className="space-y-4 mt-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold flex items-center gap-2">
                <Package className="h-4 w-4 text-muted-foreground" />
                Installed Apps
              </h3>
              <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={fetchValidations} disabled={validationsLoading}>
                <RefreshCw className={`h-3 w-3 mr-1 ${validationsLoading ? "animate-spin" : ""}`} />
                Refresh
              </Button>
            </div>

            {validationsError ? (
              <Card>
                <CardContent className="py-8 text-center">
                  <p className="text-sm text-muted-foreground">{validationsError}</p>
                  <Button variant="outline" size="sm" className="mt-2" onClick={fetchValidations}>
                    Retry
                  </Button>
                </CardContent>
              </Card>
            ) : validationsLoading ? (
              <div className="space-y-3">
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                  {[1, 2, 3, 4].map((i) => (
                    <Skeleton key={i} className="h-24 rounded-lg" />
                  ))}
                </div>
                <Skeleton className="h-[300px] rounded-lg" />
              </div>
            ) : validations.length === 0 ? (
              <div className="py-8 text-center text-muted-foreground">
                <Package className="h-8 w-8 mx-auto mb-2" />
                <p className="text-sm">No app validation data available.</p>
              </div>
            ) : (
              <>
                {/* Stats cards */}
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                  <Card>
                    <CardHeader className="pb-2">
                      <CardDescription>Total Apps</CardDescription>
                    </CardHeader>
                    <CardContent>
                      <span className="text-2xl font-semibold">{validations.length}</span>
                    </CardContent>
                  </Card>
                  <Card>
                    <CardHeader className="pb-2">
                      <CardDescription className="flex items-center gap-1"><CheckCircle2 className="h-3.5 w-3.5 text-emerald-600" /> Compliant</CardDescription>
                    </CardHeader>
                    <CardContent>
                      <span className="text-2xl font-semibold text-emerald-600">{compliantCount}</span>
                    </CardContent>
                  </Card>
                  <Card>
                    <CardHeader className="pb-2">
                      <CardDescription className="flex items-center gap-1"><XCircle className="h-3.5 w-3.5 text-red-600" /> Non-compliant</CardDescription>
                    </CardHeader>
                    <CardContent>
                      <span className="text-2xl font-semibold text-red-600">{nonCompliantCount}</span>
                    </CardContent>
                  </Card>
                  <Card>
                    <CardHeader className="pb-2">
                      <CardDescription>Unique Apps</CardDescription>
                    </CardHeader>
                    <CardContent>
                      <span className="text-2xl font-semibold">{uniqueApps}</span>
                    </CardContent>
                  </Card>
                </div>

                {/* Table */}
                <Card>
                  <CardContent className="p-0">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>App Name</TableHead>
                          <TableHead>Display Name</TableHead>
                          <TableHead>Agent Category</TableHead>
                          <TableHead>Validated Category</TableHead>
                          <TableHead>Compliance</TableHead>
                          <TableHead>Reason</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {validations.map((v) => (
                          <TableRow key={v.id}>
                            <TableCell className="font-medium text-sm">{v.app_name}</TableCell>
                            <TableCell className="text-sm">{v.display_name || "—"}</TableCell>
                            <TableCell>
                              <ClassificationBadge classification={v.agent_category} />
                            </TableCell>
                            <TableCell>
                              <ClassificationBadge classification={v.validated_category} />
                            </TableCell>
                            <TableCell>
                              {v.is_compliant ? (
                                <Badge className="bg-emerald-100 text-emerald-800 hover:bg-emerald-100 border-emerald-200 border">
                                  <CheckCircle2 className="h-3 w-3 mr-1" />
                                  Compliant
                                </Badge>
                              ) : (
                                <Badge className="bg-red-100 text-red-800 hover:bg-red-100 border-red-200 border">
                                  <XCircle className="h-3 w-3 mr-1" />
                                  Non-compliant
                                </Badge>
                              )}
                            </TableCell>
                            <TableCell className="text-sm text-muted-foreground max-w-[200px] truncate">
                              {v.reason || "—"}
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </CardContent>
                </Card>
              </>
            )}
          </TabsContent>

          {/* ─── App Validations Tab ──────────────────────────────── */}
          <TabsContent value="app-validations" className="space-y-4 mt-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold flex items-center gap-2">
                <Shield className="h-4 w-4 text-muted-foreground" />
                App Validations
              </h3>
              <div className="flex items-center gap-2">
                <Select value={complianceFilter} onValueChange={(v: string) => setComplianceFilter(v as "all" | "compliant" | "noncompliant")}>
                  <SelectTrigger className="w-[160px] h-8 text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All</SelectItem>
                    <SelectItem value="compliant">Compliant only</SelectItem>
                    <SelectItem value="noncompliant">Non-compliant only</SelectItem>
                  </SelectContent>
                </Select>
                <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={fetchValidations} disabled={validationsLoading}>
                  <RefreshCw className={`h-3 w-3 mr-1 ${validationsLoading ? "animate-spin" : ""}`} />
                  Refresh
                </Button>
              </div>
            </div>

            {validationsError ? (
              <Card>
                <CardContent className="py-8 text-center">
                  <p className="text-sm text-muted-foreground">{validationsError}</p>
                  <Button variant="outline" size="sm" className="mt-2" onClick={fetchValidations}>
                    Retry
                  </Button>
                </CardContent>
              </Card>
            ) : validationsLoading ? (
              <Skeleton className="h-[300px] w-full rounded-lg" />
            ) : validations.length === 0 ? (
              <div className="py-8 text-center text-muted-foreground">
                <Shield className="h-8 w-8 mx-auto mb-2" />
                <p className="text-sm">No validation data available.</p>
              </div>
            ) : (
              <Card>
                <CardContent className="p-0">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>App Name</TableHead>
                        <TableHead>Device</TableHead>
                        <TableHead>Agent Category</TableHead>
                        <TableHead>Validated Category</TableHead>
                        <TableHead>Compliance</TableHead>
                        <TableHead>Reason</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {filteredValidations.map((v) => (
                        <TableRow key={v.id}>
                          <TableCell className="font-medium text-sm">{v.app_name}</TableCell>
                          <TableCell className="text-sm">{v.device_hostname || "—"}</TableCell>
                          <TableCell>
                            <ClassificationBadge classification={v.agent_category} />
                          </TableCell>
                          <TableCell>
                            <ClassificationBadge classification={v.validated_category} />
                          </TableCell>
                          <TableCell>
                            {v.is_compliant ? (
                              <Badge className="bg-emerald-100 text-emerald-800 hover:bg-emerald-100 border-emerald-200 border">
                                <CheckCircle2 className="h-3 w-3 mr-1" />
                                Compliant
                              </Badge>
                            ) : (
                              <Badge className="bg-red-100 text-red-800 hover:bg-red-100 border-red-200 border">
                                <XCircle className="h-3 w-3 mr-1" />
                                Non-compliant
                              </Badge>
                            )}
                          </TableCell>
                          <TableCell className="text-sm text-muted-foreground max-w-[200px] truncate">
                            {v.reason || "—"}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </CardContent>
              </Card>
            )}
          </TabsContent>

          {/* ─── Install Tokens Tab ──────────────────────────────── */}
          <TabsContent value="install-tokens" className="space-y-4 mt-4">
            <h3 className="text-sm font-semibold flex items-center gap-2">
              <KeyRound className="h-4 w-4 text-muted-foreground" />
              Install Token
            </h3>

            {tokenError && (
              <Card>
                <CardContent className="py-8 text-center">
                  <p className="text-sm text-muted-foreground">{tokenError}</p>
                  <Button variant="outline" size="sm" className="mt-2" onClick={fetchInstallToken}>
                    Retry
                  </Button>
                </CardContent>
              </Card>
            )}

            {tokenLoading ? (
              <div className="space-y-2">
                <Skeleton className="h-10 w-3/4 rounded-lg" />
                <Skeleton className="h-28 w-full rounded-lg" />
              </div>
            ) : installToken ? (
              <div className="space-y-4">
                {/* Token info card */}
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm">Token Details</CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    <div className="flex items-center gap-2 text-sm bg-muted/50 border rounded-lg px-3 py-2">
                      <span className="text-muted-foreground shrink-0">Token</span>
                      <code className="font-mono text-xs flex-1 truncate select-all">
                        {showFullToken ? installToken.token : maskToken(installToken.token)}
                      </code>
                      <div className="flex items-center gap-1 shrink-0">
                        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => setShowFullToken(!showFullToken)}>
                          {showFullToken ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
                        </Button>
                        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => copyToClipboard(installToken.token, "Token")}>
                          <Copy className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </div>

                    <div className="grid gap-3 md:grid-cols-2">
                      <div className="space-y-1">
                        <p className="text-xs text-muted-foreground">Status</p>
                        <TokenStatusBadge
                          status={
                            installToken.revoked_at
                              ? "revoked"
                              : installToken.expires_at && new Date(installToken.expires_at) < new Date()
                                ? "expired"
                                : "active"
                          }
                        />
                      </div>
                      <div className="space-y-1">
                        <p className="text-xs text-muted-foreground">Created</p>
                        <p className="text-sm">{formatDate(installToken.created_at)}</p>
                      </div>
                      <div className="space-y-1">
                        <p className="text-xs text-muted-foreground">Expires</p>
                        <p className="text-sm">
                          {installToken.expires_at ? formatDate(installToken.expires_at) : "Never"}
                        </p>
                      </div>
                    </div>
                  </CardContent>
                </Card>

                {/* Install commands */}
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm">Install Commands</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <Tabs value={installCmdTab} onValueChange={setInstallCmdTab}>
                      <TabsList className="h-8 w-full grid grid-cols-2">
                        <TabsTrigger value="linux" className="text-xs">Linux / macOS</TabsTrigger>
                        <TabsTrigger value="windows" className="text-xs">Windows</TabsTrigger>
                      </TabsList>
                      <TabsContent value="linux" className="mt-2">
                        <div className="relative group">
                          <pre className="bg-zinc-950 text-zinc-100 p-4 rounded-lg text-xs font-mono leading-relaxed overflow-x-auto pr-24">
                            <code>{linuxCommand}</code>
                          </pre>
                          <Button
                            size="sm"
                            className="absolute top-2 right-2 h-7 text-xs gap-1.5 opacity-80 group-hover:opacity-100 transition-opacity"
                            onClick={() => copyToClipboard(linuxCommand, "Linux install command")}
                          >
                            <Copy className="h-3 w-3" />
                            Copy
                          </Button>
                        </div>
                      </TabsContent>
                      <TabsContent value="windows" className="mt-2">
                        <div className="relative group">
                          <pre className="bg-zinc-950 text-zinc-100 p-4 rounded-lg text-xs font-mono leading-relaxed overflow-x-auto pr-24">
                            <code>{windowsCommand}</code>
                          </pre>
                          <Button
                            size="sm"
                            className="absolute top-2 right-2 h-7 text-xs gap-1.5 opacity-80 group-hover:opacity-100 transition-opacity"
                            onClick={() => copyToClipboard(windowsCommand, "Windows install command")}
                          >
                            <Copy className="h-3 w-3" />
                            Copy
                          </Button>
                        </div>
                      </TabsContent>
                    </Tabs>
                  </CardContent>
                </Card>
              </div>
            ) : (
              <Card>
                <CardContent className="py-12 flex flex-col items-center gap-3">
                  <KeyRound className="h-10 w-10 text-muted-foreground" />
                  <p className="text-sm text-muted-foreground">No install token generated yet</p>
                  <p className="text-xs text-muted-foreground">Generate a token to enroll a device for this employee.</p>
                  <Button onClick={fetchInstallToken} disabled={tokenLoading}>
                    {tokenLoading ? "Generating..." : "Generate Install Token"}
                  </Button>
                </CardContent>
              </Card>
            )}
          </TabsContent>

          {/* ─── Schedule Tab ──────────────────────────────────────── */}
          <TabsContent value="schedule" className="space-y-4 mt-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold flex items-center gap-2">
                <CalendarDays className="h-4 w-4 text-muted-foreground" />
                Screenshot Schedules
                {schedules.length > 0 && (
                  <Badge variant="secondary" className="text-xs">{schedules.length}</Badge>
                )}
              </h3>
              <div className="flex items-center gap-2">
                <Button
                  size="sm"
                  className="h-7 text-xs gap-1.5"
                  onClick={() => setCreateDialogOpen(true)}
                >
                  <Plus className="h-3.5 w-3.5" />
                  Create Schedule
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 text-xs"
                  onClick={fetchSchedules}
                  disabled={schedulesLoading}
                >
                  <RefreshCw className={`h-3 w-3 mr-1 ${schedulesLoading ? "animate-spin" : ""}`} />
                  Refresh
                </Button>
              </div>
            </div>

            {schedulesError ? (
              <Card>
                <CardContent className="py-8 text-center">
                  <p className="text-sm text-muted-foreground">{schedulesError}</p>
                  <Button variant="outline" size="sm" className="mt-2" onClick={fetchSchedules}>
                    Retry
                  </Button>
                </CardContent>
              </Card>
            ) : schedulesLoading ? (
              <div className="space-y-2">
                {[1, 2].map((i) => (
                  <Skeleton key={i} className="h-24 w-full rounded-lg" />
                ))}
              </div>
            ) : schedules.length === 0 ? (
              <Card>
                <CardContent className="py-12 flex flex-col items-center gap-3">
                  <CalendarDays className="h-10 w-10 text-muted-foreground" />
                  <p className="text-sm text-muted-foreground">No screenshot schedules yet</p>
                  <p className="text-xs text-muted-foreground">Create a schedule to automatically capture screenshots for this employee.</p>
                  <Button size="sm" onClick={() => setCreateDialogOpen(true)}>
                    Create Schedule
                  </Button>
                </CardContent>
              </Card>
            ) : (
              <div className="space-y-3">
                {schedules.map((schedule) => (
                  <Card key={schedule.id}>
                    <CardContent className="p-4">
                      <div className="flex items-start justify-between gap-4">
                        <div className="min-w-0 flex-1 space-y-1">
                          <div className="flex items-center gap-2 flex-wrap">
                            <span className="font-medium text-sm truncate">
                              {schedule.name || `Schedule ${schedule.id.slice(0, 8)}`}
                            </span>
                            <Badge variant={schedule.status === "active" ? "default" : schedule.status === "paused" ? "secondary" : "destructive"} className="text-xs">
                              {schedule.status === "active" ? "Active" : schedule.status === "paused" ? "Paused" : "Expired"}
                            </Badge>
                          </div>
                          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
                            <span className="flex items-center gap-1">
                              <Clock className="h-3 w-3" />
                              {schedule.cron_expression
                                ? `Cron: ${schedule.cron_expression}`
                                : `Every ${schedule.interval_minutes} min`}
                            </span>
                            <span className="flex items-center gap-1">
                              <Clock className="h-3 w-3" />
                              {schedule.start_time} - {schedule.end_time}
                            </span>
                            {(schedule.start_date || schedule.end_date) && (
                              <span className="flex items-center gap-1">
                                <CalendarDays className="h-3 w-3" />
                                {schedule.start_date ? formatDate(schedule.start_date) : "No start"}
                                {" — "}
                                {schedule.end_date ? formatDate(schedule.end_date) : "No end"}
                              </span>
                            )}
                            {schedule.last_triggered_at && (
                              <span className="flex items-center gap-1">
                                <Clock className="h-3 w-3" />
                                Last: {timeAgo(schedule.last_triggered_at)}
                              </span>
                            )}
                          </div>
                        </div>
                        <div className="flex items-center gap-1 shrink-0">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-7 w-7"
                            onClick={() => handleToggleScheduleStatus(schedule)}
                          >
                            {schedule.status === "active" ? (
                              <Pause className="h-3.5 w-3.5" />
                            ) : (
                              <Play className="h-3.5 w-3.5" />
                            )}
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-7 w-7 text-destructive hover:text-destructive"
                            onClick={() => handleDeleteSchedule(schedule.id)}
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </Button>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}

            {/* Create Schedule Dialog */}
            <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
              <DialogContent className="max-w-lg">
                <DialogHeader>
                  <DialogTitle className="flex items-center gap-2">
                    <Plus className="h-4 w-4" />
                    Create Screenshot Schedule
                  </DialogTitle>
                  <DialogDescription>
                    Choose a preset or configure a custom schedule for this employee.
                  </DialogDescription>
                </DialogHeader>
                <div className="space-y-4 mt-2">
                  {/* Presets */}
                  <div className="grid grid-cols-2 gap-2">
                    {[
                      { name: "Every 2 Hours", interval_minutes: 120, start_time: "09:00", end_time: "17:00" },
                      { name: "Every 30 Minutes", interval_minutes: 30, start_time: "09:00", end_time: "17:00" },
                      { name: "Hourly", interval_minutes: 60, start_time: "09:00", end_time: "17:00" },
                      { name: "Every 5 Minutes", interval_minutes: 5, start_time: "09:00", end_time: "17:00" },
                      { name: "Daily at 9 AM", interval_minutes: 1440, start_time: "09:00", end_time: "09:30" },
                      { name: "Twice Daily", interval_minutes: 480, start_time: "09:00", end_time: "17:00" },
                    ].map((preset) => (
                      <button
                        key={preset.name}
                        type="button"
                        onClick={() => applyPreset(preset)}
                        className={`text-left border rounded-lg p-3 text-sm transition-colors hover:bg-muted ${
                          newScheduleName === preset.name ? "border-primary bg-primary/5" : ""
                        }`}
                      >
                        <div className="font-medium">{preset.name}</div>
                        <div className="text-xs text-muted-foreground mt-0.5">
                          {preset.interval_minutes === 1440
                            ? preset.start_time === preset.end_time
                              ? `Daily at ${preset.start_time}`
                              : `${preset.start_time} - ${preset.end_time}`
                            : `Every ${preset.interval_minutes} min • ${preset.start_time} - ${preset.end_time}`}
                        </div>
                      </button>
                    ))}
                  </div>

                  {/* Form fields */}
                  <div className="space-y-3">
                    <div className="space-y-1">
                      <label className="text-xs font-medium">Name (optional)</label>
                      <Input
                        value={newScheduleName}
                        onChange={(e) => setNewScheduleName(e.target.value)}
                        placeholder="Schedule name"
                        className="text-sm h-8"
                      />
                    </div>
                    <div className="grid grid-cols-2 gap-3">
                      <div className="space-y-1">
                        <label className="text-xs font-medium">Start Time</label>
                        <Input
                          type="time"
                          value={newScheduleStartTime}
                          onChange={(e) => setNewScheduleStartTime(e.target.value)}
                          className="text-sm h-8"
                        />
                      </div>
                      <div className="space-y-1">
                        <label className="text-xs font-medium">End Time</label>
                        <Input
                          type="time"
                          value={newScheduleEndTime}
                          onChange={(e) => setNewScheduleEndTime(e.target.value)}
                          className="text-sm h-8"
                        />
                      </div>
                    </div>
                    <div className="grid grid-cols-2 gap-3">
                      <div className="space-y-1">
                        <label className="text-xs font-medium">Start Date</label>
                        <Input
                          type="date"
                          value={newScheduleStartDate}
                          onChange={(e) => setNewScheduleStartDate(e.target.value)}
                          className="text-sm h-8"
                        />
                      </div>
                      <div className="space-y-1">
                        <label className="text-xs font-medium">End Date</label>
                        <Input
                          type="date"
                          value={newScheduleEndDate}
                          onChange={(e) => setNewScheduleEndDate(e.target.value)}
                          className="text-sm h-8"
                        />
                      </div>
                    </div>

                    {/* Advanced Cron toggle */}
                    <div className="border rounded-lg">
                      <button
                        type="button"
                        onClick={() => setShowAdvancedCron((v) => !v)}
                        className="w-full flex items-center justify-between p-3 text-sm font-medium hover:bg-muted rounded-lg transition-colors"
                      >
                        <span>Advanced: Cron Expression</span>
                        {showAdvancedCron ? (
                          <ChevronUp className="h-4 w-4 text-muted-foreground" />
                        ) : (
                          <ChevronDown className="h-4 w-4 text-muted-foreground" />
                        )}
                      </button>
                      {showAdvancedCron && (
                        <div className="px-3 pb-3 space-y-2">
                          <div className="space-y-1">
                            <label className="text-xs font-medium">Cron Expression</label>
                            <Input
                              value={newScheduleCron}
                              onChange={(e) => setNewScheduleCron(e.target.value)}
                              placeholder="0 */2 * * *"
                              className="text-sm h-8 font-mono"
                            />
                            <p className="text-[11px] text-muted-foreground">
                              5 fields: minute hour day-of-month month day-of-week
                            </p>
                          </div>
                          <div className="space-y-1">
                            <p className="text-xs font-medium">Common examples:</p>
                            <div className="space-y-1">
                              {[
                                { label: "Every 2 hours", value: "0 */2 * * *" },
                                { label: "Weekdays at 9am", value: "0 9 * * 1-5" },
                                { label: "Every 30 min during work hours", value: "*/30 9-17 * * *" },
                                { label: "Monthly on 1st", value: "0 0 1 * *" },
                              ].map((example) => (
                                <button
                                  key={example.value}
                                  type="button"
                                  onClick={() => setNewScheduleCron(example.value)}
                                  className="w-full text-left border rounded-md px-2 py-1.5 text-xs hover:bg-muted transition-colors"
                                >
                                  <span className="font-mono text-primary">{example.value}</span>
                                  <span className="text-muted-foreground ml-2">{example.label}</span>
                                </button>
                              ))}
                            </div>
                          </div>
                        </div>
                      )}
                    </div>
                  </div>

                  <div className="flex justify-end gap-2 pt-2">
                    <Button variant="outline" size="sm" onClick={() => setCreateDialogOpen(false)}>
                      Cancel
                    </Button>
                    <Button
                      size="sm"
                      onClick={handleCreateSchedule}
                      disabled={creatingSchedule || (!newScheduleCron && newScheduleInterval <= 0)}
                    >
                      {creatingSchedule ? "Creating..." : "Create Schedule"}
                    </Button>
                  </div>
                </div>
              </DialogContent>
            </Dialog>
          </TabsContent>
        </Tabs>
      </div>
    </TooltipProvider>
  );
}