"use client";

import { useEffect, useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
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
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  listCompanyAlerts,
  ackAlert,
  type ComplianceAlert,
  type PopupAnswer,
} from "@/lib/api/alerts";
import { listEmployees, type Employee } from "@/lib/api/employees";
import { getUser } from "@/lib/auth/session";
import { timeAgo } from "@/lib/utils/format";
import { toast } from "sonner";
import {
  ShieldAlert,
  CheckCircle2,
  ThumbsUp,
  AlertTriangle,
  ChevronDown,
  ChevronUp,
  RefreshCw,
  Search,
} from "lucide-react";

function decisionBadge(decision: string) {
  switch (decision) {
    case "violation":
      return (
        <Badge
          variant="outline"
          className="bg-red-100 text-red-800 border-red-200 hover:bg-red-100"
        >
          Violation
        </Badge>
      );
    case "productive":
      return (
        <Badge
          variant="outline"
          className="bg-emerald-100 text-emerald-800 border-emerald-200 hover:bg-emerald-100"
        >
          Productive
        </Badge>
      );
    case "shoutout":
      return (
        <Badge
          variant="outline"
          className="bg-blue-100 text-blue-800 border-blue-200 hover:bg-blue-100"
        >
          Shoutout
        </Badge>
      );
    default:
      return <Badge variant="outline">{decision}</Badge>;
  }
}

function statusBadge(status: string) {
  switch (status) {
    case "pending":
      return (
        <Badge
          variant="outline"
          className="bg-amber-100 text-amber-800 border-amber-200 hover:bg-amber-100"
        >
          Pending
        </Badge>
      );
    case "delivered":
      return (
        <Badge
          variant="outline"
          className="bg-blue-100 text-blue-800 border-blue-200 hover:bg-blue-100"
        >
          Delivered
        </Badge>
      );
    case "acked":
      return (
        <Badge
          variant="outline"
          className="bg-emerald-100 text-emerald-800 border-emerald-200 hover:bg-emerald-100"
        >
          Acked
        </Badge>
      );
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
}

function popupTypeBadge(type: string) {
  switch (type) {
    case "toast":
      return (
        <Badge
          variant="outline"
          className="bg-sky-100 text-sky-800 border-sky-200 hover:bg-sky-100 text-[10px]"
        >
          Toast
        </Badge>
      );
    case "modal":
      return (
        <Badge
          variant="outline"
          className="bg-violet-100 text-violet-800 border-violet-200 hover:bg-violet-100 text-[10px]"
        >
          Modal
        </Badge>
      );
    case "soft_block":
      return (
        <Badge
          variant="outline"
          className="bg-orange-100 text-orange-800 border-orange-200 hover:bg-orange-100 text-[10px]"
        >
          Soft Block
        </Badge>
      );
    default:
      return (
        <Badge variant="outline" className="text-[10px]">
          {type}
        </Badge>
      );
  }
}

function PopupAnswerRow({ answer }: { answer: PopupAnswer }) {
  return (
    <div className="ml-4 mt-2 border-l-2 border-muted pl-4 space-y-2 text-sm">
      <div className="flex items-center gap-2 flex-wrap">
        <span className="text-muted-foreground font-medium">User response:</span>
        {decisionBadge(answer.decision)}
        {popupTypeBadge(answer.popup_type)}
      </div>
      {answer.explanation && (
        <p className="text-muted-foreground italic">&ldquo;{answer.explanation}&rdquo;</p>
      )}
      <div className="flex items-center gap-4 flex-wrap text-xs text-muted-foreground">
        {answer.app_name && (
          <span>
            App: <span className="font-medium text-foreground">{answer.app_name}</span>
          </span>
        )}
        {answer.window_title && (
          <span>
            Window: <span className="font-medium text-foreground">{answer.window_title}</span>
          </span>
        )}
        {answer.classification && (
          <span>
            Classification: <span className="font-medium text-foreground">{answer.classification}</span>
          </span>
        )}
        {answer.confidence > 0 && (
          <span>
            Confidence:{" "}
            <span className="font-medium text-foreground">
              {(answer.confidence * 100).toFixed(0)}%
            </span>
          </span>
        )}
        {answer.event_time && (
          <span>Answered: {timeAgo(answer.event_time)}</span>
        )}
      </div>
    </div>
  );
}

export default function AlertsPage() {
  const [alerts, setAlerts] = useState<ComplianceAlert[]>([]);
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const [employeeFilter, setEmployeeFilter] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [expandedRows, setExpandedRows] = useState<Set<string>>(new Set());
  const [ackingIds, setAckingIds] = useState<Set<string>>(new Set());
  const [refreshing, setRefreshing] = useState(false);

  const user = getUser();

  const fetchAlerts = useCallback(async () => {
    if (!user?.company_id) return;
    try {
      const statusParam = statusFilter !== "all" ? statusFilter : undefined;
      const data = await listCompanyAlerts(user.company_id, statusParam);
      setAlerts(data || []);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load alerts");
      setAlerts([]);
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, [user?.company_id, statusFilter]);

  const fetchEmployees = useCallback(async () => {
    if (!user?.company_id) return;
    try {
      const data = await listEmployees(user.company_id);
      setEmployees(data || []);
    } catch {
      // non-fatal
    }
  }, [user?.company_id]);

  useEffect(() => {
    fetchAlerts();
    fetchEmployees();
  }, [fetchAlerts, fetchEmployees]);

  useEffect(() => {
    const interval = setInterval(fetchAlerts, 30000);
    return () => clearInterval(interval);
  }, [fetchAlerts]);

  async function handleAck(alertId: string) {
    setAckingIds((prev) => new Set(prev).add(alertId));
    try {
      await ackAlert(alertId);
      toast.success("Alert acknowledged");
      await fetchAlerts();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to acknowledge alert");
    } finally {
      setAckingIds((prev) => {
        const next = new Set(prev);
        next.delete(alertId);
        return next;
      });
    }
  }

  function handleRefresh() {
    setRefreshing(true);
    fetchAlerts();
  }

  function toggleRow(id: string) {
    setExpandedRows((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  const filtered = alerts.filter((a) => {
    if (employeeFilter !== "all" && a.employee_id !== employeeFilter) return false;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      return (
        a.message?.toLowerCase().includes(q) ||
        a.employee_name?.toLowerCase().includes(q) ||
        a.device_hostname?.toLowerCase().includes(q)
      );
    }
    return true;
  });

  const totalViolations = alerts.filter((a) => a.decision === "violation").length;
  const totalProductive = alerts.filter((a) => a.decision === "productive").length;
  const totalPending = alerts.filter((a) => a.status === "pending").length;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Alerts</h1>
          <p className="text-muted-foreground">
            View and manage compliance alerts across the fleet.
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={handleRefresh}
          disabled={refreshing}
        >
          <RefreshCw className={`mr-2 h-4 w-4 ${refreshing ? "animate-spin" : ""}`} />
          {refreshing ? "Refreshing..." : "Refresh"}
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Total Alerts
            </CardTitle>
          </CardHeader>
          <CardContent>
            {loading ? (
              <Skeleton className="h-9 w-16" />
            ) : (
              <div className="text-3xl font-bold">{alerts.length}</div>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
              <ShieldAlert className="h-3.5 w-3.5 text-red-500" />
              Violations
            </CardTitle>
          </CardHeader>
          <CardContent>
            {loading ? (
              <Skeleton className="h-9 w-16" />
            ) : (
              <div className="text-3xl font-bold text-red-600">
                {totalViolations}
              </div>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
              <ThumbsUp className="h-3.5 w-3.5 text-emerald-500" />
              Productive
            </CardTitle>
          </CardHeader>
          <CardContent>
            {loading ? (
              <Skeleton className="h-9 w-16" />
            ) : (
              <div className="text-3xl font-bold text-emerald-600">
                {totalProductive}
              </div>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
              <AlertTriangle className="h-3.5 w-3.5 text-amber-500" />
              Pending
            </CardTitle>
          </CardHeader>
          <CardContent>
            {loading ? (
              <Skeleton className="h-9 w-16" />
            ) : (
              <div className="text-3xl font-bold text-amber-600">
                {totalPending}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="flex flex-col sm:flex-row items-start sm:items-center gap-3">
        <Select value={statusFilter} onValueChange={setStatusFilter}>
          <SelectTrigger className="w-[160px]">
            <SelectValue placeholder="All statuses" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Statuses</SelectItem>
            <SelectItem value="pending">Pending</SelectItem>
            <SelectItem value="delivered">Delivered</SelectItem>
            <SelectItem value="acked">Acked</SelectItem>
          </SelectContent>
        </Select>

        {employees.length > 0 && (
          <Select value={employeeFilter} onValueChange={setEmployeeFilter}>
            <SelectTrigger className="w-[200px]">
              <SelectValue placeholder="All employees" />
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
        )}

        <div className="relative flex-1 min-w-[200px]">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search messages, employees, devices..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-8"
          />
        </div>
      </div>

      {error && (
        <Card className="border-red-200 bg-red-50">
          <CardContent className="pt-4 flex items-center gap-2">
            <AlertTriangle className="h-4 w-4 text-red-800" />
            <p className="text-sm text-red-800">{error}</p>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>
            Compliance Alerts
            {!loading && (
              <span className="ml-2 text-sm font-normal text-muted-foreground">
                {filtered.length} alert{filtered.length !== 1 ? "s" : ""}
              </span>
            )}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-16 w-full" />
              ))}
            </div>
          ) : filtered.length === 0 ? (
            <div className="py-12 text-center">
              <ShieldAlert className="mx-auto h-12 w-12 text-muted-foreground/40 mb-4" />
              <p className="text-muted-foreground text-sm">
                No alerts yet. Alerts are generated when AI compliance analysis
                detects policy violations.
              </p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-10" />
                  <TableHead>Employee</TableHead>
                  <TableHead>Device</TableHead>
                  <TableHead>Decision</TableHead>
                  <TableHead>Message</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead>Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filtered.map((alert) => {
                  const isExpanded = expandedRows.has(alert.id);
                  const canAck =
                    (alert.status === "pending" || alert.status === "delivered") &&
                    !ackingIds.has(alert.id);
                  const isAcking = ackingIds.has(alert.id);

                  return (
                    <>
                      <TableRow
                        key={alert.id}
                        className={`cursor-pointer hover:bg-muted/50 ${alert.decision === "violation" ? "border-l-2 border-l-red-300" : ""}`}
                        onClick={() => toggleRow(alert.id)}
                      >
                        <TableCell>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6"
                          >
                            {isExpanded ? (
                              <ChevronUp className="h-4 w-4" />
                            ) : (
                              <ChevronDown className="h-4 w-4" />
                            )}
                          </Button>
                        </TableCell>
                        <TableCell className="font-medium">
                          {alert.employee_name || alert.employee_id.slice(0, 8) + "..."}
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground">
                          {alert.device_hostname || alert.device_id.slice(0, 8) + "..."}
                        </TableCell>
                        <TableCell>{decisionBadge(alert.decision)}</TableCell>
                        <TableCell className="max-w-xs truncate" title={alert.message}>
                          {alert.message}
                        </TableCell>
                        <TableCell>{statusBadge(alert.status)}</TableCell>
                        <TableCell className="text-xs text-muted-foreground whitespace-nowrap">
                          {timeAgo(alert.created_at)}
                        </TableCell>
                        <TableCell>
                          {canAck && (
                            <Button
                              variant="outline"
                              size="sm"
                              className="h-7 text-xs border-emerald-200 bg-emerald-50 text-emerald-700 hover:bg-emerald-100 hover:text-emerald-800"
                              disabled={isAcking}
                              onClick={(e) => {
                                e.stopPropagation();
                                handleAck(alert.id);
                              }}
                            >
                              <CheckCircle2 className="mr-1 h-3 w-3" />
                              {isAcking ? "Acking..." : "Acknowledge"}
                            </Button>
                          )}
                          {alert.status === "acked" && (
                            <Badge variant="outline" className="bg-emerald-50 text-emerald-600 border-emerald-200 text-[10px]">
                              <CheckCircle2 className="mr-1 h-3 w-3" />
                              Acked
                            </Badge>
                          )}
                        </TableCell>
                      </TableRow>
                      {isExpanded && (
                        <TableRow className="bg-muted/30">
                          <TableCell colSpan={8} className="p-4">
                            <div className="space-y-3">
                              <div>
                                <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                                  Full Message
                                </span>
                                <p className="mt-1 text-sm">
                                  {alert.message}
                                </p>
                              </div>
                              {alert.model_used && (
                                <div className="text-xs text-muted-foreground">
                                  Model: <span className="font-mono">{alert.model_used}</span>
                                </div>
                              )}
                              {alert.popup_answer && (
                                <div>
                                  <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                                    Popup Answer
                                  </span>
                                  <PopupAnswerRow answer={alert.popup_answer} />
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
    </div>
  );
}