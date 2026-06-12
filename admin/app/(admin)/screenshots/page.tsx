"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import Link from "next/link";
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
  Camera,
  RefreshCw,
  Monitor,
  ImageOff,
  Copy,
  X,
} from "lucide-react";
import { toast } from "sonner";
import { getCompanyId, getToken } from "@/lib/auth/session";
import {
  listEmployees,
  getEmployeeDevices,
  getDeviceScreenshots,
  requestScreenshot,
  type Employee,
  type Device,
  type ScreenshotRequest,
} from "@/lib/api/employees";
import { useSocket } from "@/lib/socket";
import { timeAgo } from "@/lib/utils/format";
import { getScreenshotStatusBadgeVariant } from "@/lib/utils/badges";

interface EnrichedScreenshot extends ScreenshotRequest {
  device: Device;
  employee: Employee;
}



function getStatusLabel(status: string): string {
  switch (status) {
    case "completed":
      return "Completed";
    case "pending":
      return "Pending";
    case "failed":
      return "Failed";
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

export default function ScreenshotsPage() {
  const companyId = getCompanyId();
  const token = getToken();
  const { isConnected, on } = useSocket(token);

  const [employees, setEmployees] = useState<Employee[]>([]);
  const [allDevices, setAllDevices] = useState<Device[]>([]);
  const [screenshots, setScreenshots] = useState<EnrichedScreenshot[]>([]);
  const [loading, setLoading] = useState(true);
  const [deviceFilter, setDeviceFilter] = useState<string>("all");
  const [viewingScreenshot, setViewingScreenshot] = useState<EnrichedScreenshot | null>(null);
  const [requestDialogOpen, setRequestDialogOpen] = useState(false);
  const [requestDeviceId, setRequestDeviceId] = useState<string>("");
  const [requesting, setRequesting] = useState(false);

  const pendingRequestIds = useRef<Record<string, string>>({});

  const fetchData = useCallback(async () => {
    if (!companyId) {
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const empList = await listEmployees(companyId);
      setEmployees(empList);

      const devicesPerEmployee = await Promise.all(
        empList.map(async (emp) => {
          try {
            const devs = await getEmployeeDevices(emp.id);
            return devs.map((d) => ({ device: d, employee: emp }));
          } catch {
            return [];
          }
        })
      );

      const flatDevices = devicesPerEmployee.flat();
      const devices = flatDevices.map((d) => d.device);
      setAllDevices(devices);

      const screenshotResults = await Promise.all(
        devices.map(async (device) => {
          try {
            const ssList = await getDeviceScreenshots(device.id);
            const emp = flatDevices.find((d) => d.device.id === device.id)?.employee;
            if (!emp) return [];
            return ssList.map((ss) => ({ ...ss, device, employee: emp }));
          } catch {
            return [];
          }
        })
      );

      const allScreenshots = screenshotResults.flat();
      allScreenshots.sort(
        (a, b) =>
          new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      );
      setScreenshots(allScreenshots);
    } catch {
      setScreenshots([]);
      setAllDevices([]);
      setEmployees([]);
    } finally {
      setLoading(false);
    }
  }, [companyId]);

  useEffect(() => {
    if (companyId) {
      fetchData();
    } else {
      setLoading(false);
    }
  }, [companyId, fetchData]);

  useEffect(() => {
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  }, [fetchData]);

  useEffect(() => {
    const offScreenshotReady = on(
      "screenshot_ready",
      (data: {
        request_id: string;
        device_id: string;
        status: string;
        image_path: string;
      }) => {
        delete pendingRequestIds.current[data.device_id];
        fetchData();
      }
    );
    return () => {
      offScreenshotReady();
    };
  }, [on, fetchData]);

  const filteredScreenshots =
    deviceFilter === "all"
      ? screenshots
      : screenshots.filter((ss) => ss.device.id === deviceFilter);

  async function handleRequestScreenshot() {
    if (!requestDeviceId) {
      toast.error("Please select a device");
      return;
    }
    setRequesting(true);
    try {
      const req = await requestScreenshot(requestDeviceId);
      pendingRequestIds.current[requestDeviceId] = req.id;
      toast.success("Screenshot requested");
      setRequestDialogOpen(false);
      setRequestDeviceId("");
      // Optimistically add a pending entry
      const device = allDevices.find((d) => d.id === requestDeviceId);
      const employee = employees.find((e) => e.id === device?.employee_id);
      if (device && employee) {
        setScreenshots((prev) => [
          {
            ...req,
            device,
            employee,
          },
          ...prev,
        ]);
      }
    } catch {
      toast.error("Failed to request screenshot");
    } finally {
      setRequesting(false);
    }
  }

  function copyImageUrl(ss: EnrichedScreenshot) {
    const url = `${window.location.origin}${getScreenshotImageUrl(ss.id)}`;
    navigator.clipboard
      .writeText(url)
      .then(() => toast.success("Image URL copied to clipboard"))
      .catch(() => toast.error("Failed to copy URL"));
  }

  if (!companyId) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Screenshots</h1>
          <p className="text-muted-foreground">
            View and manage device screenshots.
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
              Screenshots
            </h1>
            <p className="text-muted-foreground">
              Browse screenshots captured from monitored devices.
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
              onClick={fetchData}
              disabled={loading}
            >
              <RefreshCw
                className={`mr-2 h-4 w-4 ${loading ? "animate-spin" : ""}`}
              />
              {loading ? "Refreshing..." : "Refresh"}
            </Button>
            <Button size="sm" onClick={() => setRequestDialogOpen(true)}>
              <Camera className="mr-2 h-4 w-4" />
              Request Screenshot
            </Button>
          </div>
        </div>

        <Card>
          <CardHeader>
            <div className="flex items-center justify-between flex-wrap gap-4">
              <div>
                <CardTitle>Captured Screenshots</CardTitle>
                <CardDescription>
                  {loading
                    ? "Loading..."
                    : `${filteredScreenshots.length} screenshot${filteredScreenshots.length === 1 ? "" : "s"} found`}
                </CardDescription>
              </div>
              <div className="w-full sm:w-64">
                <Select value={deviceFilter} onValueChange={setDeviceFilter}>
                  <SelectTrigger>
                    <SelectValue placeholder="Filter by device" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Devices</SelectItem>
                    {allDevices.map((device) => (
                      <SelectItem key={device.id} value={device.id}>
                        {device.hostname || "Unnamed Device"}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            {loading ? (
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
                  {deviceFilter !== "all"
                    ? "Try selecting a different device."
                    : "Request a screenshot from a device to get started."}
                </p>
              </div>
            ) : (
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
                {filteredScreenshots.map((ss) => {
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
                            alt={`Screenshot from ${ss.device.hostname || "device"}`}
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
                                  {ss.device.hostname || "Unnamed Device"}
                                </span>
                              </div>
                            </TooltipTrigger>
                            <TooltipContent>
                              <p>{ss.device.hostname || "Unnamed Device"}</p>
                            </TooltipContent>
                          </Tooltip>
                          <Badge
                            variant={getScreenshotStatusBadgeVariant(ss.status)}
                            className="text-[10px] px-1.5 py-0.5 shrink-0"
                          >
                            {getStatusLabel(ss.status)}
                          </Badge>
                        </div>

                        <div className="flex items-center justify-between text-xs text-muted-foreground">
                          <Link href={`/employees/${ss.employee.id}`} className="truncate hover:underline text-blue-600">
                            {ss.employee.first_name} {ss.employee.last_name}
                          </Link>
                          <span className="shrink-0">{timeAgo(ss.created_at)}</span>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

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
                    {viewingScreenshot && (
                      <>
                        {viewingScreenshot.device.hostname || "Unnamed Device"} ·{" "}
                        <Link href={`/employees/${viewingScreenshot.employee.id}`} className="text-blue-600 hover:underline">
                          {viewingScreenshot.employee.first_name}{" "}
                          {viewingScreenshot.employee.last_name}
                        </Link>{" "}
                        ·{" "}
                        {new Date(
                          viewingScreenshot.created_at
                        ).toLocaleString()}
                      </>
                    )}
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
                    alt={`Screenshot from ${viewingScreenshot.device.hostname || "device"}`}
                    className="w-full rounded-lg border"
                    crossOrigin="anonymous"
                  />
                )}
            </div>
          </DialogContent>
        </Dialog>

        {/* Request screenshot dialog */}
        <Dialog
          open={requestDialogOpen}
          onOpenChange={(open) => {
            setRequestDialogOpen(open);
            if (!open) setRequestDeviceId("");
          }}
        >
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Request Screenshot</DialogTitle>
              <DialogDescription>
                Select a device to capture a screenshot from.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-2">
              <Select
                value={requestDeviceId}
                onValueChange={setRequestDeviceId}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Choose a device..." />
                </SelectTrigger>
                <SelectContent>
                  {allDevices.length === 0 && (
                    <div className="px-2 py-4 text-sm text-muted-foreground text-center">
                      No devices available
                    </div>
                  )}
                  {allDevices.map((device) => {
                    const emp = employees.find(
                      (e) => e.id === device.employee_id
                    );
                    return (
                      <SelectItem key={device.id} value={device.id}>
                        {device.hostname || "Unnamed Device"}
                        {emp
                          ? ` (${emp.first_name} ${emp.last_name})`
                          : ""}
                      </SelectItem>
                    );
                  })}
                </SelectContent>
              </Select>
              <div className="flex justify-end gap-2">
                <Button
                  variant="outline"
                  onClick={() => setRequestDialogOpen(false)}
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleRequestScreenshot}
                  disabled={requesting || !requestDeviceId}
                >
                  {requesting ? (
                    <>
                      <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                      Requesting...
                    </>
                  ) : (
                    <>
                      <Camera className="mr-2 h-4 w-4" />
                      Request Screenshot
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
