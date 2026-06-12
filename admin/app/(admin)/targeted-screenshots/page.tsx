"use client";

import { useEffect, useState, useCallback, useMemo } from "react";
import Link from "next/link";
import { DateRange } from "react-day-picker";
import { format, startOfDay, endOfDay, subDays } from "date-fns";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import { Calendar } from "@/components/ui/calendar";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Camera,
  RefreshCw,
  Monitor,
  ImageOff,
  Copy,
  X,
  Plus,
  Trash2,
  Pause,
  Play,
  Clock,
  CalendarIcon,
  User,
} from "lucide-react";
import { toast } from "sonner";
import { getCompanyId, getToken } from "@/lib/auth/session";
import {
  listEmployees,
  getEmployeeDevices,
  type Employee,
  type Device,
} from "@/lib/api/employees";
import {
  listTargetedSchedules,
  createTargetedSchedule,
  updateTargetedSchedule,
  deleteTargetedSchedule,
  getTargetedScreenshots,
  type TargetedScreenshotSchedule,
  type TargetedScreenshot,
} from "@/lib/api/targeted-schedules";
import { useSocket } from "@/lib/socket";
import { timeAgo, formatDate } from "@/lib/utils/format";
import { getScreenshotStatusBadgeVariant, getScreenshotStatusLabel } from "@/lib/utils/badges";

function getScheduleStatusBadgeVariant(
  status: string
): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "active":
      return "default";
    case "paused":
      return "secondary";
    case "expired":
      return "destructive";
    default:
      return "outline";
  }
}

function getScheduleStatusLabel(status: string): string {
  switch (status) {
    case "active":
      return "Active";
    case "paused":
      return "Paused";
    case "expired":
      return "Expired";
    default:
      return status;
  }
}

function getScreenshotImageUrl(requestId: string): string {
  const t =
    typeof window !== "undefined"
      ? localStorage.getItem("ainms_token") || ""
      : "";
  return `/api/screenshots/${requestId}?token=${encodeURIComponent(t)}`;
}

function formatTimeWindow(startTime: string, endTime: string): string {
  const fmt = (t: string) => {
    const [h, m] = t.split(":");
    return `${h}:${m}`;
  };
  return `${fmt(startTime)} - ${fmt(endTime)}`;
}

function formatDateRange(startDate: string, endDate: string): string {
  return `${formatDate(startDate)} — ${formatDate(endDate)}`;
}

export default function TargetedScreenshotsPage() {
  const companyId = getCompanyId();
  const token = getToken();
  const { isConnected, on } = useSocket(token);

  const [employees, setEmployees] = useState<Employee[]>([]);
  const [schedules, setSchedules] = useState<TargetedScreenshotSchedule[]>([]);
  const [screenshots, setScreenshots] = useState<TargetedScreenshot[]>([]);
  const [allDevices, setAllDevices] = useState<Device[]>([]);

  const [loadingSchedules, setLoadingSchedules] = useState(false);
  const [loadingScreenshots, setLoadingScreenshots] = useState(false);

  const [activeTab, setActiveTab] = useState("schedules");

  // Create schedule dialog state
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [newScheduleEmployeeId, setNewScheduleEmployeeId] = useState("");
  const [newScheduleName, setNewScheduleName] = useState("");
  const [newScheduleInterval, setNewScheduleInterval] = useState(5);
  const [newScheduleStartTime, setNewScheduleStartTime] = useState("09:00");
  const [newScheduleEndTime, setNewScheduleEndTime] = useState("17:00");
  const [newScheduleStartDate, setNewScheduleStartDate] = useState<Date | undefined>(
    new Date()
  );
  const [newScheduleEndDate, setNewScheduleEndDate] = useState<Date | undefined>(
    undefined
  );

  // Screenshot filters
  const [screenshotEmployeeFilter, setScreenshotEmployeeFilter] = useState<string>("all");
  const [screenshotScheduleFilter, setScreenshotScheduleFilter] = useState<string>("all");
  const [screenshotDateRange, setScreenshotDateRange] = useState<DateRange | undefined>(
    undefined
  );
  const [screenshotCalendarOpen, setScreenshotCalendarOpen] = useState(false);

  // Full-size screenshot dialog
  const [viewingScreenshot, setViewingScreenshot] = useState<TargetedScreenshot | null>(null);

  const employeeMap = useMemo(() => {
    const map = new Map<string, Employee>();
    for (const e of employees) map.set(e.id, e);
    return map;
  }, [employees]);

  const deviceMap = useMemo(() => {
    const map = new Map<string, Device>();
    for (const d of allDevices) map.set(d.id, d);
    return map;
  }, [allDevices]);

  const scheduleMap = useMemo(() => {
    const map = new Map<string, TargetedScreenshotSchedule>();
    for (const s of schedules) map.set(s.id, s);
    return map;
  }, [schedules]);

  const fetchEmployees = useCallback(async () => {
    if (!companyId) return;
    try {
      const empList = await listEmployees(companyId);
      setEmployees(empList);

      const devicesPerEmployee = await Promise.all(
        empList.map(async (emp) => {
          try {
            return await getEmployeeDevices(emp.id);
          } catch {
            return [];
          }
        })
      );
      setAllDevices(devicesPerEmployee.flat());
    } catch {
      setEmployees([]);
      setAllDevices([]);
    }
  }, [companyId]);

  const fetchSchedules = useCallback(async () => {
    if (!companyId) return;
    setLoadingSchedules(true);
    try {
      const data = await listTargetedSchedules(companyId);
      setSchedules(data || []);
    } catch {
      setSchedules([]);
    } finally {
      setLoadingSchedules(false);
    }
  }, [companyId]);

  const fetchScreenshots = useCallback(async () => {
    if (!companyId) return;
    setLoadingScreenshots(true);
    try {
      const params: Record<string, string> = { company_id: companyId };
      if (screenshotEmployeeFilter && screenshotEmployeeFilter !== "all") {
        params.employee_id = screenshotEmployeeFilter;
      }
      if (screenshotScheduleFilter && screenshotScheduleFilter !== "all") {
        params.schedule_id = screenshotScheduleFilter;
      }
      if (screenshotDateRange?.from) {
        params.from = startOfDay(screenshotDateRange.from).toISOString();
      }
      if (screenshotDateRange?.to) {
        params.to = endOfDay(screenshotDateRange.to).toISOString();
      }
      const data = await getTargetedScreenshots(params);
      const sorted = (data || []).sort(
        (a, b) =>
          new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      );
      setScreenshots(sorted);
    } catch {
      setScreenshots([]);
    } finally {
      setLoadingScreenshots(false);
    }
  }, [companyId, screenshotEmployeeFilter, screenshotScheduleFilter, screenshotDateRange]);

  useEffect(() => {
    if (companyId) {
      fetchEmployees();
    }
  }, [companyId, fetchEmployees]);

  useEffect(() => {
    if (companyId) {
      fetchSchedules();
    }
  }, [companyId, fetchSchedules]);

  useEffect(() => {
    if (companyId) {
      fetchScreenshots();
    }
  }, [companyId, fetchScreenshots]);

  useEffect(() => {
    const interval = setInterval(() => {
      fetchSchedules();
      fetchScreenshots();
    }, 30000);
    return () => clearInterval(interval);
  }, [fetchSchedules, fetchScreenshots]);

  useEffect(() => {
    const off = on(
      "screenshot_ready",
      (data: {
        request_id: string;
        device_id: string;
        status: string;
        image_path: string;
      }) => {
        fetchScreenshots();
      }
    );
    return () => {
      off();
    };
  }, [on, fetchScreenshots]);

  async function handleCreateSchedule() {
    if (!newScheduleEmployeeId) {
      toast.error("Please select an employee");
      return;
    }
    if (!newScheduleStartDate) {
      toast.error("Please select a start date");
      return;
    }
    setCreating(true);
    try {
      await createTargetedSchedule({
        company_id: companyId || undefined,
        employee_id: newScheduleEmployeeId,
        name: newScheduleName || undefined,
        interval_minutes: Math.max(1, newScheduleInterval),
        start_time: newScheduleStartTime,
        end_time: newScheduleEndTime,
        start_date: format(newScheduleStartDate, "yyyy-MM-dd"),
        end_date: newScheduleEndDate
          ? format(newScheduleEndDate, "yyyy-MM-dd")
          : undefined,
      });
      toast.success("Schedule created");
      setCreateDialogOpen(false);
      resetCreateForm();
      fetchSchedules();
    } catch {
      toast.error("Failed to create schedule");
    } finally {
      setCreating(false);
    }
  }

  function resetCreateForm() {
    setNewScheduleEmployeeId("");
    setNewScheduleName("");
    setNewScheduleInterval(5);
    setNewScheduleStartTime("09:00");
    setNewScheduleEndTime("17:00");
    setNewScheduleStartDate(new Date());
    setNewScheduleEndDate(undefined);
  }

  async function handleToggleScheduleStatus(schedule: TargetedScreenshotSchedule) {
    const nextStatus = schedule.status === "active" ? "paused" : "active";
    try {
      await updateTargetedSchedule(schedule.id, { status: nextStatus });
      toast.success(`Schedule ${nextStatus}`);
      fetchSchedules();
    } catch {
      toast.error("Failed to update schedule");
    }
  }

  async function handleDeleteSchedule(scheduleId: string) {
    if (!confirm("Delete this schedule? This cannot be undone.")) return;
    try {
      await deleteTargetedSchedule(scheduleId);
      toast.success("Schedule deleted");
      fetchSchedules();
    } catch {
      toast.error("Failed to delete schedule");
    }
  }

  function copyImageUrl(ss: TargetedScreenshot) {
    const url = `${window.location.origin}${getScreenshotImageUrl(ss.id)}`;
    navigator.clipboard
      .writeText(url)
      .then(() => toast.success("Image URL copied to clipboard"))
      .catch(() => toast.error("Failed to copy URL"));
  }

  const filteredScreenshots = useMemo(() => {
    let result = screenshots;
    if (screenshotEmployeeFilter && screenshotEmployeeFilter !== "all") {
      // Find all devices for this employee, filter screenshots by device_id
      const employeeDeviceIds = new Set(
        allDevices
          .filter((d) => d.employee_id === screenshotEmployeeFilter)
          .map((d) => d.id)
      );
      result = result.filter((s) => employeeDeviceIds.has(s.device_id));
    }
    if (screenshotScheduleFilter && screenshotScheduleFilter !== "all") {
      result = result.filter((s) => s.schedule_id === screenshotScheduleFilter);
    }
    return result;
  }, [screenshots, screenshotEmployeeFilter, screenshotScheduleFilter, allDevices]);

  const dateRangeLabel = (() => {
    if (!screenshotDateRange?.from) return "All time";
    if (
      screenshotDateRange.to &&
      screenshotDateRange.from.getTime() !== screenshotDateRange.to.getTime()
    ) {
      return `${format(screenshotDateRange.from, "MMM dd, yyyy")} — ${format(
        screenshotDateRange.to,
        "MMM dd, yyyy"
      )}`;
    }
    return format(screenshotDateRange.from, "MMM dd, yyyy");
  })();

  const quickRanges = [
    {
      label: "Today",
      from: startOfDay(new Date()),
      to: endOfDay(new Date()),
    },
    {
      label: "Last 7 days",
      from: startOfDay(subDays(new Date(), 6)),
      to: endOfDay(new Date()),
    },
    {
      label: "Last 30 days",
      from: startOfDay(subDays(new Date(), 29)),
      to: endOfDay(new Date()),
    },
  ];

  if (!companyId) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            Targeted Screenshots
          </h1>
          <p className="text-muted-foreground">
            Manage targeted screenshot schedules and browse captures.
          </p>
        </div>
        <Card>
          <CardContent className="py-12 text-center">
            <p className="text-muted-foreground">
              No company assigned. Contact a super admin.
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <TooltipProvider delayDuration={200}>
      <div className="space-y-6">
        <div className="flex items-center justify-between flex-wrap gap-4">
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">
              Targeted Screenshots
            </h1>
            <p className="text-muted-foreground">
              Manage schedules and view targeted screenshot captures.
            </p>
          </div>
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <span
                className={`relative inline-flex h-2 w-2 rounded-full ${
                  isConnected ? "bg-emerald-500" : "bg-red-500"
                }`}
              >
                {isConnected && (
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-500 opacity-40" />
                )}
              </span>
              {isConnected ? "Socket connected" : "Socket disconnected"}
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                fetchSchedules();
                fetchScreenshots();
              }}
              disabled={loadingSchedules || loadingScreenshots}
            >
              <RefreshCw
                className={`mr-2 h-4 w-4 ${
                  loadingSchedules || loadingScreenshots ? "animate-spin" : ""
                }`}
              />
              Refresh
            </Button>
            <Button size="sm" onClick={() => setCreateDialogOpen(true)}>
              <Plus className="mr-2 h-4 w-4" />
              Create Schedule
            </Button>
          </div>
        </div>

        <Tabs value={activeTab} onValueChange={setActiveTab}>
          <TabsList>
            <TabsTrigger value="schedules">
              <Clock className="mr-1.5 h-4 w-4" />
              Schedules
              <Badge variant="secondary" className="ml-2 text-[10px] px-1.5 py-0">
                {schedules.length}
              </Badge>
            </TabsTrigger>
            <TabsTrigger value="screenshots">
              <Camera className="mr-1.5 h-4 w-4" />
              Screenshots
              <Badge variant="secondary" className="ml-2 text-[10px] px-1.5 py-0">
                {screenshots.length}
              </Badge>
            </TabsTrigger>
          </TabsList>

          {/* ── Schedules Tab ── */}
          <TabsContent value="schedules" className="space-y-4">
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between flex-wrap gap-4">
                  <div>
                    <CardTitle>Targeted Screenshot Schedules</CardTitle>
                    <CardDescription>
                      {loadingSchedules
                        ? "Loading..."
                        : `${schedules.length} schedule${
                            schedules.length === 1 ? "" : "s"
                          }`}
                    </CardDescription>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                {loadingSchedules ? (
                  <div className="space-y-3">
                    {Array.from({ length: 4 }).map((_, i) => (
                      <div
                        key={i}
                        className="rounded-lg border p-4 space-y-2"
                      >
                        <Skeleton className="h-5 w-1/3" />
                        <Skeleton className="h-4 w-2/3" />
                        <Skeleton className="h-4 w-1/2" />
                      </div>
                    ))}
                  </div>
                ) : schedules.length === 0 ? (
                  <div className="flex flex-col items-center justify-center py-16 text-center">
                    <Clock className="h-12 w-12 text-muted-foreground mb-4" />
                    <p className="text-muted-foreground font-medium">
                      No schedules found
                    </p>
                    <p className="text-sm text-muted-foreground mt-1">
                      Create a schedule to start capturing targeted screenshots.
                    </p>
                  </div>
                ) : (
                  <div className="space-y-3">
                    {schedules.map((schedule) => {
                      const emp = employeeMap.get(schedule.employee_id);
                      return (
                        <div
                          key={schedule.id}
                          className="rounded-lg border bg-card p-4 flex items-start justify-between gap-4 flex-wrap sm:flex-nowrap"
                        >
                          <div className="space-y-1.5 min-w-0 flex-1">
                            <div className="flex items-center gap-2 flex-wrap">
                              <span className="font-semibold text-sm">
                                {schedule.name || "Untitled Schedule"}
                              </span>
                              <Badge
                                variant={getScheduleStatusBadgeVariant(
                                  schedule.status
                                )}
                                className="text-[10px] px-1.5 py-0"
                              >
                                {getScheduleStatusLabel(schedule.status)}
                              </Badge>
                            </div>
                            <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                              <User className="h-3 w-3 shrink-0" />
                              <span className="truncate">
                                {emp
                                  ? `${emp.first_name} ${emp.last_name}`
                                  : schedule.employee_id}
                              </span>
                            </div>
                            <div className="flex items-center gap-3 text-xs text-muted-foreground flex-wrap">
                              <span className="flex items-center gap-1">
                                <Clock className="h-3 w-3" />
                                Every {schedule.interval_minutes} min
                              </span>
                              <span className="flex items-center gap-1">
                                <CalendarIcon className="h-3 w-3" />
                                {formatTimeWindow(
                                  schedule.start_time,
                                  schedule.end_time
                                )}
                              </span>
                              <span>
                                {formatDateRange(
                                  schedule.start_date,
                                  schedule.end_date
                                )}
                              </span>
                            </div>
                            {schedule.last_triggered_at && (
                              <div className="text-xs text-muted-foreground">
                                Last triggered: {timeAgo(schedule.last_triggered_at)}
                              </div>
                            )}
                          </div>
                          <div className="flex items-center gap-1 shrink-0">
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-8 w-8"
                              onClick={() =>
                                handleToggleScheduleStatus(schedule)
                              }
                              title={
                                schedule.status === "active"
                                  ? "Pause schedule"
                                  : "Resume schedule"
                              }
                            >
                              {schedule.status === "active" ? (
                                <Pause className="h-4 w-4" />
                              ) : (
                                <Play className="h-4 w-4" />
                              )}
                            </Button>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-8 w-8 text-destructive"
                              onClick={() => handleDeleteSchedule(schedule.id)}
                              title="Delete schedule"
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          {/* ── Screenshots Tab ── */}
          <TabsContent value="screenshots" className="space-y-4">
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between flex-wrap gap-4">
                  <div>
                    <CardTitle>Captured Screenshots</CardTitle>
                    <CardDescription>
                      {loadingScreenshots
                        ? "Loading..."
                        : `${filteredScreenshots.length} screenshot${
                            filteredScreenshots.length === 1 ? "" : "s"
                          } found`}
                    </CardDescription>
                  </div>
                  <div className="flex items-center gap-2 flex-wrap">
                    <Popover
                      open={screenshotCalendarOpen}
                      onOpenChange={setScreenshotCalendarOpen}
                    >
                      <PopoverTrigger asChild>
                        <Button
                          variant="outline"
                          className={`justify-start text-left font-normal gap-2 ${
                            !screenshotDateRange?.from
                              ? "text-muted-foreground"
                              : ""
                          }`}
                        >
                          <CalendarIcon className="h-4 w-4" />
                          <span>{dateRangeLabel}</span>
                        </Button>
                      </PopoverTrigger>
                      <PopoverContent className="w-auto p-0" align="end">
                        <div className="flex flex-col gap-2 p-3 border-b">
                          <p className="text-sm font-medium">Quick select</p>
                          <div className="flex gap-2 flex-wrap">
                            {quickRanges.map((range) => (
                              <Button
                                key={range.label}
                                variant="outline"
                                size="sm"
                                onClick={() => {
                                  setScreenshotDateRange({
                                    from: range.from,
                                    to: range.to,
                                  });
                                  setScreenshotCalendarOpen(false);
                                }}
                              >
                                {range.label}
                              </Button>
                            ))}
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => {
                                setScreenshotDateRange(undefined);
                                setScreenshotCalendarOpen(false);
                              }}
                            >
                              All time
                            </Button>
                          </div>
                        </div>
                        <Calendar
                          mode="range"
                          selected={screenshotDateRange}
                          onSelect={(range) => {
                            setScreenshotDateRange(range ?? undefined);
                            if (range?.to) {
                              setScreenshotCalendarOpen(false);
                            }
                          }}
                          numberOfMonths={2}
                          defaultMonth={subDays(new Date(), 15)}
                        />
                      </PopoverContent>
                    </Popover>
                    {screenshotDateRange?.from && (
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => setScreenshotDateRange(undefined)}
                        title="Clear date filter"
                      >
                        <X className="h-4 w-4" />
                      </Button>
                    )}
                    <Select
                      value={screenshotEmployeeFilter}
                      onValueChange={setScreenshotEmployeeFilter}
                    >
                      <SelectTrigger className="w-56">
                        <SelectValue placeholder="Filter by employee" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">All Employees</SelectItem>
                        {employees.map((emp) => (
                          <SelectItem key={emp.id} value={emp.id}>
                            {emp.first_name} {emp.last_name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <Select
                      value={screenshotScheduleFilter}
                      onValueChange={setScreenshotScheduleFilter}
                    >
                      <SelectTrigger className="w-56">
                        <SelectValue placeholder="Filter by schedule" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">All Schedules</SelectItem>
                        {schedules.map((s) => (
                          <SelectItem key={s.id} value={s.id}>
                            {s.name || "Untitled"}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                {loadingScreenshots ? (
                  <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
                    {Array.from({ length: 8 }).map((_, i) => (
                      <div key={i} className="space-y-3">
                        <Skeleton className="aspect-video w-full rounded-lg" />
                        <Skeleton className="h-4 w-3/4" />
                        <Skeleton className="h-3 w-1/2" />
                      </div>
                    ))}
                  </div>
                ) : filteredScreenshots.length === 0 ? (
                  <div className="flex flex-col items-center justify-center py-16 text-center">
                    <ImageOff className="h-12 w-12 text-muted-foreground mb-4" />
                    <p className="text-muted-foreground font-medium">
                      No screenshots found
                    </p>
                    <p className="text-sm text-muted-foreground mt-1">
                      {screenshotEmployeeFilter !== "all" ||
                      screenshotScheduleFilter !== "all" ||
                      screenshotDateRange?.from
                        ? "Try adjusting your filters."
                        : "Schedules will generate screenshots automatically."}
                    </p>
                  </div>
                ) : (
                  <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
                    {filteredScreenshots.map((ss) => {
                      const device = deviceMap.get(ss.device_id);
                      const emp = device
                        ? employeeMap.get(device.employee_id)
                        : undefined;
                      const schedule = ss.schedule_id
                        ? scheduleMap.get(ss.schedule_id)
                        : undefined;
                      const isCompleted =
                        ss.status === "completed" && ss.image_path;
                      return (
                        <div
                          key={ss.id}
                          className="group relative rounded-lg border bg-card overflow-hidden hover:shadow-md transition-shadow"
                        >
                          <div
                            className="aspect-video bg-muted/30 cursor-pointer overflow-hidden"
                            onClick={() => setViewingScreenshot(ss)}
                          >
                            {ss.status === "pending" ? (
                              <div className="flex items-center justify-center h-full">
                                <div className="flex flex-col items-center gap-2">
                                  <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
                                  <span className="text-xs text-muted-foreground">
                                    Capturing...
                                  </span>
                                </div>
                              </div>
                            ) : isCompleted ? (
                              <img
                                src={getScreenshotImageUrl(ss.id)}
                                alt={`Screenshot from ${
                                  device?.hostname || "device"
                                }`}
                                className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                                crossOrigin="anonymous"
                              />
                            ) : (
                              <div className="flex items-center justify-center h-full">
                                <div className="flex flex-col items-center gap-2 text-muted-foreground">
                                  <X className="h-6 w-6" />
                                  <span className="text-xs">Failed</span>
                                </div>
                              </div>
                            )}
                          </div>

                          <div className="p-3 space-y-2">
                            <div className="flex items-center justify-between">
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <div className="flex items-center gap-1.5 min-w-0">
                                    <Monitor className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                                    <span className="text-sm font-medium truncate">
                                      {device?.hostname || "Unnamed Device"}
                                    </span>
                                  </div>
                                </TooltipTrigger>
                                <TooltipContent>
                                  <p>
                                    {device?.hostname || "Unnamed Device"}
                                  </p>
                                </TooltipContent>
                              </Tooltip>
                              <Badge
                                variant={getScreenshotStatusBadgeVariant(
                                  ss.status
                                )}
                                className="text-[10px] px-1.5 py-0.5 shrink-0"
                              >
                                {getScreenshotStatusLabel(ss.status)}
                              </Badge>
                            </div>

                            <div className="flex items-center justify-between text-xs text-muted-foreground">
                              {emp ? (
                                <Link
                                  href={`/employees/${emp.id}`}
                                  className="truncate hover:underline text-blue-600"
                                >
                                  {emp.first_name} {emp.last_name}
                                </Link>
                              ) : (
                                <span className="truncate">Unknown</span>
                              )}
                              <span className="shrink-0">
                                {timeAgo(ss.created_at)}
                              </span>
                            </div>

                            {schedule && (
                              <div className="flex items-center gap-1 text-[11px] text-muted-foreground">
                                <Clock className="h-3 w-3" />
                                <span className="truncate">
                                  {schedule.name || "Untitled Schedule"}
                                </span>
                              </div>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>

        {/* Full-size screenshot dialog */}
        <Dialog
          open={!!viewingScreenshot}
          onOpenChange={(open) => {
            if (!open) setViewingScreenshot(null);
          }}
        >
          <DialogContent className="max-w-5xl p-0 gap-0 overflow-hidden">
            <DialogHeader className="px-6 pt-6 pb-2">
              <div className="flex items-center justify-between">
                <div>
                  <DialogTitle className="flex items-center gap-2">
                    Screenshot
                    {viewingScreenshot && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8"
                        onClick={() => copyImageUrl(viewingScreenshot)}
                      >
                        <Copy className="h-4 w-4" />
                      </Button>
                    )}
                  </DialogTitle>
                  <DialogDescription>
                    {viewingScreenshot && (() => {
                      const device = deviceMap.get(viewingScreenshot.device_id);
                      const emp = device
                        ? employeeMap.get(device.employee_id)
                        : undefined;
                      const schedule = viewingScreenshot.schedule_id
                        ? scheduleMap.get(viewingScreenshot.schedule_id)
                        : undefined;
                      return (
                        <>
                          {device?.hostname || "Unnamed Device"} ·{" "}
                          {emp ? (
                            <Link
                              href={`/employees/${emp.id}`}
                              className="text-blue-600 hover:underline"
                            >
                              {emp.first_name} {emp.last_name}
                            </Link>
                          ) : (
                            "Unknown"
                          )}{" "}
                          · {new Date(viewingScreenshot.created_at).toLocaleString()}
                          {schedule && (
                            <span className="ml-1">
                              · {schedule.name || "Untitled Schedule"}
                            </span>
                          )}
                        </>
                      );
                    })()}
                  </DialogDescription>
                </div>
              </div>
            </DialogHeader>
            <div className="px-6 pb-6">
              {viewingScreenshot &&
                viewingScreenshot.status === "completed" &&
                viewingScreenshot.image_path && (
                  <img
                    src={getScreenshotImageUrl(viewingScreenshot.id)}
                    alt={`Screenshot from ${
                      deviceMap.get(viewingScreenshot.device_id)?.hostname ||
                      "device"
                    }`}
                    className="w-full rounded-lg border"
                    crossOrigin="anonymous"
                  />
                )}
            </div>
          </DialogContent>
        </Dialog>

        {/* Create schedule dialog */}
        <Dialog
          open={createDialogOpen}
          onOpenChange={(open) => {
            setCreateDialogOpen(open);
            if (!open) resetCreateForm();
          }}
        >
          <DialogContent className="sm:max-w-lg">
            <DialogHeader>
              <DialogTitle>Create Targeted Schedule</DialogTitle>
              <DialogDescription>
                Define a recurring screenshot window for a specific employee.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-2">
              <div className="space-y-1.5">
                <label className="text-sm font-medium">Employee</label>
                <Select
                  value={newScheduleEmployeeId}
                  onValueChange={setNewScheduleEmployeeId}
                >
                  <SelectTrigger>
                    <SelectValue placeholder="Select an employee..." />
                  </SelectTrigger>
                  <SelectContent>
                    {employees.length === 0 && (
                      <div className="px-2 py-4 text-sm text-muted-foreground text-center">
                        No employees available
                      </div>
                    )}
                    {employees.map((emp) => (
                      <SelectItem key={emp.id} value={emp.id}>
                        {emp.first_name} {emp.last_name}
                        <span className="ml-2 text-xs text-muted-foreground">
                          {emp.email}
                        </span>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-1.5">
                <label className="text-sm font-medium">Name (optional)</label>
                <input
                  type="text"
                  value={newScheduleName}
                  onChange={(e) => setNewScheduleName(e.target.value)}
                  placeholder="e.g. Daily check-in"
                  className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <label className="text-sm font-medium">
                    Interval (minutes)
                  </label>
                  <input
                    type="number"
                    min={1}
                    value={newScheduleInterval}
                    onChange={(e) =>
                      setNewScheduleInterval(Number(e.target.value))
                    }
                    className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                  />
                </div>
                <div className="space-y-1.5">
                  <label className="text-sm font-medium">Start time</label>
                  <input
                    type="time"
                    value={newScheduleStartTime}
                    onChange={(e) => setNewScheduleStartTime(e.target.value)}
                    className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                  />
                </div>
                <div className="space-y-1.5">
                  <label className="text-sm font-medium">End time</label>
                  <input
                    type="time"
                    value={newScheduleEndTime}
                    onChange={(e) => setNewScheduleEndTime(e.target.value)}
                    className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                  />
                </div>
                <div className="space-y-1.5">
                  <label className="text-sm font-medium">Start date</label>
                  <Popover>
                    <PopoverTrigger asChild>
                      <Button
                        variant="outline"
                        className={`w-full justify-start text-left font-normal gap-2 ${
                          !newScheduleStartDate
                            ? "text-muted-foreground"
                            : ""
                        }`}
                      >
                        <CalendarIcon className="h-4 w-4" />
                        <span>
                          {newScheduleStartDate
                            ? format(newScheduleStartDate, "MMM dd, yyyy")
                            : "Pick a date"}
                        </span>
                      </Button>
                    </PopoverTrigger>
                    <PopoverContent className="w-auto p-0" align="start">
                      <Calendar
                        mode="single"
                        selected={newScheduleStartDate}
                        onSelect={setNewScheduleStartDate}
                      />
                    </PopoverContent>
                  </Popover>
                </div>
                <div className="space-y-1.5">
                  <label className="text-sm font-medium">End date (optional)</label>
                  <Popover>
                    <PopoverTrigger asChild>
                      <Button
                        variant="outline"
                        className={`w-full justify-start text-left font-normal gap-2 ${
                          !newScheduleEndDate
                            ? "text-muted-foreground"
                            : ""
                        }`}
                      >
                        <CalendarIcon className="h-4 w-4" />
                        <span>
                          {newScheduleEndDate
                            ? format(newScheduleEndDate, "MMM dd, yyyy")
                            : "No end date"}
                        </span>
                      </Button>
                    </PopoverTrigger>
                    <PopoverContent className="w-auto p-0" align="start">
                      <Calendar
                        mode="single"
                        selected={newScheduleEndDate}
                        onSelect={setNewScheduleEndDate}
                      />
                    </PopoverContent>
                  </Popover>
                </div>
              </div>

              <div className="flex justify-end gap-2 pt-2">
                <Button
                  variant="outline"
                  onClick={() => setCreateDialogOpen(false)}
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleCreateSchedule}
                  disabled={
                    creating ||
                    !newScheduleEmployeeId ||
                    !newScheduleStartDate
                  }
                >
                  {creating ? (
                    <>
                      <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                      Creating...
                    </>
                  ) : (
                    <>
                      <Plus className="mr-2 h-4 w-4" />
                      Create Schedule
                    </>
                  )}
                </Button>
              </div>
            </div>
          </DialogContent>
        </Dialog>
      </div>
    </TooltipProvider>
  );
}
