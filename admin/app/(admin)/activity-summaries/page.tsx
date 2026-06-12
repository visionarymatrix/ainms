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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Calendar } from "@/components/ui/calendar";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
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
  AreaChart,
  Area,
} from "recharts";
import {
  listActivitySummaries,
  ActivitySummary,
} from "@/lib/api/activity-summaries";
import { api } from "@/lib/api/client";
import {
  Clock,
  Camera,
  Monitor,
  RefreshCw,
  BarChart3,
  CalendarIcon,
  X,
  AppWindow,
  Clock3,
  TrendingUp,
} from "lucide-react";
import { toast } from "sonner";
import { timeAgo, formatDuration } from "@/lib/utils/format";

interface Device {
  id: string;
  hostname: string | null;
  os_type: string;
  status: string;
  connection_status?: string;
  employee_id?: string;
}

interface AppUsageEvent {
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

const CLASSIFICATION_COLORS: Record<string, string> = {
  productive: "#22c55e",
  unproductive: "#ef4444",
  neutral: "#6b7280",
  unknown: "#9ca3af",
};

const APP_BAR_COLORS = [
  "hsl(var(--primary))",
  "hsl(210 80% 50%)",
  "hsl(160 60% 45%)",
  "hsl(30 80% 55%)",
  "hsl(280 65% 60%)",
  "hsl(340 75% 55%)",
  "#8b5cf6",
  "#f59e0b",
  "#06b6d4",
  "#84cc16",
];



function formatWindow(start: string, end: string): string {
  const s = new Date(start);
  const e = new Date(end);
  const fmt = (d: Date) =>
    d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  return `${fmt(s)} - ${fmt(e)}`;
}

function formatDateShort(date: Date): string {
  return format(date, "MMM dd, yyyy");
}



// ── Custom Tooltip: App Usage Bar Chart ──────────────────────────────────

interface AppUsageTooltipProps {
  active?: boolean;
  payload?: Array<{
    value: number;
    payload: {
      name: string;
      hours: number;
      windowTitles: string[];
      totalDuration: number;
      sessionCount: number;
    };
  }>;
}

function AppUsageTooltip({ active, payload }: AppUsageTooltipProps) {
  if (!active || !payload || payload.length === 0) return null;
  const data = payload[0].payload;
  return (
    <div className="rounded-lg border bg-background p-3 shadow-lg max-w-xs">
      <p className="font-semibold text-sm mb-1.5 flex items-center gap-1.5">
        <AppWindow className="h-3.5 w-3.5" />
        {data.name}
      </p>
      <div className="grid grid-cols-2 gap-x-3 gap-y-1 text-xs text-muted-foreground">
        <span>Total time:</span>
        <span className="font-medium text-foreground">
          {formatDuration(data.totalDuration)}
        </span>
        <span>Sessions:</span>
        <span className="font-medium text-foreground">
          {data.sessionCount}
        </span>
      </div>
      {data.windowTitles.length > 0 && (
        <div className="mt-2 pt-2 border-t">
          <p className="text-xs font-medium text-muted-foreground mb-1">
            Recent window titles:
          </p>
          <ul className="space-y-0.5">
            {data.windowTitles.slice(0, 5).map((title, i) => (
              <li key={i} className="text-xs truncate" title={title}>
                {title}
              </li>
            ))}
            {data.windowTitles.length > 5 && (
              <li className="text-xs text-muted-foreground">
                +{data.windowTitles.length - 5} more
              </li>
            )}
          </ul>
        </div>
      )}
    </div>
  );
}

// ── Custom Tooltip: Activity Timeline ───────────────────────────────────

interface TimelineTooltipProps {
  active?: boolean;
  payload?: Array<{
    value: number;
    payload: {
      time: string;
      timeLabel: string;
      apps: {
        name: string;
        duration: number;
        titles: string[];
      }[];
      totalMinutes: number;
    };
  }>;
}

function TimelineTooltip({ active, payload }: TimelineTooltipProps) {
  if (!active || !payload || payload.length === 0) return null;
  const data = payload[0].payload;
  return (
    <div className="rounded-lg border bg-background p-3 shadow-lg max-w-xs">
      <p className="font-semibold text-sm mb-1.5">{data.timeLabel}</p>
      <p className="text-xs text-muted-foreground mb-1.5">
        Total: {formatDuration(data.totalMinutes * 60)}
      </p>
      <div className="space-y-1">
        {data.apps.slice(0, 4).map((app, i) => (
          <div key={i} className="flex items-center gap-1.5 text-xs">
            <span
              className="h-2.5 w-2.5 rounded-sm shrink-0"
              style={{
                backgroundColor:
                  APP_BAR_COLORS[i % APP_BAR_COLORS.length],
              }}
            />
            <span className="truncate font-medium">{app.name}</span>
            <span className="text-muted-foreground">
              ({formatDuration(app.duration)})
            </span>
          </div>
        ))}
        {data.apps.length > 4 && (
          <p className="text-xs text-muted-foreground">
            +{data.apps.length - 4} more apps
          </p>
        )}
      </div>
      {data.apps.some((a) => a.titles.length > 0) && (
        <div className="mt-1.5 pt-1.5 border-t text-xs text-muted-foreground">
          {data.apps.slice(0, 3).flatMap((app) =>
            app.titles.slice(0, 2).map((t, j) => (
              <p
                key={`${app.name}-${j}`}
                className="truncate"
                title={t}
              >
                {app.name}: {t}
              </p>
            ))
          )}
        </div>
      )}
    </div>
  );
}

// ── Main Component ───────────────────────────────────────────────────────

export default function ActivitySummariesPage() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string>("");
  const [summaries, setSummaries] = useState<ActivitySummary[]>([]);
  const [events, setEvents] = useState<AppUsageEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [dataLoading, setDataLoading] = useState(false);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [dateRange, setDateRange] = useState<DateRange | undefined>(
    undefined
  );
  const [calendarOpen, setCalendarOpen] = useState(false);

  const fetchDevices = useCallback(async () => {
    try {
      const data = await api.get<Device[]>("/v1/devices/status");
      const activeDevices = (data || []).filter((d) => d.status === "active");
      setDevices(activeDevices);
      if (activeDevices.length > 0 && !selectedDevice) {
        setSelectedDevice(activeDevices[0].id);
      }
    } catch (err) {
      toast.error("Failed to load devices");
    } finally {
      setLoading(false);
    }
  }, [selectedDevice]);

  const fetchData = useCallback(async () => {
    if (!selectedDevice) return;
    setDataLoading(true);
    try {
      const filters: Record<string, string> = {};
      if (dateRange?.from) {
        filters.from = startOfDay(dateRange.from).toISOString();
      }
      if (dateRange?.to) {
        filters.to = endOfDay(dateRange.to).toISOString();
      }

      const [summaryData, eventData] = await Promise.all([
        listActivitySummaries(selectedDevice, {
          limit: 500,
          from: filters.from,
          to: filters.to,
        }),
        api
          .get<AppUsageEvent[]>(
            `/v1/devices/${selectedDevice}/events`,
            Object.keys(filters).length > 0
              ? { limit: "2000", ...filters }
              : { limit: "2000" }
          )
          .catch(() => []),
      ]);
      setSummaries(summaryData || []);
      setEvents((eventData || []) as AppUsageEvent[]);
    } catch (err) {
      toast.error("Failed to load activity data");
      setSummaries([]);
      setEvents([]);
    } finally {
      setDataLoading(false);
    }
  }, [selectedDevice, dateRange]);

  useEffect(() => {
    fetchDevices();
  }, [fetchDevices]);

  useEffect(() => {
    if (selectedDevice) {
      fetchData();
    }
  }, [selectedDevice, fetchData]);

  useEffect(() => {
    const interval = setInterval(() => {
      if (selectedDevice) {
        fetchData();
      }
    }, 30000);
    return () => clearInterval(interval);
  }, [selectedDevice, fetchData]);

  // ── Chart data: top apps by duration with window titles ──

  const appUsageData = useMemo(() => {
    if (events.length === 0) return [];
    const appMap = new Map<
      string,
      {
        name: string;
        totalDuration: number;
        sessionCount: number;
        titles: Set<string>;
      }
    >();
    for (const e of events) {
      // Poll-sampled events may have 0 duration — treat as 1 sample = ~10s activity
      const effectiveDuration = e.duration_sec > 0 ? e.duration_sec : 10;
      const key = e.app_name;
      const existing = appMap.get(key);
      if (existing) {
        existing.totalDuration += effectiveDuration;
        existing.sessionCount += 1;
        if (e.window_title) existing.titles.add(e.window_title);
      } else {
        appMap.set(key, {
          name: e.app_name,
          totalDuration: effectiveDuration,
          sessionCount: 1,
          titles: new Set(e.window_title ? [e.window_title] : []),
        });
      }
    }
    return Array.from(appMap.values())
      .sort((a, b) => b.totalDuration - a.totalDuration)
      .slice(0, 15)
      .map((d) => ({
        name: d.name,
        hours: Number((d.totalDuration / 3600).toFixed(2)),
        totalDuration: d.totalDuration,
        sessionCount: d.sessionCount,
        windowTitles: Array.from(d.titles).slice(0, 10),
      }));
  }, [events]);

  // ── Chart data: classification breakdown ──

  const classificationData = useMemo(() => {
    if (events.length === 0) return [];
    const classMap = new Map<string, number>();
    for (const e of events) {
      const cls = e.classification || "unknown";
      const effectiveDuration = e.duration_sec > 0 ? e.duration_sec : 10;
      classMap.set(cls, (classMap.get(cls) || 0) + effectiveDuration);
    }
    return Array.from(classMap.entries())
      .map(([name, value]) => ({
        name: name.charAt(0).toUpperCase() + name.slice(1),
        value,
        color: CLASSIFICATION_COLORS[name] || CLASSIFICATION_COLORS.unknown,
      }))
      .sort((a, b) => b.value - a.value);
  }, [events]);

  // ── Chart data: activity timeline (sessions grouped into hourly buckets) ──

  const timelineData = useMemo(() => {
    if (events.length === 0) return [];
    const bucketMap = new Map<
      string,
      {
        totalMinutes: number;
        apps: Map<string, { duration: number; titles: Set<string> }>;
      }
    >();

    for (const e of events) {
      const start = new Date(e.start_time);
      const bucketKey = format(start, "yyyy-MM-dd HH:00");
      if (!bucketMap.has(bucketKey)) {
        bucketMap.set(bucketKey, {
          totalMinutes: 0,
          apps: new Map(),
        });
      }
      const bucket = bucketMap.get(bucketKey)!;
      const effectiveDuration = e.duration_sec > 0 ? e.duration_sec : 10;
      bucket.totalMinutes += effectiveDuration / 60;
      const appName = e.app_name;
      const appEntry = bucket.apps.get(appName) || {
        duration: 0,
        titles: new Set<string>(),
      };
      appEntry.duration += effectiveDuration;
      if (e.window_title) appEntry.titles.add(e.window_title);
      bucket.apps.set(appName, appEntry);
    }

    return Array.from(bucketMap.entries())
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([timeKey, bucket]) => ({
        time: timeKey,
        timeLabel: format(new Date(timeKey), "MMM dd, HH:mm"),
        totalMinutes: Number(bucket.totalMinutes.toFixed(1)),
        apps: Array.from(bucket.apps.entries())
          .map(([name, data]) => ({
            name,
            duration: data.duration,
            titles: Array.from(data.titles).slice(0, 3),
          }))
          .sort((a, b) => b.duration - a.duration),
      }));
  }, [events]);

  // ── Stats ──
  const totalSummaries = summaries.length;
  const totalScreenshots = summaries.reduce(
    (sum, s) => sum + s.screenshot_count,
    0
  );
  const totalTrackedHours =
    events.length > 0
      ? (events.reduce((sum, e) => sum + (e.duration_sec > 0 ? e.duration_sec : 10), 0) / 3600).toFixed(1)
      : "0";
  const uniqueApps =
    events.length > 0
      ? new Set(events.map((e) => e.app_name)).size
      : 0;
  const latestSummary =
    summaries.length > 0 ? timeAgo(summaries[0].window_start) : "Never";

  const selectedDeviceInfo = devices.find((d) => d.id === selectedDevice);

  const dateRangeLabel = (() => {
    if (!dateRange?.from) return "All time";
    if (
      dateRange.to &&
      dateRange.from.getTime() !== dateRange.to.getTime()
    ) {
      return `${formatDateShort(dateRange.from)} — ${formatDateShort(
        dateRange.to
      )}`;
    }
    return formatDateShort(dateRange.from);
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

  if (loading) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-semibold tracking-tight">
          Activity Summaries
        </h1>
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
      <div className="flex items-center justify-between flex-wrap gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            Activity Summaries
          </h1>
          <p className="text-muted-foreground">
            AI-generated activity summaries with app usage charts and timeline.
          </p>
        </div>
        <div className="flex items-center gap-3 flex-wrap">
          <Popover open={calendarOpen} onOpenChange={setCalendarOpen}>
            <PopoverTrigger asChild>
              <Button
                variant="outline"
                className={`justify-start text-left font-normal gap-2 ${
                  !dateRange?.from ? "text-muted-foreground" : ""
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
                        setDateRange({ from: range.from, to: range.to });
                        setCalendarOpen(false);
                      }}
                    >
                      {range.label}
                    </Button>
                  ))}
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      setDateRange(undefined);
                      setCalendarOpen(false);
                    }}
                  >
                    All time
                  </Button>
                </div>
              </div>
              <Calendar
                mode="range"
                selected={dateRange}
                onSelect={(range) => {
                  setDateRange(range ?? undefined);
                  if (range?.to) {
                    setCalendarOpen(false);
                  }
                }}
                numberOfMonths={2}
                defaultMonth={subDays(new Date(), 15)}
              />
            </PopoverContent>
          </Popover>
          {dateRange?.from && (
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setDateRange(undefined)}
              title="Clear date filter"
            >
              <X className="h-4 w-4" />
            </Button>
          )}
          <Select value={selectedDevice} onValueChange={setSelectedDevice}>
            <SelectTrigger className="w-64">
              <SelectValue placeholder="Select device" />
            </SelectTrigger>
            <SelectContent>
              {devices.map((device) => (
                <SelectItem key={device.id} value={device.id}>
                  {device.hostname || device.id.slice(0, 8)}
                  <span className="ml-2 text-xs text-muted-foreground">
                    ({device.os_type})
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button variant="outline" size="sm" onClick={fetchData}>
            <RefreshCw className="mr-1 h-3 w-3" />
            Refresh
          </Button>
        </div>
      </div>

      {selectedDevice && (
        <>
          {/* Stats Cards */}
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                  Total Summaries
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{totalSummaries}</div>
                <p className="text-xs text-muted-foreground mt-1">
                  5-min windows
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
                  <Clock3 className="h-3.5 w-3.5" />
                  Tracked Hours
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {totalTrackedHours}h
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  from event data
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
                  <AppWindow className="h-3.5 w-3.5" />
                  Unique Apps
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{uniqueApps}</div>
                <p className="text-xs text-muted-foreground mt-1">
                  distinct applications
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                  Screenshots
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{totalScreenshots}</div>
                <p className="text-xs text-muted-foreground mt-1">
                  across all summaries
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                  Latest Summary
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{latestSummary}</div>
                <p className="text-xs text-muted-foreground mt-1">
                  {selectedDeviceInfo?.hostname || "Unknown"} &middot;{" "}
                  <Badge
                    variant={
                      selectedDeviceInfo?.connection_status === "online"
                        ? "default"
                        : "secondary"
                    }
                    className="text-xs"
                  >
                    {selectedDeviceInfo?.connection_status || "unknown"}
                  </Badge>
                </p>
                {selectedDeviceInfo?.employee_id && (
                  <Link
                    href={`/employees/${selectedDeviceInfo.employee_id}`}
                    className="text-xs text-blue-600 hover:underline mt-1 inline-block"
                  >
                    View employee profile →
                  </Link>
                )}
              </CardContent>
            </Card>
          </div>

          {/* Charts */}
          {events.length > 0 && (
            <div className="grid gap-4 lg:grid-cols-3">
              {/* Top Apps Bar Chart */}
              <Card className="lg:col-span-2">
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <BarChart3 className="h-5 w-5" />
                    Most Used Applications
                  </CardTitle>
                  <CardDescription>
                    Total time per app · hover for window titles
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="h-80">
                    <ResponsiveContainer width="100%" height="100%">
                      <BarChart
                        data={appUsageData}
                        layout="vertical"
                        margin={{ top: 4, right: 16, bottom: 4, left: 80 }}
                      >
                        <CartesianGrid
                          strokeDasharray="3 3"
                          horizontal={false}
                        />
                        <XAxis
                          type="number"
                          tick={{ fontSize: 11 }}
                          tickFormatter={(v: number) => `${v}h`}
                        />
                        <YAxis
                          type="category"
                          dataKey="name"
                          tick={{ fontSize: 11 }}
                          width={76}
                          interval={0}
                        />
                        <RechartsTooltip content={<AppUsageTooltip />} />
                        <Bar
                          dataKey="hours"
                          radius={[0, 4, 4, 0]}
                          maxBarSize={28}
                        >
                          {appUsageData.map((_entry, index) => (
                            <Cell
                              key={`cell-${index}`}
                              fill={
                                APP_BAR_COLORS[
                                  index % APP_BAR_COLORS.length
                              ]
                              }
                            />
                          ))}
                        </Bar>
                      </BarChart>
                    </ResponsiveContainer>
                  </div>
                </CardContent>
              </Card>

              {/* Classification Pie Chart */}
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <TrendingUp className="h-5 w-5" />
                    Classification
                  </CardTitle>
                  <CardDescription>
                    Productive vs unproductive vs neutral
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="h-80">
                    {classificationData.length > 0 ? (
                      <ResponsiveContainer width="100%" height="100%">
                        <PieChart>
                          <Pie
                            data={classificationData}
                            dataKey="value"
                            nameKey="name"
                            cx="50%"
                            cy="45%"
                            innerRadius={55}
                            outerRadius={85}
                            label={(props: any) =>
                              `${props.name ?? ""} ${((props.percent ?? 0) * 100).toFixed(0)}%`
                            }
                            labelLine
                          >
                            {classificationData.map((entry, index) => (
                              <Cell
                                key={`cell-${index}`}
                                fill={entry.color}
                              />
                            ))}
                          </Pie>
                          <RechartsTooltip
                            formatter={(value: any) =>
                              formatDuration(value as number)
                            }
                            contentStyle={{
                              borderRadius: "6px",
                              fontSize: 12,
                            }}
                          />
                          <Legend
                            verticalAlign="bottom"
                            height={36}
                            wrapperStyle={{ fontSize: 12 }}
                          />
                        </PieChart>
                      </ResponsiveContainer>
                    ) : (
                      <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
                        No classification data
                      </div>
                    )}
                  </div>
                </CardContent>
              </Card>
            </div>
          )}

          {/* Activity Timeline */}
          {timelineData.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Clock className="h-5 w-5" />
                  Activity Timeline
                </CardTitle>
                <CardDescription>
                  Hourly activity levels · hover for details
                  {dateRange?.from && (
                    <span className="ml-1 text-primary">
                      {" "}
                      — {dateRangeLabel}
                    </span>
                  )}
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="h-64">
                  <ResponsiveContainer width="100%" height="100%">
                    <AreaChart
                      data={timelineData}
                      margin={{ top: 4, right: 16, bottom: 4, left: 8 }}
                    >
                      <CartesianGrid
                        strokeDasharray="3 3"
                        vertical={false}
                      />
                      <XAxis
                        dataKey="timeLabel"
                        tick={{ fontSize: 11 }}
                        interval="preserveStartEnd"
                      />
                      <YAxis
                        tick={{ fontSize: 11 }}
                        label={{
                          value: "Minutes",
                          angle: -90,
                          position: "insideLeft",
                          style: { fontSize: 11 },
                        }}
                      />
                      <RechartsTooltip content={<TimelineTooltip />} />
                      <Area
                        type="monotone"
                        dataKey="totalMinutes"
                        stroke="hsl(var(--primary))"
                        fill="hsl(var(--primary))"
                        fillOpacity={0.15}
                        strokeWidth={2}
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Summaries Table */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <BarChart3 className="h-5 w-5" />
                AI Activity Summaries
              </CardTitle>
              <CardDescription>
                AI-generated summaries of user activity per 5-minute
                window
                {dateRange?.from && (
                  <span className="ml-1 text-primary">
                    {" "}
                    — {dateRangeLabel}
                  </span>
                )}
              </CardDescription>
            </CardHeader>
            <CardContent>
              {dataLoading ? (
                <div className="space-y-2">
                  {[1, 2, 3, 4, 5].map((i) => (
                    <Skeleton key={i} className="h-16" />
                  ))}
                </div>
              ) : summaries.length === 0 ? (
                <p className="text-muted-foreground text-sm py-8 text-center">
                  No activity summaries available for this device
                  {dateRange?.from
                    ? " in the selected date range"
                    : " yet"}
                  .{" "}
                  Summaries are generated after 5 minutes of active
                  use.
                </p>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Window</TableHead>
                      <TableHead>Summary</TableHead>
                      <TableHead>Top Apps</TableHead>
                      <TableHead className="text-center">
                        Screenshots
                      </TableHead>
                      <TableHead className="text-right">
                        Recorded
                      </TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {summaries.map((s) => (
                      <TableRow
                        key={s.id}
                        className="cursor-pointer hover:bg-muted/50"
                        onClick={() =>
                          setExpandedId(
                            expandedId === s.id ? null : s.id
                          )
                        }
                      >
                        <TableCell className="font-medium whitespace-nowrap">
                          <Clock className="inline h-3 w-3 mr-1 text-muted-foreground" />
                          {formatWindow(s.window_start, s.window_end)}
                        </TableCell>
                        <TableCell className="max-w-[400px]">
                          <p
                            className={
                              expandedId === s.id ? "" : "truncate"
                            }
                          >
                            {s.summary_text}
                          </p>
                        </TableCell>
                        <TableCell>
                          <div className="flex flex-wrap gap-1">
                            {s.top_apps.slice(0, 3).map((app) => (
                              <Badge
                                key={app}
                                variant="secondary"
                                className="text-xs"
                              >
                                {app}
                              </Badge>
                            ))}
                            {s.top_apps.length > 3 && (
                              <Badge
                                variant="outline"
                                className="text-xs"
                              >
                                +{s.top_apps.length - 3}
                              </Badge>
                            )}
                          </div>
                        </TableCell>
                        <TableCell className="text-center">
                          <Camera className="inline h-3 w-3 mr-1 text-muted-foreground" />
                          {s.screenshot_count}
                        </TableCell>
                        	<TableCell className="text-right text-muted-foreground text-sm whitespace-nowrap">
                          	{timeAgo(s.window_start)}
                        	</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>
        </>
      )}

      {!selectedDevice && devices.length === 0 && (
        <Card>
          <CardContent className="py-12 text-center">
            <Monitor className="mx-auto h-12 w-12 text-muted-foreground" />
            <h3 className="mt-4 text-lg font-semibold">
              No active devices
            </h3>
            <p className="mt-2 text-sm text-muted-foreground">
              Enroll a device to start seeing activity summaries.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}