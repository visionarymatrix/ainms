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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Separator } from "@/components/ui/separator";
import { Plus, Trash2, Copy, Eye, EyeOff, RefreshCw, Monitor, Terminal, Laptop, CloudOff, Camera, Send, MessageSquare } from "lucide-react";
import Link from "next/link";
import { toast } from "sonner";
import { getUser, getCompanyId, getToken } from "@/lib/auth/session";
import { listEmployees, registerEmployee, deactivateEmployee, getEmployeeDevices, requestScreenshot, getDeviceScreenshots, sendNLQuery, type Employee, type Device, type ScreenshotRequest, type NLQueryResponse, type AgentReport } from "@/lib/api/employees";
import { getEmployeeInstallToken, type EmployeeInstallToken } from "@/lib/api/install-tokens";
import { listRoles, getRole, type Role } from "@/lib/api/roles";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSocket } from "@/lib/socket";

const statusBadge: Record<string, "default" | "destructive" | "outline"> = {
  active: "default",
  inactive: "outline",
  suspended: "destructive",
};

function timeAgo(dateStr: string | null): string {
  if (!dateStr) return "Never";
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  return `${diffDay}d ago`;
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function maskToken(token: string): string {
  if (token.length <= 8) return token;
  return `${token.slice(0, 4)}...${token.slice(-4)}`;
}

function copyToClipboard(text: string, label: string) {
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(text)
      .then(() => toast.success(`${label} copied to clipboard`))
      .catch(() => fallbackCopy(text, label));
  } else {
    fallbackCopy(text, label);
  }
}

function fallbackCopy(text: string, label: string) {
  try {
    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.style.position = "fixed";
    textarea.style.left = "-9999px";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    document.execCommand("copy");
    document.body.removeChild(textarea);
    toast.success(`${label} copied to clipboard`);
  } catch {
    toast.error("Copy failed. Please select and copy manually.");
  }
}

interface EmployeeDetailDialogProps {
  employee: Employee | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDeactivate?: () => void;
}

function EmployeeDetailDialog({ employee, open, onOpenChange, onDeactivate }: EmployeeDetailDialogProps) {
  const [devices, setDevices] = useState<Device[]>([]);
  const [devicesLoading, setDevicesLoading] = useState(false);
  const [installToken, setInstallToken] = useState<EmployeeInstallToken | null>(null);
  const [tokenLoading, setTokenLoading] = useState(false);
  const [showFullToken, setShowFullToken] = useState(false);
  const [activeTab, setActiveTab] = useState("linux");
  const [screenshots, setScreenshots] = useState<ScreenshotRequest[]>([]);
  const [screenshotsLoading, setScreenshotsLoading] = useState(false);
  const [screenshotRequesting, setScreenshotRequesting] = useState<string | null>(null);
  const [viewingScreenshot, setViewingScreenshot] = useState<ScreenshotRequest | null>(null);
  const [role, setRole] = useState<Role | null>(null);
  const [roleLoading, setRoleLoading] = useState(false);
  const [nlQuery, setNlQuery] = useState("");
  const [nlQueryLoading, setNlQueryLoading] = useState(false);
  const [nlReports, setNlReports] = useState<Array<{ query: string; query_id: string; report: AgentReport | null; timestamp: string }>>([]);

  const { on } = useSocket(getToken());

  const fetchDevices = useCallback(async () => {
    if (!employee) return;
    setDevicesLoading(true);
    try {
      const data = await getEmployeeDevices(employee.id);
      setDevices(data || []);
    } catch {
      setDevices([]);
    } finally {
      setDevicesLoading(false);
    }
  }, [employee]);

  const fetchInstallToken = useCallback(async () => {
    if (!employee) return;
    setTokenLoading(true);
    try {
      const data = await getEmployeeInstallToken(employee.id);
      setInstallToken(data);
    } catch {
      setInstallToken(null);
    } finally {
      setTokenLoading(false);
    }
  }, [employee]);

  const fetchScreenshots = useCallback(async () => {
    if (!employee) return;
    const deviceIds = devices.map(d => d.id);
    if (deviceIds.length === 0) {
      setScreenshots([]);
      return;
    }
    setScreenshotsLoading(true);
    try {
      const allScreenshots: ScreenshotRequest[] = [];
      await Promise.all(deviceIds.map(async (deviceId) => {
        const data = await getDeviceScreenshots(deviceId);
        if (data && data.length > 0) {
          allScreenshots.push(...data);
        }
      }));
      allScreenshots.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
      setScreenshots(allScreenshots.slice(0, 6));
    } catch {
      setScreenshots([]);
    } finally {
      setScreenshotsLoading(false);
    }
  }, [employee, devices]);

  const pendingRequestIds = useRef<Record<string, string>>({});

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
                  return updated.slice(0, 6);
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

        const checkSocket = setInterval(() => {
          if (!pendingRequestIds.current[deviceId]) {
            clearTimeout(timeout);
            clearInterval(checkSocket);
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

  const handleSendNLQuery = useCallback(async () => {
    if (!employee || !nlQuery.trim()) return;
    setNlQueryLoading(true);
    const queryText = nlQuery.trim();
    setNlQuery("");
    try {
      const resp = await sendNLQuery(employee.id, queryText);
      setNlReports((prev) => [...prev, { query: queryText, query_id: resp.query_id, report: null, timestamp: new Date().toISOString() }]);
    } catch {
      toast.error("Failed to send query to agent");
    } finally {
      setNlQueryLoading(false);
    }
  }, [employee, nlQuery]);

  useEffect(() => {
    if (open && employee) {
      fetchDevices();
      fetchInstallToken();
    }
  }, [open, employee, fetchDevices, fetchInstallToken]);

  useEffect(() => {
    if (open && employee && devices.length > 0) {
      fetchScreenshots();
    }
  }, [open, employee, devices.length, fetchScreenshots]);

  useEffect(() => {
    if (!open) {
      setShowFullToken(false);
      setActiveTab("linux");
      setScreenshots([]);
      setViewingScreenshot(null);
      setRole(null);
      setNlReports([]);
      setNlQuery("");
    }
  }, [open]);

  useEffect(() => {
    if (open && employee?.role_id) {
      setRoleLoading(true);
      getRole(employee.role_id)
        .then(setRole)
        .catch(() => setRole(null))
        .finally(() => setRoleLoading(false));
    } else {
      setRole(null);
      setRoleLoading(false);
    }
  }, [open, employee?.role_id]);

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
    const offScreenshotReady = on("screenshot_ready", (data: { request_id: string; device_id: string; status: string; image_path: string }) => {
      delete pendingRequestIds.current[data.device_id];
      fetchScreenshots();
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
  }, [on, fetchScreenshots]);

  if (!employee) return null;

  const linuxCommand = installToken?.install_cmd || "";
  const windowsCommand = installToken?.windows_cmd || "";

  const token = getToken();

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="sm:max-w-[720px] p-0 gap-0 overflow-hidden flex flex-col max-h-[85vh]">
          <DialogHeader className="px-6 pt-6 pb-4">
            <div className="flex items-center gap-4">
              <div className="h-12 w-12 rounded-full bg-primary/10 flex items-center justify-center text-primary font-bold text-lg shrink-0">
                {employee.first_name[0]}{employee.last_name[0]}
              </div>
              <div className="min-w-0">
                <DialogTitle className="text-xl">{employee.first_name} {employee.last_name}</DialogTitle>
                <DialogDescription className="flex items-center gap-2 mt-1">
                  <code className="text-xs bg-muted px-1.5 py-0.5 rounded font-mono">{employee.employee_id}</code>
                  <Badge variant={statusBadge[employee.status] || "outline"} className="text-xs">{employee.status}</Badge>
                  {employee.email && <span className="text-xs">{employee.email}</span>}
                </DialogDescription>
              </div>
            </div>
          </DialogHeader>

          <Separator />

          <div className="flex-1 overflow-y-auto">
            <div className="px-6 py-4 space-y-6">

              {employee.role_id && (
                <div>
                  <h3 className="text-sm font-semibold flex items-center gap-2 mb-3">
                    Role
                  </h3>
                  {roleLoading ? (
                    <Skeleton className="h-12 w-full rounded-lg" />
                  ) : role ? (
                    <div className="rounded-lg border bg-card px-3 py-2.5 space-y-1">
                      <div className="flex items-center gap-2">
                        <Badge variant="secondary" className="text-xs">{role.name}</Badge>
                        <span className="text-sm font-medium">{role.name}</span>
                      </div>
                      {role.work_description && (
                        <p className="text-xs text-muted-foreground">{role.work_description}</p>
                      )}
                    </div>
                  ) : (
                    <p className="text-sm text-muted-foreground">Role information unavailable</p>
                  )}
                  <Separator className="mt-6" />
                </div>
              )}

              <div>
                <div className="flex items-center justify-between mb-3">
                  <h3 className="text-sm font-semibold flex items-center gap-2">
                    <Laptop className="h-4 w-4 text-muted-foreground shrink-0" />
                    Devices
                    {devices.length > 0 && (
                      <Badge variant="secondary" className="text-xs">{devices.length}</Badge>
                    )}
                  </h3>
                  <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={fetchDevices} disabled={devicesLoading}>
                    <RefreshCw className={`h-3 w-3 mr-1 ${devicesLoading ? "animate-spin" : ""}`} />
                    Refresh
                  </Button>
                </div>

                {devicesLoading ? (
                  <div className="space-y-2">
                    {[1, 2].map((i) => (
                      <Skeleton key={i} className="h-12 w-full rounded-lg" />
                    ))}
                  </div>
                ) : devices.length === 0 ? (
                  <div className="flex items-center gap-3 py-4 text-muted-foreground">
                    <CloudOff className="h-5 w-5 shrink-0" />
                    <div>
                      <p className="text-sm">No devices connected</p>
                      <p className="text-xs">Share the install command below to enroll a device</p>
                    </div>
                  </div>
                ) : (
                  <div className="grid gap-2">
                    {devices.map((device) => {
                      const isOnline = device.connection_status === "online";
                      const isIdle = device.connection_status === "idle";
                      return (
                        <div key={device.id} className="flex items-center gap-3 rounded-lg border bg-card px-3 py-2.5">
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <div className="relative flex h-2.5 w-2.5 shrink-0">
                                {(isOnline || isIdle) && (
                                  <span className={`absolute inline-flex h-full w-full animate-ping rounded-full opacity-40 ${isOnline ? "bg-emerald-500" : "bg-amber-500"}`} />
                                )}
                                <span className={`relative inline-flex h-2.5 w-2.5 rounded-full ${isOnline ? "bg-emerald-500" : isIdle ? "bg-amber-500" : "bg-gray-400"}`} />
                              </div>
                            </TooltipTrigger>
                            <TooltipContent>{isOnline ? "Online" : isIdle ? "Idle" : "Offline"}</TooltipContent>
                          </Tooltip>
                          <Monitor className="h-4 w-4 text-muted-foreground shrink-0" />
                          <div className="min-w-0 flex-1">
                            <p className="text-sm font-medium truncate">{device.hostname || "Unnamed Device"}</p>
                            <p className="text-xs text-muted-foreground truncate">
                              {device.os_type === "linux" ? "Linux" : device.os_type === "macos" ? "macOS" : device.os_type === "windows" ? "Windows" : device.os_type}
                              {device.os_version ? ` ${device.os_version}` : ""}
                            </p>
                          </div>
                          <div className="flex items-center gap-2">
                            <div className="text-xs text-muted-foreground shrink-0">
                              {timeAgo(device.last_heartbeat)}
                            </div>
                            {(isOnline || isIdle) && (
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button
                                    variant="ghost"
                                    size="icon"
                                    className="h-7 w-7"
                                    disabled={screenshotRequesting === device.id}
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      handleTakeScreenshot(device.id);
                                    }}
                                  >
                                    {screenshotRequesting === device.id ? (
                                      <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                                    ) : (
                                      <Camera className="h-3.5 w-3.5" />
                                    )}
                                  </Button>
                                </TooltipTrigger>
                                <TooltipContent>Take Screenshot</TooltipContent>
                              </Tooltip>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>

              <Separator />

              <div>
                <div className="flex items-center justify-between mb-3">
                  <h3 className="text-sm font-semibold flex items-center gap-2">
                    <Camera className="h-4 w-4 text-muted-foreground shrink-0" />
                    Screenshots
                    {screenshots.length > 0 && (
                      <Badge variant="secondary" className="text-xs">{screenshots.length}</Badge>
                    )}
                  </h3>
                  <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={fetchScreenshots} disabled={screenshotsLoading}>
                    <RefreshCw className={`h-3 w-3 mr-1 ${screenshotsLoading ? "animate-spin" : ""}`} />
                    Refresh
                  </Button>
                </div>

                {screenshotsLoading && screenshots.length === 0 ? (
                  <div className="space-y-2">
                    <Skeleton className="h-20 w-full" />
                    <Skeleton className="h-20 w-full" />
                  </div>
                ) : screenshots.length === 0 ? (
                  <div className="flex items-center gap-3 py-4 text-muted-foreground">
                    <Camera className="h-5 w-5 shrink-0" />
                    <p className="text-sm">No screenshots yet. Take one from a device above.</p>
                  </div>
                ) : (
                  <div className="grid grid-cols-2 gap-2">
                    {screenshots.map((ss) => (
                      <div key={ss.id} className="relative group rounded-lg border overflow-hidden bg-muted/30">
                        {ss.status === 'pending' ? (
                          <div className="flex items-center justify-center h-28">
                            <div className="flex flex-col items-center gap-1">
                              <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
                              <span className="text-xs text-muted-foreground">Capturing...</span>
                            </div>
                          </div>
                        ) : ss.status === 'completed' && ss.image_path ? (
                          <img
                            src={`/api/screenshots/${ss.id}?token=${token}`}
                            alt={`Screenshot ${ss.id}`}
                            className="w-full h-28 object-cover cursor-pointer hover:opacity-90 transition-opacity"
                            onClick={() => setViewingScreenshot(ss)}
                          />
                        ) : (
                          <div className="flex items-center justify-center h-28 text-muted-foreground text-xs">
                            {ss.status}
                          </div>
                        )}
                        <div className="absolute bottom-0 inset-x-0 bg-black/60 px-2 py-1 flex items-center justify-between">
                          <p className="text-white text-[10px] truncate">
                            {new Date(ss.created_at).toLocaleString()}
                          </p>
                          {ss.status === 'completed' && (
                            <span className="text-emerald-400 text-[10px]">&#x2713;</span>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              <Separator />

              <div>
                <h3 className="text-sm font-semibold flex items-center gap-2 mb-3">
                  <MessageSquare className="h-4 w-4 text-muted-foreground shrink-0" />
                  Query Agent
                </h3>

                <div className="space-y-3">
                  {nlReports.length > 0 && (
                    <div className="space-y-3 max-h-64 overflow-y-auto">
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
                                      <p>Top apps: {entry.report.summary.top_apps.slice(0, 3).map(a => `${a.app_name} (${Math.round(a.duration_sec / 60)}min)`).join(', ')}</p>
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
                </div>
              </div>

              <Separator />

              <div>
                <h3 className="text-sm font-semibold flex items-center gap-2 mb-3">
                  <Terminal className="h-4 w-4 text-muted-foreground shrink-0" />
                  Install Command
                </h3>

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

                    <Tabs value={activeTab} onValueChange={setActiveTab}>
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
              </div>
            </div>
          </div>

          <Separator />

          <DialogFooter className="px-6 py-3 shrink-0">
            {employee.status === "active" && onDeactivate && (
              <Button variant="destructive" size="sm" onClick={onDeactivate} className="mr-auto">
                <Trash2 className="mr-1.5 h-3.5 w-3.5" />
                Deactivate
              </Button>
            )}
            <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!viewingScreenshot} onOpenChange={(open) => !open && setViewingScreenshot(null)}>
        <DialogContent className="max-w-4xl p-0 gap-0 overflow-hidden">
          <DialogHeader className="px-6 pt-6 pb-2">
            <DialogTitle className="flex items-center gap-2">
              Screenshot
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8"
                onClick={() => viewingScreenshot && copyToClipboard(`${window.location.origin}/api/screenshots/${viewingScreenshot.id}?token=${token}`, "Image URL")}
              >
                <Copy className="h-4 w-4" />
              </Button>
            </DialogTitle>
            <DialogDescription>
              {viewingScreenshot && `Taken ${new Date(viewingScreenshot.created_at).toLocaleString()}`}
            </DialogDescription>
          </DialogHeader>
          <div className="px-6 pb-6">
            {viewingScreenshot && (
              <img
                src={`/api/screenshots/${viewingScreenshot.id}?token=${token}`}
                alt="Full screenshot"
                className="w-full rounded-lg border"
              />
            )}
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default function EmployeesPage() {
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detailEmployee, setDetailEmployee] = useState<Employee | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [email, setEmail] = useState("");
  const [selectedRoleId, setSelectedRoleId] = useState<string>("");
  const [roles, setRoles] = useState<Role[]>([]);
  const [creating, setCreating] = useState(false);
  const user = getUser();
  const companyId = getCompanyId();

  const token = getToken();
  const { isConnected } = useSocket(token);

  useEffect(() => {
    if (companyId) {
      fetchEmployees();
    } else {
      setLoading(false);
    }
  }, [companyId]);

  useEffect(() => {
    if (companyId) {
      listRoles(companyId).then(setRoles).catch(() => setRoles([]));
    }
  }, [companyId]);

  async function fetchEmployees() {
    if (!companyId) return;
    try {
      const data = await listEmployees(companyId);
      setEmployees(data);
    } catch {
      setEmployees([]);
    } finally {
      setLoading(false);
    }
  }

  async function handleAddEmployee(e: React.FormEvent) {
    e.preventDefault();
    if (!companyId) return;
    setCreating(true);
    try {
              await registerEmployee(companyId, {
                first_name: firstName,
                last_name: lastName,
                email: email || undefined,
                role_id: selectedRoleId || undefined,
              });
              setDialogOpen(false);
              setFirstName("");
              setLastName("");
              setEmail("");
              setSelectedRoleId("");
      await fetchEmployees();
    } catch {
      setDialogOpen(false);
    } finally {
      setCreating(false);
    }
  }

  async function handleDeactivate() {
    if (!selectedId) return;
    try {
      await deactivateEmployee(selectedId);
      await fetchEmployees();
      setConfirmOpen(false);
      setDetailOpen(false);
    } catch {
      setConfirmOpen(false);
      setSelectedId(null);
    }
  }

  function openDetailDialog(employee: Employee) {
    setDetailEmployee(employee);
    setDetailOpen(true);
  }

  function openDeactivateDialog(employeeId: string) {
    setSelectedId(employeeId);
    setConfirmOpen(true);
  }

  if (!companyId) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Employees</h1>
          <p className="text-muted-foreground">Manage employees across companies.</p>
        </div>
        <Card>
          <CardContent className="py-12 text-center">
            <p className="text-muted-foreground">No company assigned. Contact a super admin.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <TooltipProvider delayDuration={200}>
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">Employees</h1>
            <p className="text-muted-foreground">
              Manage employees in your company.
            </p>
          </div>
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <span className={`relative inline-flex h-2 w-2 rounded-full ${isConnected ? "bg-emerald-500" : "bg-red-500"}`}>
                {isConnected && (
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-500 opacity-40" />
                )}
              </span>
              {isConnected ? "Socket connected" : "Socket disconnected"}
            </div>
            <div className="flex gap-2">
              <Button asChild variant="outline">
                <Link href="/employees/new">
                  <Plus className="mr-2 h-4 w-4" />
                  New Employee
                </Link>
              </Button>
              <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
                <DialogTrigger asChild>
                  <Button>
                    <Plus className="mr-2 h-4 w-4" />
                    Quick Add
                  </Button>
                </DialogTrigger>
                <DialogContent>
                  <DialogHeader>
                    <DialogTitle>Add Employee</DialogTitle>
                    <DialogDescription>
                      Register a new employee in your company.
                    </DialogDescription>
                  </DialogHeader>
                  <form onSubmit={handleAddEmployee} className="flex flex-col gap-4">
                    <div className="flex flex-col gap-2">
                      <Label htmlFor="first-name">First Name</Label>
                      <Input
                        id="first-name"
                        value={firstName}
                        onChange={(e) => setFirstName(e.target.value)}
                        placeholder="Jane"
                        required
                      />
                    </div>
                    <div className="flex flex-col gap-2">
                      <Label htmlFor="last-name">Last Name</Label>
                      <Input
                        id="last-name"
                        value={lastName}
                        onChange={(e) => setLastName(e.target.value)}
                        placeholder="Doe"
                        required
                      />
                    </div>
                    <div className="flex flex-col gap-2">
                      <Label htmlFor="emp-email">Email (optional)</Label>
                      <Input
                        id="emp-email"
                        type="email"
                        value={email}
                        onChange={(e) => setEmail(e.target.value)}
                        placeholder="jane@acme.com"
                      />
                    </div>
                    <div className="flex flex-col gap-2">
                      <Label htmlFor="emp-role">Role</Label>
                      <Select value={selectedRoleId} onValueChange={setSelectedRoleId}>
                        <SelectTrigger id="emp-role">
                          <SelectValue placeholder="No role assigned" />
                        </SelectTrigger>
                        <SelectContent>
                          {roles.map((role) => (
                            <SelectItem key={role.id} value={role.id}>
                              {role.name}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <Button type="submit" disabled={creating}>
                      {creating ? "Adding..." : "Add Employee"}
                    </Button>
                  </form>
                </DialogContent>
              </Dialog>
            </div>
          </div>
        </div>
        <Card>
          <CardHeader>
            <CardTitle>All Employees</CardTitle>
            <CardDescription>
              {loading ? "Loading..." : `${employees.length} employees registered`}
            </CardDescription>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="space-y-3">
                {[1, 2, 3].map((i) => (
                  <Skeleton key={i} className="h-10 w-full" />
                ))}
              </div>
            ) : employees.length === 0 ? (
              <p className="text-muted-foreground text-sm py-8 text-center">
                No employees yet. Add your first employee to get started.
              </p>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Employee ID</TableHead>
                    <TableHead>Name</TableHead>
                    <TableHead>Email</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Created</TableHead>
                    <TableHead className="text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {employees.map((emp) => (
                    <TableRow
                      key={emp.id}
                      className="cursor-pointer hover:bg-muted/50"
                      onClick={() => openDetailDialog(emp)}
                    >
                      <TableCell className="font-mono text-sm">
                        {emp.employee_id}
                      </TableCell>
                      <TableCell className="font-medium">
                        {emp.first_name} {emp.last_name}
                      </TableCell>
                      <TableCell>{emp.email || "—"}</TableCell>
                      <TableCell>
                        <Badge variant={statusBadge[emp.status] || "outline"}>
                          {emp.status}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        {new Date(emp.created_at).toLocaleDateString()}
                      </TableCell>
                      <TableCell className="text-right">
                        {emp.status === "active" && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation();
                              setSelectedId(emp.id);
                              setConfirmOpen(true);
                            }}
                          >
                            <Trash2 className="h-4 w-4 text-destructive" />
                          </Button>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>

        <EmployeeDetailDialog
          employee={detailEmployee}
          open={detailOpen}
          onOpenChange={setDetailOpen}
          onDeactivate={detailEmployee ? () => openDeactivateDialog(detailEmployee.id) : undefined}
        />

        <Dialog open={confirmOpen} onOpenChange={setConfirmOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Deactivate Employee</DialogTitle>
              <DialogDescription>
                Are you sure you want to deactivate this employee? This action cannot be undone.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setConfirmOpen(false)}>
                Cancel
              </Button>
              <Button variant="destructive" onClick={handleDeactivate}>
                Deactivate
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </TooltipProvider>
  );
}
