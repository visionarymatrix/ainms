"use client";

import { useEffect, useState, useCallback, useRef } from "react";
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
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { api } from "@/lib/api/client";
import { getUser, getToken } from "@/lib/auth/session";
import { requestScreenshot, getDeviceScreenshots, type ScreenshotRequest } from "@/lib/api/employees";
import { useSocket } from "@/lib/socket";
import {
  CheckCircle,
  XCircle,
  Copy,
  ChevronDown,
  ChevronUp,
  Cpu,
  HardDrive,
  MemoryStick,
  Network,
  Wifi,
  AlertCircle,
  Camera,
  RefreshCw,
  X,
  Eye,
} from "lucide-react";
import { toast } from "sonner";

interface Device {
  id: string;
  employee_id: string;
  hostname: string | null;
  os_type: string;
  os_version: string | null;
  agent_version: string | null;
  fingerprint: string | null;
  cpu_info: string | null;
  ram_info: string | null;
  disk_info: string | null;
  mac_addresses: string | null;
  ip_addresses: string | null;
  status: "pending" | "active" | "rejected";
  connection_status?: "online" | "idle" | "offline";
  approved_by: string | null;
  approved_at: string | null;
  last_heartbeat: string | null;
  enrolled_at: string;
}

interface Employee {
  id: string;
  first_name: string;
  last_name: string;
  employee_id: string;
}

function timeAgo(dateStr: string | null): string {
  if (!dateStr) return "Never";
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return "just now";
  if (diffMin < 60) return `${diffMin} min ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  return `${diffDay}d ago`;
}

function truncateFingerprint(fp: string | null): string {
  if (!fp) return "—";
  if (fp.length <= 20) return fp;
  return `${fp.slice(0, 12)}...${fp.slice(-8)}`;
}

function getConnectionStatus(device: Device): { label: string; variant: "default" | "outline" | "secondary" | "destructive" } {
  if (device.connection_status) {
    switch (device.connection_status) {
      case "online": return { label: "online", variant: "default" };
      case "idle": return { label: "idle", variant: "secondary" };
      case "offline": return { label: "offline", variant: "destructive" };
    }
  }
  return { label: "unknown", variant: "secondary" };
}

function getStatusBadge(status: string): { label: string; className: string } {
  switch (status) {
    case "pending":
      return { label: "Pending", className: "bg-amber-100 text-amber-800 hover:bg-amber-100 border-amber-200" };
    case "active":
      return { label: "Active", className: "bg-emerald-100 text-emerald-800 hover:bg-emerald-100 border-emerald-200" };
    case "rejected":
      return { label: "Rejected", className: "bg-red-100 text-red-800 hover:bg-red-100 border-red-200" };
    default:
      return { label: status, className: "" };
  }
}

function copyToClipboard(text: string, label: string) {
  navigator.clipboard.writeText(text);
  toast.success(`${label} copied to clipboard`);
}

export default function DevicesPage() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [pendingDevices, setPendingDevices] = useState<Device[]>([]);
  const [employees, setEmployees] = useState<Record<string, Employee>>({});
  const [loading, setLoading] = useState(true);
  const [pendingLoading, setPendingLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedRows, setExpandedRows] = useState<Set<string>>(new Set());
  const [actionLoading, setActionLoading] = useState<Record<string, boolean>>({});
  const [screenshotLoading, setScreenshotLoading] = useState<Record<string, boolean>>({});
  const [pendingScreenshots, setPendingScreenshots] = useState<Record<string, string>>({});
  const [completedScreenshots, setCompletedScreenshots] = useState<Record<string, ScreenshotRequest>>({});
  const [viewingScreenshot, setViewingScreenshot] = useState<ScreenshotRequest | null>(null);
  const [rejectDialogOpen, setRejectDialogOpen] = useState(false);
  const [selectedDeviceId, setSelectedDeviceId] = useState<string | null>(null);
  const user = getUser();

  const token = getToken();
  const { isConnected, on } = useSocket(token);

  const fetchEmployees = useCallback(async () => {
    if (!user?.company_id) return;
    try {
      const data = await api.get<Employee[]>(`/v1/companies/${user.company_id}/employees`);
      const empMap: Record<string, Employee> = {};
      data.forEach((emp) => {
        empMap[emp.id] = emp;
      });
      setEmployees(empMap);
    } catch (err) {
      console.error("Failed to fetch employees:", err);
    }
  }, [user?.company_id]);

  const fetchDevices = useCallback(async () => {
    try {
      const data = await api.get<Device[]>("/v1/devices/status");
      setDevices(data || []);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load devices");
      setDevices([]);
    } finally {
      setLoading(false);
    }
  }, []);

  const fetchPendingDevices = useCallback(async () => {
    try {
      console.log("Fetching pending devices...");
      const data = await api.get<Device[]>("/v1/devices/pending");
      console.log("Pending devices response:", data);
      setPendingDevices(data || []);
    } catch (err) {
      console.error("Failed to fetch pending devices:", err);
      setPendingDevices([]);
    } finally {
      setPendingLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchDevices();
    fetchEmployees();
    fetchPendingDevices();
  }, [fetchDevices, fetchEmployees, fetchPendingDevices]);

  useEffect(() => {
    const interval = setInterval(fetchPendingDevices, 30000);
    return () => clearInterval(interval);
  }, [fetchPendingDevices]);

  useEffect(() => {
    const interval = setInterval(fetchDevices, 30000);
    return () => clearInterval(interval);
  }, [fetchDevices]);

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
          d.id === data.device_id
            ? { ...d, connection_status: "offline" as const }
            : d
        )
      );
    });
    const offScreenshotReady = on(
      "screenshot_ready",
      (data: { request_id: string; device_id: string; status: string; image_path: string }) => {
        const completed: ScreenshotRequest = {
          id: data.request_id,
          device_id: data.device_id,
          requested_by: "",
          reason: "",
          policy: "",
          status: data.status,
          image_path: data.image_path,
          created_at: new Date().toISOString(),
          completed_at: new Date().toISOString(),
        };
        setCompletedScreenshots((prev) => ({ ...prev, [data.device_id]: completed }));
        setPendingScreenshots((prev) => {
          const next = { ...prev };
          delete next[data.device_id];
          return next;
        });
        setScreenshotLoading((prev) => ({ ...prev, [data.device_id]: false }));
        toast.success("Screenshot captured and uploaded!");
      }
    );
    return () => {
      offOnline();
      offOffline();
      offScreenshotReady();
    };
  }, [on]);

  async function handleApprove(deviceId: string) {
    setActionLoading((prev) => ({ ...prev, [deviceId]: true }));
    try {
      await api.post(`/v1/devices/${deviceId}/approve`, {});
      toast.success("Device approved successfully");
      await fetchPendingDevices();
      await fetchDevices();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to approve device");
    } finally {
      setActionLoading((prev) => ({ ...prev, [deviceId]: false }));
    }
  }

  async function handleReject(deviceId: string) {
    setActionLoading((prev) => ({ ...prev, [deviceId]: true }));
    try {
      await api.post(`/v1/devices/${deviceId}/reject`, {});
      toast.success("Device rejected");
      await fetchPendingDevices();
      await fetchDevices();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to reject device");
    } finally {
      setActionLoading((prev) => ({ ...prev, [deviceId]: false }));
      setRejectDialogOpen(false);
      setSelectedDeviceId(null);
    }
  }

  function openRejectDialog(deviceId: string) {
    setSelectedDeviceId(deviceId);
    setRejectDialogOpen(true);
  }

  const pendingRequestIds = useRef<Record<string, string>>({});

  async function handleTakeScreenshot(deviceId: string) {
    setScreenshotLoading((prev) => ({ ...prev, [deviceId]: true }));
    setPendingScreenshots((prev) => ({ ...prev, [deviceId]: "requested" }));
    try {
      const req = await requestScreenshot(deviceId);
      const requestId = req.id;
      pendingRequestIds.current[deviceId] = requestId;
      setPendingScreenshots((prev) => ({ ...prev, [deviceId]: "capturing" }));
      toast.info("Screenshot requested — waiting for agent to capture...");

      await new Promise<void>((resolve) => {
        const timeout = setTimeout(async () => {
          try {
            const screenshots = await getDeviceScreenshots(deviceId);
            const completed = screenshots.find(
              (s) => s.id === requestId && s.status === "completed" && s.image_path
            );
            if (completed) {
              setCompletedScreenshots((prev) => ({ ...prev, [deviceId]: completed }));
              setPendingScreenshots((prev) => {
                const next = { ...prev };
                delete next[deviceId];
                return next;
              });
              toast.success("Screenshot captured and uploaded!");
            } else {
              setPendingScreenshots((prev) => ({ ...prev, [deviceId]: "timeout" }));
              toast.warning("Screenshot request sent, but agent hasn't responded yet. Check back later.");
            }
          } catch {
            setPendingScreenshots((prev) => ({ ...prev, [deviceId]: "timeout" }));
            toast.warning("Screenshot request sent, but agent hasn't responded yet. Check back later.");
          }
          resolve();
        }, 60000);


        const checkSocket = setInterval(() => {
          if (!pendingRequestIds.current[deviceId]) {
            clearTimeout(timeout);
            clearInterval(checkSocket);
            resolve();
          }
        }, 500);
      });
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to request screenshot");
      setPendingScreenshots((prev) => {
        const next = { ...prev };
        delete next[deviceId];
        return next;
      });
    } finally {
      delete pendingRequestIds.current[deviceId];
      setScreenshotLoading((prev) => ({ ...prev, [deviceId]: false }));
    }
  }

  function getScreenshotImageUrl(requestId: string): string {
    const t = typeof window !== "undefined" ? localStorage.getItem("token") || "" : "";
    return `/api/screenshots/${requestId}?token=${encodeURIComponent(t)}`;
  }

  function toggleRow(deviceId: string) {
    setExpandedRows((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(deviceId)) {
        newSet.delete(deviceId);
      } else {
        newSet.add(deviceId);
      }
      return newSet;
    });
  }

  const onlineCount = devices.filter((d) => d.connection_status === "online").length;

  const activeCount = devices.filter((d) => d.status === "active").length;
  const pendingCount = devices.filter((d) => d.status === "pending").length;

  return (
    <TooltipProvider delayDuration={200}>
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">Devices</h1>
            <p className="text-muted-foreground">
              Monitor and manage enrolled devices across your organization.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <span className={`relative inline-flex h-2 w-2 rounded-full ${isConnected ? "bg-emerald-500" : "bg-red-500"}`}>
                {isConnected && (
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-500 opacity-40" />
                )}
              </span>
              {isConnected ? "Socket connected" : "Socket disconnected"}
            </div>
          </div>
        </div>

        <div className="grid gap-4 md:grid-cols-4">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                Total Devices
              </CardTitle>
            </CardHeader>
            <CardContent>
              {loading ? (
                <Skeleton className="h-9 w-16" />
              ) : (
                <div className="text-3xl font-bold">{devices.length}</div>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                Online
              </CardTitle>
            </CardHeader>
            <CardContent>
              {loading ? (
                <Skeleton className="h-9 w-16" />
              ) : (
                <div className="text-3xl font-bold text-emerald-600">{onlineCount}</div>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                Active
              </CardTitle>
            </CardHeader>
            <CardContent>
              {loading ? (
                <Skeleton className="h-9 w-16" />
              ) : (
                <div className="text-3xl font-bold text-blue-600">{activeCount}</div>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                Pending
              </CardTitle>
            </CardHeader>
            <CardContent>
              {loading ? (
                <Skeleton className="h-9 w-16" />
              ) : (
                <div className="text-3xl font-bold text-amber-600">{pendingCount}</div>
              )}
            </CardContent>
          </Card>
        </div>

        {error && (
          <Card className="border-red-200 bg-red-50">
            <CardContent className="pt-4 flex items-center gap-2">
              <AlertCircle className="h-4 w-4 text-red-800" />
              <p className="text-sm text-red-800">{error}</p>
            </CardContent>
          </Card>
        )}

        {(pendingDevices.length > 0 || pendingLoading) && (
          <Card className="border-amber-200 bg-amber-50/50">
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <AlertCircle className="h-5 w-5 text-amber-600" />
                    Pending Approvals
                    {!pendingLoading && pendingDevices.length > 0 && (
                      <Badge variant="secondary" className="bg-amber-100 text-amber-800">
                        {pendingDevices.length}
                      </Badge>
                    )}
                  </CardTitle>
                  <CardDescription>
                    Review and approve new device enrollments
                  </CardDescription>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={fetchPendingDevices}
                  disabled={pendingLoading}
                >
                  {pendingLoading ? "Refreshing..." : "Refresh"}
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              {pendingLoading ? (
                <div className="space-y-4">
                  {[1, 2].map((i) => (
                    <Skeleton key={i} className="h-32 w-full" />
                  ))}
                </div>
              ) : pendingDevices.length === 0 ? (
                <p className="text-sm text-muted-foreground text-center py-4">
                  No pending devices
                </p>
              ) : (
                <div className="space-y-4">
                  {pendingDevices.map((device) => {
                    const employee = employees[device.employee_id];
                    return (
                      <Card key={device.id} className="bg-white">
                        <CardContent className="p-4">
                          <div className="flex flex-col lg:flex-row lg:items-start lg:justify-between gap-4">
                            <div className="flex-1 space-y-3">
                              <div className="flex items-center gap-2 flex-wrap">
                                <h3 className="font-semibold text-lg">
                                  {device.hostname || "Unknown Device"}
                                </h3>
                                <Badge variant="outline" className="bg-amber-50 text-amber-700 border-amber-200">
                                  Pending
                                </Badge>
                              </div>
                              
                              <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                                <div>
                                  <span className="text-muted-foreground">Employee:</span>
                                  <p className="font-medium">
                                    {employee
                                      ? `${employee.first_name} ${employee.last_name}`
                                      : device.employee_id.slice(0, 8)}...
                                  </p>
                                </div>
                                <div>
                                  <span className="text-muted-foreground">OS:</span>
                                  <p className="font-medium">
                                    {device.os_type} {device.os_version || ""}
                                  </p>
                                </div>
                                <div>
                                  <span className="text-muted-foreground">Enrolled:</span>
                                  <p className="font-medium">{timeAgo(device.enrolled_at)}</p>
                                </div>
                                <div className="col-span-2 md:col-span-1">
                                  <span className="text-muted-foreground">Fingerprint:</span>
                                  <div className="flex items-center gap-1">
                                    <code className="text-xs bg-muted px-1.5 py-0.5 rounded">
                                      {truncateFingerprint(device.fingerprint)}
                                    </code>
                                    {device.fingerprint && (
                                      <Button
                                        variant="ghost"
                                        size="icon"
                                        className="h-5 w-5"
                                        onClick={() => copyToClipboard(device.fingerprint!, "Fingerprint")}
                                      >
                                        <Copy className="h-3 w-3" />
                                      </Button>
                                    )}
                                  </div>
                                </div>
                              </div>

                              <div className="grid grid-cols-2 md:grid-cols-3 gap-2 text-xs text-muted-foreground">
                                {device.cpu_info && (
                                  <div className="col-span-2 flex items-start gap-1">
                                    <Cpu className="h-3 w-3 mt-0.5 shrink-0" />
                                    <span className="break-words">{device.cpu_info}</span>
                                  </div>
                                )}
                                {device.ram_info && (
                                  <div className="flex items-center gap-1">
                                    <MemoryStick className="h-3 w-3 shrink-0" />
                                    <span>{device.ram_info}</span>
                                  </div>
                                )}
                                {device.disk_info && (
                                  <div className="flex items-center gap-1">
                                    <HardDrive className="h-3 w-3 shrink-0" />
                                    <span>{device.disk_info}</span>
                                  </div>
                                )}
                                {device.mac_addresses && (
                                  <div className="flex items-center gap-1">
                                    <Network className="h-3 w-3" />
                                    <span className="truncate" title={device.mac_addresses}>
                                      MAC: {device.mac_addresses.split(",")[0]}
                                    </span>
                                  </div>
                                )}
                                {device.ip_addresses && (
                                  <div className="flex items-center gap-1">
                                    <Wifi className="h-3 w-3" />
                                    <span className="truncate" title={device.ip_addresses}>
                                      IP: {device.ip_addresses.split(",")[0]}
                                    </span>
                                  </div>
                                )}
                              </div>
                            </div>

                            <div className="flex flex-col sm:flex-row gap-2 lg:min-w-[200px]">
                              <Button
                                variant="outline"
                                className="border-emerald-200 bg-emerald-50 text-emerald-700 hover:bg-emerald-100 hover:text-emerald-800"
                                onClick={() => handleApprove(device.id)}
                                disabled={actionLoading[device.id]}
                              >
                                <CheckCircle className="mr-2 h-4 w-4" />
                                {actionLoading[device.id] ? "Approving..." : "Approve"}
                              </Button>
                              <Button
                                variant="outline"
                                className="border-red-200 bg-red-50 text-red-700 hover:bg-red-100 hover:text-red-800"
                                onClick={() => openRejectDialog(device.id)}
                                disabled={actionLoading[device.id]}
                              >
                                <XCircle className="mr-2 h-4 w-4" />
                                Reject
                              </Button>
                            </div>
                          </div>
                        </CardContent>
                      </Card>
                    );
                  })}
                </div>
              )}
            </CardContent>
          </Card>
        )}

        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle>Device Fleet</CardTitle>
                <CardDescription>
                  {loading ? "Loading..." : `${devices.length} devices registered`}
                </CardDescription>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={fetchDevices}
                disabled={loading}
              >
                {loading ? "Refreshing..." : "Refresh"}
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="space-y-3">
                {[1, 2, 3].map((i) => (
                  <Skeleton key={i} className="h-12 w-full" />
                ))}
              </div>
            ) : devices.length === 0 ? (
              <p className="text-sm text-muted-foreground text-center py-8">
                No devices enrolled yet. Run the agent to enroll a device.
              </p>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-10"></TableHead>
                    <TableHead>Hostname</TableHead>
                    <TableHead>Employee</TableHead>
                    <TableHead>OS</TableHead>
                    <TableHead>Fingerprint</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Connection</TableHead>
                    <TableHead>Last Seen</TableHead>
                    <TableHead>Enrolled</TableHead>
                    <TableHead className="text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {devices.map((device) => {
                    const connectionStatus = getConnectionStatus(device);
                    const statusBadge = getStatusBadge(device.status);
                    const employee = employees[device.employee_id];
                    const isExpanded = expandedRows.has(device.id);

                    return (
                      <>
                        <TableRow
                          key={device.id}
                          className="cursor-pointer hover:bg-muted/50"
                          onClick={() => toggleRow(device.id)}
                        >
                          <TableCell>
                            <Button variant="ghost" size="icon" className="h-6 w-6">
                              {isExpanded ? (
                                <ChevronUp className="h-4 w-4" />
                              ) : (
                                <ChevronDown className="h-4 w-4" />
                              )}
                            </Button>
                          </TableCell>
                          <TableCell className="font-medium">
                            {device.hostname || "unknown"}
                          </TableCell>
                          <TableCell>
                            {employee
                              ? `${employee.first_name} ${employee.last_name}`
                              : device.employee_id.slice(0, 8)}...
                          </TableCell>
                          <TableCell>
                            <Tooltip>
                              <TooltipTrigger className="cursor-help">
                                <span className="capitalize">{device.os_type}</span>
                              </TooltipTrigger>
                              <TooltipContent>
                                <p>{device.os_version || "Unknown version"}</p>
                              </TooltipContent>
                            </Tooltip>
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center gap-1">
                              <Tooltip>
                                <TooltipTrigger className="cursor-help">
                                  <code className="text-xs bg-muted px-1.5 py-0.5 rounded">
                                    {truncateFingerprint(device.fingerprint)}
                                  </code>
                                </TooltipTrigger>
                                <TooltipContent className="max-w-md">
                                  <p className="font-mono text-xs break-all">
                                    {device.fingerprint || "No fingerprint"}
                                  </p>
                                </TooltipContent>
                              </Tooltip>
                              {device.fingerprint && (
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  className="h-5 w-5 opacity-0 group-hover:opacity-100 hover:opacity-100"
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    copyToClipboard(device.fingerprint!, "Fingerprint");
                                  }}
                                >
                                  <Copy className="h-3 w-3" />
                                </Button>
                              )}
                            </div>
                          </TableCell>
                          <TableCell>
                            <Badge variant="outline" className={statusBadge.className}>
                              {statusBadge.label}
                            </Badge>
                          </TableCell>
                          <TableCell>
                            <Badge variant={connectionStatus.variant}>
                              {connectionStatus.label}
                            </Badge>
                          </TableCell>
                          <TableCell>{timeAgo(device.last_heartbeat)}</TableCell>
                          <TableCell>{timeAgo(device.enrolled_at)}</TableCell>
                          <TableCell className="text-right">
                            {device.connection_status === "online" || device.connection_status === "idle" ? (
                              <div className="flex items-center justify-end gap-1">
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  className="h-7 text-xs gap-1"
                                  disabled={screenshotLoading[device.id]}
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    handleTakeScreenshot(device.id);
                                  }}
                                >
                                  {screenshotLoading[device.id] ? (
                                    <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                                  ) : (
                                    <Camera className="h-3.5 w-3.5" />
                                  )}
                                  {pendingScreenshots[device.id] === "capturing" ? "Capturing..." : "Screenshot"}
                                </Button>
                              </div>
                            ) : null}
                          </TableCell>
                        </TableRow>
                        {isExpanded && (
                          <TableRow className="bg-muted/30">
                            <TableCell colSpan={10} className="p-4">
                              <div className="space-y-4">
                                <h4 className="font-semibold text-sm">System Information</h4>
                                <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4 text-sm">
                                  {device.fingerprint && (
                                    <div className="col-span-2 lg:col-span-2">
                                      <span className="text-muted-foreground block mb-1">Fingerprint</span>
                                      <div className="flex items-center gap-2">
                                        <code className="bg-background px-2 py-1 rounded text-xs font-mono break-all flex-1">
                                          {device.fingerprint}
                                        </code>
                                        <Button
                                          variant="ghost"
                                          size="icon"
                                          className="h-6 w-6 shrink-0"
                                          onClick={() => copyToClipboard(device.fingerprint!, "Fingerprint")}
                                        >
                                          <Copy className="h-3 w-3" />
                                        </Button>
                                      </div>
                                    </div>
                                  )}
                                  {device.cpu_info && (
                                    <div className="col-span-2 md:col-span-1">
                                      <span className="text-muted-foreground block mb-1">CPU</span>
                                      <div className="flex items-start gap-2">
                                        <Cpu className="h-4 w-4 text-muted-foreground mt-0.5 shrink-0" />
                                        <span className="break-words text-sm">{device.cpu_info}</span>
                                      </div>
                                    </div>
                                  )}
                                  {device.ram_info && (
                                    <div>
                                      <span className="text-muted-foreground block mb-1">RAM</span>
                                      <div className="flex items-center gap-2">
                                        <MemoryStick className="h-4 w-4 text-muted-foreground shrink-0" />
                                        <span className="text-sm">{device.ram_info}</span>
                                      </div>
                                    </div>
                                  )}
                                  {device.disk_info && (
                                    <div>
                                      <span className="text-muted-foreground block mb-1">Disk</span>
                                      <div className="flex items-center gap-2">
                                        <HardDrive className="h-4 w-4 text-muted-foreground shrink-0" />
                                        <span className="text-sm">{device.disk_info}</span>
                                      </div>
                                    </div>
                                  )}
                                  {device.mac_addresses && (
                                    <div className="col-span-2 md:col-span-3 lg:col-span-4">
                                      <span className="text-muted-foreground block mb-1">MAC Addresses</span>
                                      <div className="flex items-start gap-2">
                                        <Network className="h-4 w-4 text-muted-foreground mt-0.5 shrink-0" />
                                        <div className="flex flex-wrap gap-1">
                                          {device.mac_addresses.split(",").map((mac) => {
                                            const [iface, addr] = mac.trim().split("=");
                                            return (
                                              <code key={iface} className="text-xs bg-muted px-1.5 py-0.5 rounded whitespace-nowrap">
                                                {iface}: {addr}
                                              </code>
                                            );
                                          })}
                                        </div>
                                      </div>
                                    </div>
                                  )}
                                  {device.ip_addresses && (
                                    <div className="col-span-2 md:col-span-3 lg:col-span-4">
                                      <span className="text-muted-foreground block mb-1">IP Addresses</span>
                                      <div className="flex items-start gap-2">
                                        <Wifi className="h-4 w-4 text-muted-foreground mt-0.5 shrink-0" />
                                        <div className="flex flex-wrap gap-1">
                                          {device.ip_addresses.split(",").map((ip, i) => (
                                            <code key={i} className="text-xs bg-muted px-1.5 py-0.5 rounded whitespace-nowrap">
                                              {ip.trim()}
                                            </code>
                                          ))}
                                        </div>
                                      </div>
                                    </div>
                                  )}
                                  {device.approved_by && (
                                    <div>
                                      <span className="text-muted-foreground block mb-1">Approved By</span>
                                      <span className="font-mono text-xs">{device.approved_by}</span>
                                    </div>
                                  )}
                                  {device.approved_at && (
                                    <div>
                                      <span className="text-muted-foreground block mb-1">Approved At</span>
                                      <span>{new Date(device.approved_at).toLocaleString()}</span>
                                    </div>
                                  )}
                                </div>


                                {(completedScreenshots[device.id] || pendingScreenshots[device.id]) && (
                                  <div className="mt-4 pt-4 border-t">
                                    <h4 className="font-semibold text-sm flex items-center gap-2 mb-2">
                                      <Camera className="h-4 w-4 text-muted-foreground" />
                                      Screenshot
                                    </h4>
                                    {completedScreenshots[device.id] ? (
                                      <div className="relative group rounded-lg overflow-hidden border bg-card w-fit">
                                        <img
                                          src={getScreenshotImageUrl(completedScreenshots[device.id].id)}
                                          alt="Device screenshot"
                                          className="max-h-48 object-cover cursor-pointer hover:opacity-90 transition-opacity"
                                          onClick={() => setViewingScreenshot(completedScreenshots[device.id])}
                                        />
                                        <div className="absolute top-1 right-1 opacity-0 group-hover:opacity-100 transition-opacity">
                                          <Button
                                            variant="secondary"
                                            size="icon"
                                            className="h-7 w-7"
                                            onClick={() => setViewingScreenshot(completedScreenshots[device.id])}
                                          >
                                            <Eye className="h-3.5 w-3.5" />
                                          </Button>
                                        </div>
                                        <div className="absolute bottom-0 inset-x-0 bg-black/60 px-2 py-1">
                                          <p className="text-white text-[10px]">
                                            {new Date(completedScreenshots[device.id].created_at).toLocaleString()}
                                          </p>
                                        </div>
                                      </div>
                                    ) : pendingScreenshots[device.id] === "timeout" ? (
                                      <div className="flex items-center gap-2 text-muted-foreground text-sm py-2">
                                        <AlertCircle className="h-4 w-4" />
                                        <span>Agent did not respond. Try again later.</span>
                                      </div>
                                    ) : (
                                      <div className="flex items-center gap-2 text-muted-foreground text-sm py-2">
                                        <RefreshCw className="h-4 w-4 animate-spin" />
                                        <span>
                                          {pendingScreenshots[device.id] === "capturing"
                                            ? "Agent is capturing screenshot..."
                                            : "Requesting screenshot..."}
                                        </span>
                                      </div>
                                    )}
                                  </div>
                                )}
                              </div>
                            </TableCell>
                          </TableRow>
                        )}
                      </>
                    );
                  })}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>

        <Dialog open={rejectDialogOpen} onOpenChange={setRejectDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Reject Device</DialogTitle>
              <DialogDescription>
                Are you sure you want to reject this device enrollment? This action cannot be undone.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setRejectDialogOpen(false)}>
                Cancel
              </Button>
              <Button
                variant="destructive"
                onClick={() => selectedDeviceId && handleReject(selectedDeviceId)}
                disabled={selectedDeviceId ? actionLoading[selectedDeviceId] : false}
              >
                {selectedDeviceId && actionLoading[selectedDeviceId] ? "Rejecting..." : "Reject"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <Dialog open={!!viewingScreenshot} onOpenChange={(open) => { if (!open) setViewingScreenshot(null); }}>
          <DialogContent className="max-w-4xl p-0 gap-0 overflow-hidden">
            <DialogHeader className="px-6 pt-6 pb-2">
              <DialogTitle>Screenshot</DialogTitle>
              <DialogDescription>
                {viewingScreenshot ? new Date(viewingScreenshot.created_at).toLocaleString() : ""}
              </DialogDescription>
            </DialogHeader>
            <div className="px-6 pb-6">
              {viewingScreenshot && (
                <img
                  src={getScreenshotImageUrl(viewingScreenshot.id)}
                  alt="Device screenshot"
                  className="w-full rounded-lg"
                />
              )}
            </div>
          </DialogContent>
        </Dialog>
      </div>
    </TooltipProvider>
  );
}
