"use client";

import { useEffect, useState } from "react";
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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Plus, Trash2 } from "lucide-react";
import Link from "next/link";
import { getCompanyId, getToken } from "@/lib/auth/session";
import { listEmployees, registerEmployee, deactivateEmployee, type Employee } from "@/lib/api/employees";
import { listRoles, type Role } from "@/lib/api/roles";
import { useSocket } from "@/lib/socket";
import { SocketIndicator } from "@/components/shared/socket-indicator";
import { EmployeeStatusBadge } from "@/components/shared/status-badge";

export default function EmployeesPage() {
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [email, setEmail] = useState("");
  const [selectedRoleId, setSelectedRoleId] = useState<string>("");
  const [roles, setRoles] = useState<Role[]>([]);
  const [creating, setCreating] = useState(false);
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
    } catch {
      setConfirmOpen(false);
      setSelectedId(null);
    }
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
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Employees</h1>
          <p className="text-muted-foreground">
            Manage employees in your company.
          </p>
        </div>
        <div className="flex items-center gap-3">
          <SocketIndicator isConnected={isConnected} />
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
                  <TableRow key={emp.id}>
                    <TableCell className="font-mono text-sm">
                      {emp.employee_id}
                    </TableCell>
                    <TableCell className="font-medium">
                      {emp.first_name} {emp.last_name}
                    </TableCell>
                    <TableCell>{emp.email || "—"}</TableCell>
                    <TableCell>
                      <EmployeeStatusBadge status={emp.status} />
                    </TableCell>
                    <TableCell>
                      {new Date(emp.created_at).toLocaleDateString()}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex items-center justify-end gap-2">
                        <Link
                          href={`/employees/${emp.id}`}
                          className="text-blue-600 hover:underline text-sm"
                        >
                          View Details
                        </Link>
                        {emp.status === "active" && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => openDeactivateDialog(emp.id)}
                          >
                            <Trash2 className="h-4 w-4 text-destructive" />
                          </Button>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

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
  );
}