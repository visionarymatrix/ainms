"use client";

import { useEffect, useState, useCallback, useMemo } from "react";
import { format, subDays } from "date-fns";
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
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  Tooltip as RechartsTooltip,
  Legend,
} from "recharts";
import {
  Brain,
  CalendarIcon,
  Clock,
  CheckCircle2,
  XCircle,
  MinusCircle,
  User,
} from "lucide-react";
import { toast } from "sonner";
import { getAIActivityAnalysis, type AIActivityAnalysis } from "@/lib/api/ai-activity";
import { listEmployees, type Employee } from "@/lib/api/employees";
import { getCompanyId } from "@/lib/auth/session";
import { formatDuration } from "@/lib/utils/format";
import { ClassificationBadge } from "@/components/shared/status-badge";

const CLASSIFICATION_COLORS: Record<string, string> = {
  productive: "#22c55e",
  unproductive: "#ef4444",
  neutral: "#6b7280",
};

function formatHoursMinutes(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  if (hours === 0 && minutes === 0) return "0m";
  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  return parts.join(" ");
}

export default function AIActivityPage() {
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [selectedEmployeeId, setSelectedEmployeeId] = useState<string>("");
  const [fromDate, setFromDate] = useState<Date>(subDays(new Date(), 7));
  const [toDate, setToDate] = useState<Date>(new Date());
  const [analysis, setAnalysis] = useState<AIActivityAnalysis | null>(null);
  const [loading, setLoading] = useState(false);
  const [employeesLoading, setEmployeesLoading] = useState(true);
  const [fromCalendarOpen, setFromCalendarOpen] = useState(false);
  const [toCalendarOpen, setToCalendarOpen] = useState(false);

  const companyId = getCompanyId();

  const fetchEmployees = useCallback(async () => {
    if (!companyId) {
      setEmployeesLoading(false);
      return;
    }
    setEmployeesLoading(true);
    try {
      const data = await listEmployees(companyId);
      const sorted = (data || []).slice().sort((a, b) =>
        `${a.first_name} ${a.last_name}`.localeCompare(`${b.first_name} ${b.last_name}`)
      );
      setEmployees(sorted);
    } catch {
      toast.error("Failed to load employees");
      setEmployees([]);
    } finally {
      setEmployeesLoading(false);
    }
  }, [companyId]);

  useEffect(() => {
    fetchEmployees();
  }, [fetchEmployees]);

  const handleAnalyze = useCallback(async () => {
    if (!selectedEmployeeId) {
      toast.error("Please select an employee");
      return;
    }
    setLoading(true);
    setAnalysis(null);
    try {
      const data = await getAIActivityAnalysis(selectedEmployeeId, {
        from: format(fromDate, "yyyy-MM-dd") + "T00:00:00Z",
        to: format(toDate, "yyyy-MM-dd") + "T23:59:59Z",
      });
      setAnalysis(data);
    } catch {
      toast.error("Failed to analyze activity. The AI service may be unavailable.");
      setAnalysis(null);
    } finally {
      setLoading(false);
    }
  }, [selectedEmployeeId, fromDate, toDate]);

  const pieData = useMemo(() => {
    if (!analysis) return [];
    return [
      { name: "Productive", value: analysis.productive_duration_sec, color: CLASSIFICATION_COLORS.productive },
      { name: "Unproductive", value: analysis.unproductive_duration_sec, color: CLASSIFICATION_COLORS.unproductive },
      { name: "Neutral", value: analysis.neutral_duration_sec, color: CLASSIFICATION_COLORS.neutral },
    ].filter((d) => d.value > 0);
  }, [analysis]);

  const totalHours = useMemo(() => {
    if (!analysis) return 0;
    return analysis.total_duration_sec / 3600;
  }, [analysis]);

  if (employeesLoading) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-semibold tracking-tight">AI Activity Analysis</h1>
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
      {/* Header */}
      <div>
        <h1 className="text-2xl font-semibold tracking-tight flex items-center gap-2">
          <Brain className="h-6 w-6" />
          AI Activity Analysis
        </h1>
        <p className="text-muted-foreground mt-1">
          AI-powered classification of employee activity into productive, unproductive, and neutral categories.
        </p>
      </div>

      {/* Filters */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {/* Employee selector */}
        <div className="space-y-2">
          <label className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
            <User className="h-3.5 w-3.5" />
            Employee
          </label>
          <Select value={selectedEmployeeId} onValueChange={setSelectedEmployeeId}>
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

        {/* From date */}
        <div className="space-y-2">
          <label className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
            <CalendarIcon className="h-3.5 w-3.5" />
            From
          </label>
          <Popover open={fromCalendarOpen} onOpenChange={setFromCalendarOpen}>
            <PopoverTrigger asChild>
              <Button variant="outline" className="w-full justify-start text-left font-normal">
                <CalendarIcon className="mr-2 h-4 w-4" />
                {format(fromDate, "MMM dd, yyyy")}
              </Button>
            </PopoverTrigger>
            <PopoverContent className="w-auto p-0" align="start">
              <Calendar
                mode="single"
                selected={fromDate}
                onSelect={(d) => {
                  if (d) {
                    setFromDate(d);
                    setFromCalendarOpen(false);
                  }
                }}
                disabled={(d) => d > toDate}
              />
            </PopoverContent>
          </Popover>
        </div>

        {/* To date */}
        <div className="space-y-2">
          <label className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
            <CalendarIcon className="h-3.5 w-3.5" />
            To
          </label>
          <Popover open={toCalendarOpen} onOpenChange={setToCalendarOpen}>
            <PopoverTrigger asChild>
              <Button variant="outline" className="w-full justify-start text-left font-normal">
                <CalendarIcon className="mr-2 h-4 w-4" />
                {format(toDate, "MMM dd, yyyy")}
              </Button>
            </PopoverTrigger>
            <PopoverContent className="w-auto p-0" align="start">
              <Calendar
                mode="single"
                selected={toDate}
                onSelect={(d) => {
                  if (d) {
                    setToDate(d);
                    setToCalendarOpen(false);
                  }
                }}
                disabled={(d) => d < fromDate}
              />
            </PopoverContent>
          </Popover>
        </div>

        {/* Analyze button */}
        <div className="space-y-2">
          <label className="text-sm font-medium text-muted-foreground invisible">Action</label>
          <Button
            className="w-full"
            onClick={handleAnalyze}
            disabled={!selectedEmployeeId || loading}
          >
            {loading ? (
              <>
                <Brain className="mr-2 h-4 w-4 animate-pulse" />
                Analyzing...
              </>
            ) : (
              <>
                <Brain className="mr-2 h-4 w-4" />
                Analyze Activity
              </>
            )}
          </Button>
        </div>
      </div>

      {/* Results */}
      {analysis ? (
        <>
          {/* Project Context Banner */}
          {analysis.project_name ? (
            <Card className="border-l-4 border-l-primary">
              <CardContent className="py-4">
                <div className="flex items-start gap-3">
                  <CheckCircle2 className="h-5 w-5 text-primary mt-0.5 shrink-0" />
                  <div>
                    <p className="font-medium">
                      Working on: {analysis.project_name}
                    </p>
                    <p className="text-sm text-muted-foreground mt-0.5">
                      Approved apps: {analysis.working_apps.join(", ")}
                    </p>
                  </div>
                </div>
              </CardContent>
            </Card>
          ) : (
            <Card className="border-l-4 border-l-muted-foreground/40">
              <CardContent className="py-4">
                <div className="flex items-start gap-3">
                  <MinusCircle className="h-5 w-5 text-muted-foreground mt-0.5 shrink-0" />
                  <p className="text-sm text-muted-foreground">
                    No project assigned — analysis based on role only
                  </p>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Summary Cards */}
          <div className="grid gap-4 md:grid-cols-3">
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                  <CheckCircle2 className="h-4 w-4" style={{ color: CLASSIFICATION_COLORS.productive }} />
                  Productive
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-3xl font-bold">
                  {formatHoursMinutes(analysis.productive_duration_sec)}
                </div>
                <p className="text-sm text-muted-foreground mt-1">
                  {analysis.productive_pct.toFixed(1)}% of tracked time
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                  <XCircle className="h-4 w-4" style={{ color: CLASSIFICATION_COLORS.unproductive }} />
                  Unproductive
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-3xl font-bold">
                  {formatHoursMinutes(analysis.unproductive_duration_sec)}
                </div>
                <p className="text-sm text-muted-foreground mt-1">
                  {analysis.unproductive_pct.toFixed(1)}% of tracked time
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                  <MinusCircle className="h-4 w-4" style={{ color: CLASSIFICATION_COLORS.neutral }} />
                  Neutral
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-3xl font-bold">
                  {formatHoursMinutes(analysis.neutral_duration_sec)}
                </div>
                <p className="text-sm text-muted-foreground mt-1">
                  {analysis.neutral_pct.toFixed(1)}% of tracked time
                </p>
              </CardContent>
            </Card>
          </div>

          {/* Pie Chart */}
          <div className="grid gap-4 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <PieChart className="h-5 w-5" />
                  Activity Breakdown
                </CardTitle>
                <CardDescription>
                  Productive vs unproductive vs neutral time distribution
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="h-80 relative">
                  <ResponsiveContainer width="100%" height="100%">
                    <PieChart>
                      <Pie
                        data={pieData}
                        dataKey="value"
                        nameKey="name"
                        cx="50%"
                        cy="50%"
                        innerRadius={60}
                        outerRadius={100}
                        label={({ name, percent }: { name?: string; percent?: number }) =>
                          `${name ?? ""} ${((percent ?? 0) * 100).toFixed(0)}%`
                        }
                        labelLine
                      >
                        {pieData.map((entry, index) => (
                          <Cell key={`cell-${index}`} fill={entry.color} />
                        ))}
                      </Pie>
                      <RechartsTooltip
                        formatter={(value: unknown) => formatDuration(value as number)}
                        contentStyle={{ borderRadius: "6px", fontSize: 12 }}
                      />
                      <Legend
                        verticalAlign="bottom"
                        height={36}
                        wrapperStyle={{ fontSize: 12 }}
                      />
                    </PieChart>
                  </ResponsiveContainer>
                  {/* Center label */}
                  <div className="absolute inset-0 flex items-center justify-center pointer-events-none" style={{ top: "-10%" }}>
                    <div className="text-center">
                      <div className="text-2xl font-bold">{totalHours.toFixed(1)}h</div>
                      <div className="text-xs text-muted-foreground">total</div>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* AI-Annotated Activity Table */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Brain className="h-5 w-5" />
                  AI-Annotated Activities
                </CardTitle>
                <CardDescription>
                  Each event classified by AI with reasoning
                </CardDescription>
              </CardHeader>
              <CardContent>
                {analysis.events.length === 0 ? (
                  <div className="py-8 text-center text-muted-foreground">
                    <Clock className="mx-auto h-8 w-8 mb-2" />
                    <p className="text-sm">No activity events found for this period.</p>
                  </div>
                ) : (
                  <div className="max-h-80 overflow-y-auto">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>App</TableHead>
                          <TableHead>Window Title</TableHead>
                          <TableHead>Duration</TableHead>
                          <TableHead>Classification</TableHead>
                          <TableHead>AI Reason</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {analysis.events.map((event, idx) => (
                          <TableRow key={idx}>
                            <TableCell className="font-medium text-sm">
                              {event.app_name}
                            </TableCell>
                            <TableCell className="text-sm text-muted-foreground max-w-[200px] truncate" title={event.window_title}>
                              {event.window_title}
                            </TableCell>
                            <TableCell className="text-sm whitespace-nowrap">
                              {formatDuration(event.duration_sec)}
                            </TableCell>
                            <TableCell>
                              <ClassificationBadge classification={event.classification} />
                            </TableCell>
                            <TableCell className="text-xs text-muted-foreground max-w-[250px] truncate" title={event.reason}>
                              {event.reason}
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </div>
                )}
              </CardContent>
            </Card>
          </div>
        </>
      ) : loading ? (
        <div className="space-y-4">
          <div className="grid gap-4 md:grid-cols-3">
            {[1, 2, 3].map((i) => (
              <Skeleton key={i} className="h-32" />
            ))}
          </div>
          <Skeleton className="h-80" />
        </div>
      ) : (
        <Card>
          <CardContent className="py-12 text-center">
            <Brain className="mx-auto h-10 w-10 text-muted-foreground" />
            <h3 className="mt-4 text-lg font-semibold">No analysis yet</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Select an employee and date range to analyze activity.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}