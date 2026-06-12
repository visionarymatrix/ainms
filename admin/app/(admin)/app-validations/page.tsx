"use client";

import { useEffect, useState, useCallback, useMemo } from "react";
import Link from "next/link";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { getCompanyId } from "@/lib/auth/session";
import {
  getEmployeeValidations,
  getCompanyValidations,
  getCompanyNonCompliantApps,
  type InstalledAppValidationWithDetails,
} from "@/lib/api/installed-apps";
import { listEmployees, type Employee } from "@/lib/api/employees";
import {
  ShieldCheck,
  User,
  LayoutGrid,
  CheckCircle2,
  XCircle,
  MinusCircle,
  AlertCircle,
} from "lucide-react";
import { toast } from "sonner";

const CLASSIFICATION_COLORS: Record<string, string> = {
  productive: "#22c55e",
  unproductive: "#ef4444",
  neutral: "#6b7280",
};

function getCategoryBadgeStyle(category: string): React.CSSProperties {
  const color = CLASSIFICATION_COLORS[category] || CLASSIFICATION_COLORS.neutral;
  return {
    backgroundColor: color,
    color: "#ffffff",
  };
}

export default function AppValidationsPage() {
  const [mounted, setMounted] = useState(false);
  const companyId = mounted ? getCompanyId() : null;

  const [employees, setEmployees] = useState<Employee[]>([]);
  const [selectedEmployeeId, setSelectedEmployeeId] = useState<string>("__all__");
  const [showNonCompliantOnly, setShowNonCompliantOnly] = useState(false);
  const [validations, setValidations] = useState<InstalledAppValidationWithDetails[]>([]);

  const [loadingEmployees, setLoadingEmployees] = useState(false);
  const [loadingValidations, setLoadingValidations] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  const fetchEmployees = useCallback(async (cid: string) => {
    if (!cid) return;
    setLoadingEmployees(true);
    try {
      const data = await listEmployees(cid);
      const sorted = (data || [])
        .slice()
        .sort((a, b) =>
          `${a.first_name} ${a.last_name}`.localeCompare(`${b.first_name} ${b.last_name}`)
        );
      setEmployees(sorted);
    } catch {
      toast.error("Failed to load employees");
      setEmployees([]);
    } finally {
      setLoadingEmployees(false);
    }
  }, []);

  const fetchCompanyValidations = useCallback(async (cid: string) => {
    if (!cid) return;
    setLoadingValidations(true);
    try {
      const data = await getCompanyValidations(cid);
      setValidations(data || []);
    } catch {
      toast.error("Failed to load validations");
      setValidations([]);
    } finally {
      setLoadingValidations(false);
    }
  }, []);

  const fetchCompanyNonCompliant = useCallback(async (cid: string) => {
    if (!cid) return;
    setLoadingValidations(true);
    try {
      const data = await getCompanyNonCompliantApps(cid);
      setValidations(data || []);
    } catch {
      toast.error("Failed to load non-compliant apps");
      setValidations([]);
    } finally {
      setLoadingValidations(false);
    }
  }, []);

  const fetchEmployeeValidations = useCallback(async (eid: string) => {
    if (!eid) return;
    setLoadingValidations(true);
    try {
      const data = await getEmployeeValidations(eid);
      setValidations(data || []);
    } catch {
      toast.error("Failed to load employee validations");
      setValidations([]);
    } finally {
      setLoadingValidations(false);
    }
  }, []);

  useEffect(() => {
    if (companyId) {
      fetchEmployees(companyId);
      fetchCompanyValidations(companyId);
    }
  }, [companyId, fetchEmployees, fetchCompanyValidations]);

  useEffect(() => {
    if (selectedEmployeeId && selectedEmployeeId !== "__all__") {
      fetchEmployeeValidations(selectedEmployeeId);
    } else if (companyId) {
      fetchCompanyValidations(companyId);
    }
  }, [selectedEmployeeId, companyId, fetchEmployeeValidations, fetchCompanyValidations]);

  const stats = useMemo(() => {
    const total = validations.length;
    const compliant = validations.filter((v) => v.is_compliant).length;
    const nonCompliant = validations.filter((v) => !v.is_compliant).length;
    const noRule = validations.filter((v) => !v.role_id).length;
    return { total, compliant, nonCompliant, noRule };
  }, [validations]);

  const filteredValidations = useMemo(() => {
    if (showNonCompliantOnly) {
      return validations.filter((v) => !v.is_compliant);
    }
    return validations;
  }, [validations, showNonCompliantOnly]);

  const isLoading = loadingEmployees || loadingValidations;

  if (!companyId) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-semibold tracking-tight">App Validations</h1>
        <Card>
          <CardContent className="py-12 text-center">
            <AlertCircle className="mx-auto h-10 w-10 text-muted-foreground" />
            <h3 className="mt-4 text-lg font-semibold">No company selected</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Unable to determine your company. Please log in again.
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">App Validations</h1>
          <p className="text-muted-foreground">
            Review application classification validations and compliance per employee.
          </p>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-2">
          <label className="text-sm font-medium text-muted-foreground flex items-center gap-1.5">
            <User className="h-3.5 w-3.5" />
            Employee
          </label>
          <Select
            value={selectedEmployeeId}
            onValueChange={setSelectedEmployeeId}
            disabled={loadingEmployees}
          >
            <SelectTrigger className="w-full">
              <SelectValue placeholder="All employees" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__all__">All employees</SelectItem>
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
            <AlertCircle className="h-3.5 w-3.5" />
            Compliance Filter
          </label>
          <Button
            variant={showNonCompliantOnly ? "default" : "outline"}
            onClick={() => setShowNonCompliantOnly(!showNonCompliantOnly)}
            className="w-full justify-start"
          >
            {showNonCompliantOnly ? (
              <>
                <XCircle className="h-4 w-4 mr-2" />
                Non-compliant only
              </>
            ) : (
              <>
                <LayoutGrid className="h-4 w-4 mr-2" />
                All apps ({validations.length})
              </>
            )}
          </Button>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <LayoutGrid className="h-4 w-4" />
              Total Apps
            </CardTitle>
          </CardHeader>
          <CardContent>
            {loadingValidations ? (
              <Skeleton className="h-9 w-16" />
            ) : (
              <div className="text-3xl font-bold">{stats.total}</div>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <CheckCircle2 className="h-4 w-4" />
              Compliant
            </CardTitle>
          </CardHeader>
          <CardContent>
            {loadingValidations ? (
              <Skeleton className="h-9 w-16" />
            ) : (
              <div className="text-3xl font-bold">{stats.compliant}</div>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <XCircle className="h-4 w-4" />
              Non-Compliant
            </CardTitle>
          </CardHeader>
          <CardContent>
            {loadingValidations ? (
              <Skeleton className="h-9 w-16" />
            ) : (
              <div className="text-3xl font-bold">{stats.nonCompliant}</div>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <MinusCircle className="h-4 w-4" />
              No Rule
            </CardTitle>
          </CardHeader>
          <CardContent>
            {loadingValidations ? (
              <Skeleton className="h-9 w-16" />
            ) : (
              <div className="text-3xl font-bold">{stats.noRule}</div>
            )}
          </CardContent>
        </Card>
      </div>

      {loadingValidations && filteredValidations.length === 0 ? (
        <Card>
          <CardHeader>
            <Skeleton className="h-6 w-48" />
            <Skeleton className="h-4 w-72" />
          </CardHeader>
          <CardContent>
            <Skeleton className="h-64 w-full" />
          </CardContent>
        </Card>
      ) : filteredValidations.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            <ShieldCheck className="mx-auto h-10 w-10 text-muted-foreground" />
            <h3 className="mt-4 text-lg font-semibold">No validations found</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              {showNonCompliantOnly
                ? "No non-compliant applications found."
                : selectedEmployeeId !== "__all__"
                ? "No application validations available for this employee."
                : "No application validations found across the company."}
            </p>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <ShieldCheck className="h-5 w-5" />
              Application Validations
            </CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>App Name</TableHead>
                  <TableHead>Employee</TableHead>
                  <TableHead>Device</TableHead>
                  <TableHead>Role</TableHead>
                  <TableHead>Agent Category</TableHead>
                  <TableHead>Validated Category</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Reason</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredValidations.map((v) => (
                  <TableRow key={v.id}>
                    <TableCell className="font-medium">{v.display_name || v.app_name}</TableCell>
                    <TableCell>
                      {v.employee_id ? (
                        <Link href={`/employees/${v.employee_id}`} className="text-blue-600 hover:underline">
                          {v.employee_name}
                        </Link>
                      ) : (
                        v.employee_name
                      )}
                    </TableCell>
                    <TableCell>{v.device_hostname || "—"}</TableCell>
                    <TableCell>{v.role_name || "—"}</TableCell>
                    <TableCell>
                      <Badge
                        className="rounded-full"
                        style={getCategoryBadgeStyle(v.agent_category)}
                      >
                        {v.agent_category}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <Badge
                        className="rounded-full"
                        style={getCategoryBadgeStyle(v.validated_category)}
                      >
                        {v.validated_category}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      {v.is_compliant ? (
                        <CheckCircle2 className="h-5 w-5 text-green-500" />
                      ) : (
                        <XCircle className="h-5 w-5 text-red-500" />
                      )}
                    </TableCell>
                    <TableCell className="max-w-xs truncate" title={v.reason}>
                      {v.reason || "—"}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
