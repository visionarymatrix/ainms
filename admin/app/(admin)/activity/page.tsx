"use client";

import { useEffect, useState, useCallback } from "react";
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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api } from "@/lib/api/client";
import { getUser } from "@/lib/auth/session";
import {
  Activity,
  Clock,
  TrendingUp,
  Monitor,
  RefreshCw,
  ArrowUpDown,
} from "lucide-react";
import { toast } from "sonner";
import { timeAgo, formatDuration } from "@/lib/utils/format";
import { getClassificationBadge } from "@/lib/utils/badges";

interface Device {
  id: string;
  hostname: string | null;
  os_type: string;
  status: string;
  connection_status?: string;
  employee_id?: string;
}

interface UsageSummary {
  device_id: string;
  app_name: string;
  total_duration_sec: number;
  session_count: number;
  productive_duration_sec: number;
  unproductive_duration_sec: number;
  neutral_duration_sec: number;
}

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



export default function ActivityPage() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string>("");
  const [summaries, setSummaries] = useState<UsageSummary[]>([]);
  const [events, setEvents] = useState<AppUsageEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [eventsLoading, setEventsLoading] = useState(false);
  const [summarySort, setSummarySort] = useState<"duration" | "sessions" | "productive">("duration");
  const user = getUser();

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

  const fetchSummaries = useCallback(async () => {
    if (!selectedDevice) return;
    setEventsLoading(true);
    try {
      const data = await api.get<UsageSummary[]>(
        `/v1/devices/${selectedDevice}/usage-summaries`
      );
      setSummaries(data || []);
    } catch (err) {
      toast.error("Failed to load usage summaries");
      setSummaries([]);
    } finally {
      setEventsLoading(false);
    }
  }, [selectedDevice]);

  const fetchEvents = useCallback(async () => {
    if (!selectedDevice) return;
    try {
      const data = await api.get<AppUsageEvent[]>(
        `/v1/devices/${selectedDevice}/events?limit=200`
      );
      setEvents(data || []);
    } catch (err) {
      toast.error("Failed to load events");
      setEvents([]);
    }
  }, [selectedDevice]);

  useEffect(() => {
    fetchDevices();
  }, [fetchDevices]);

  useEffect(() => {
    if (selectedDevice) {
      fetchSummaries();
      fetchEvents();
    }
  }, [selectedDevice, fetchSummaries, fetchEvents]);

  useEffect(() => {
    const interval = setInterval(() => {
      if (selectedDevice) {
        fetchSummaries();
        fetchEvents();
      }
    }, 30000);
    return () => clearInterval(interval);
  }, [selectedDevice, fetchSummaries, fetchEvents]);

  const sortedSummaries = [...summaries].sort((a, b) => {
    switch (summarySort) {
      case "duration": return b.total_duration_sec - a.total_duration_sec;
      case "sessions": return b.session_count - a.session_count;
      case "productive": return b.productive_duration_sec - a.productive_duration_sec;
      default: return 0;
    }
  });

  const totalDuration = summaries.reduce((sum, s) => sum + s.total_duration_sec, 0);
  const productiveDuration = summaries.reduce((sum, s) => sum + s.productive_duration_sec, 0);
  const totalSessions = summaries.reduce((sum, s) => sum + s.session_count, 0);
  const productivityScore = totalDuration > 0 ? (productiveDuration / totalDuration * 100).toFixed(0) : "0";

  const selectedDeviceInfo = devices.find((d) => d.id === selectedDevice);

  if (loading) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-semibold tracking-tight">Activity</h1>
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
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Activity</h1>
          <p className="text-muted-foreground">
            Monitor application usage and productivity across devices.
          </p>
        </div>
        <div className="flex items-center gap-3">
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
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              fetchSummaries();
              fetchEvents();
            }}
          >
            <RefreshCw className="mr-1 h-3 w-3" />
            Refresh
          </Button>
        </div>
      </div>

      {selectedDevice && (
        <>
          <div className="grid gap-4 md:grid-cols-4">
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                  Total Usage
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{formatDuration(totalDuration)}</div>
                <p className="text-xs text-muted-foreground mt-1">
                  across {summaries.length} applications
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                  Productivity Score
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{productivityScore}%</div>
                <p className="text-xs text-muted-foreground mt-1">
                  {formatDuration(productiveDuration)} productive time
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                  Sessions
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{totalSessions}</div>
                <p className="text-xs text-muted-foreground mt-1">total app sessions</p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">
                  Device Info
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-sm font-medium">
                  {selectedDeviceInfo?.hostname || "Unknown"}
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  {selectedDeviceInfo?.os_type} &middot;{" "}
                  <Badge variant={selectedDeviceInfo?.connection_status === "online" ? "default" : "secondary"} className="text-xs">
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

          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <TrendingUp className="h-5 w-5" />
                    Application Usage
                  </CardTitle>
                  <CardDescription>
                    Time spent per application with productivity classification
                  </CardDescription>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground">Sort by:</span>
                  <Button
                    variant={summarySort === "duration" ? "default" : "outline"}
                    size="sm"
                    onClick={() => setSummarySort("duration")}
                  >
                    Duration
                  </Button>
                  <Button
                    variant={summarySort === "sessions" ? "default" : "outline"}
                    size="sm"
                    onClick={() => setSummarySort("sessions")}
                  >
                    Sessions
                  </Button>
                  <Button
                    variant={summarySort === "productive" ? "default" : "outline"}
                    size="sm"
                    onClick={() => setSummarySort("productive")}
                  >
                    Productive
                  </Button>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              {eventsLoading ? (
                <div className="space-y-2">
                  {[1, 2, 3, 4, 5].map((i) => (
                    <Skeleton key={i} className="h-12" />
                  ))}
                </div>
              ) : sortedSummaries.length === 0 ? (
                <p className="text-muted-foreground text-sm py-8 text-center">
                  No usage data available for this device yet.
                </p>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Application</TableHead>
                      <TableHead className="text-right">Total Time</TableHead>
                      <TableHead className="text-right">Sessions</TableHead>
                      <TableHead className="text-right">Productive</TableHead>
                      <TableHead className="text-right">Unproductive</TableHead>
                      <TableHead className="text-right">Neutral</TableHead>
                      <TableHead>Classification</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {sortedSummaries.map((s) => {
                      const badge = getClassificationBadge(
                        s.productive_duration_sec >= s.unproductive_duration_sec && s.productive_duration_sec >= s.neutral_duration_sec
                          ? "productive"
                          : s.unproductive_duration_sec >= s.neutral_duration_sec
                          ? "unproductive"
                          : "neutral"
                      );
                      return (
                        <TableRow key={`${s.app_name}-${s.device_id}`}>
                          <TableCell className="font-medium">{s.app_name}</TableCell>
                          <TableCell className="text-right">{formatDuration(s.total_duration_sec)}</TableCell>
                          <TableCell className="text-right">{s.session_count}</TableCell>
                          <TableCell className="text-right text-emerald-600">{formatDuration(s.productive_duration_sec)}</TableCell>
                          <TableCell className="text-right text-red-600">{formatDuration(s.unproductive_duration_sec)}</TableCell>
                          <TableCell className="text-right text-blue-600">{formatDuration(s.neutral_duration_sec)}</TableCell>
                          <TableCell>
                            <Badge className={badge.className}>{badge.label}</Badge>
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Activity className="h-5 w-5" />
                Recent Activity
              </CardTitle>
              <CardDescription>
                Latest application events from this device
              </CardDescription>
            </CardHeader>
            <CardContent>
              {events.length === 0 ? (
                <p className="text-muted-foreground text-sm py-8 text-center">
                  No recent activity recorded.
                </p>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Application</TableHead>
                      <TableHead>Window Title</TableHead>
                      <TableHead className="text-right">Duration</TableHead>
                      <TableHead>Classification</TableHead>
                      <TableHead className="text-right">Confidence</TableHead>
                      <TableHead className="text-right">Time</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {events.slice(0, 50).map((e, i) => {
                      const badge = getClassificationBadge(e.classification);
                      return (
                        <TableRow key={i}>
                          <TableCell className="font-medium">{e.app_name}</TableCell>
                          <TableCell className="max-w-[200px] truncate text-muted-foreground text-sm">
                            {e.window_title || "—"}
                          </TableCell>
                          <TableCell className="text-right">
                            {e.duration_sec > 0 ? formatDuration(e.duration_sec) : "—"}
                          </TableCell>
                          <TableCell>
                            <Badge className={badge.className}>{badge.label}</Badge>
                          </TableCell>
                          <TableCell className="text-right text-muted-foreground text-sm">
                            {(e.confidence * 100).toFixed(0)}%
                          </TableCell>
                          <TableCell className="text-right text-muted-foreground text-sm">
                            {timeAgo(e.end_time)}
                          </TableCell>
                        </TableRow>
                      );
                    })}
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
            <h3 className="mt-4 text-lg font-semibold">No active devices</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              Enroll a device to start seeing activity data.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}