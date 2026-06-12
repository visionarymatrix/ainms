"use client";

import { useEffect, useState, useCallback, useMemo } from "react";
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
  DialogFooter,
  DialogHeader,
  DialogTitle,
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
import {
  Folder,
  Plus,
  Pencil,
  Trash2,
  Users,
  CheckCircle,
  XCircle,
  Tag,
  ArrowLeft,
  X,
} from "lucide-react";
import { toast } from "sonner";
import { getCompanyId } from "@/lib/auth/session";
import {
  listProjects,
  createProject,
  updateProject,
  deleteProject,
  assignEmployeeToProject,
  unassignEmployeeFromProject,
  type Project,
  type EmployeeProjectAssignment,
} from "@/lib/api/projects";
import {
  listEmployees,
  type Employee,
} from "@/lib/api/employees";

interface ProjectAssignment extends EmployeeProjectAssignment {
  employee?: Employee;
}

export default function ProjectsPage() {
  const companyId = getCompanyId();

  const [projects, setProjects] = useState<Project[]>([]);
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [loading, setLoading] = useState(true);

  // Filter
  const [statusFilter, setStatusFilter] = useState<string>("all");

  // Detail view
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [assignments, setAssignments] = useState<ProjectAssignment[]>([]);
  const [loadingAssignments, setLoadingAssignments] = useState(false);

  // Create dialog
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [newWorkingApps, setNewWorkingApps] = useState<string[]>([]);
  const [newAppInput, setNewAppInput] = useState("");

  // Edit dialog
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [editingProject, setEditingProject] = useState<Project | null>(null);
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [editStatus, setEditStatus] = useState("active");
  const [editWorkingApps, setEditWorkingApps] = useState<string[]>([]);
  const [editAppInput, setEditAppInput] = useState("");
  const [saving, setSaving] = useState(false);

  // Delete dialog
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [deletingProjectId, setDeletingProjectId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);

  // Assign dialog
  const [assignDialogOpen, setAssignDialogOpen] = useState(false);
  const [assignEmployeeId, setAssignEmployeeId] = useState("");
  const [assignIsPrimary, setAssignIsPrimary] = useState(false);
  const [assigning, setAssigning] = useState(false);

  // Unassign confirmation
  const [unassignDialogOpen, setUnassignDialogOpen] = useState(false);
  const [unassigningEmployeeId, setUnassigningEmployeeId] = useState<string | null>(null);
  const [unassigningEmployeeName, setUnassigningEmployeeName] = useState("");
  const [unassigning, setUnassigning] = useState(false);

  const employeeMap = useMemo(() => {
    const map = new Map<string, Employee>();
    for (const e of employees) map.set(e.id, e);
    return map;
  }, [employees]);

  const fetchProjects = useCallback(async () => {
    if (!companyId) return;
    setLoading(true);
    try {
      const data = await listProjects(companyId);
      setProjects(data || []);
    } catch {
      toast.error("Failed to load projects");
      setProjects([]);
    } finally {
      setLoading(false);
    }
  }, [companyId]);

  const fetchEmployees = useCallback(async () => {
    if (!companyId) return;
    try {
      const data = await listEmployees(companyId);
      setEmployees(data || []);
    } catch {
      setEmployees([]);
    }
  }, [companyId]);

  useEffect(() => {
    if (companyId) {
      fetchProjects();
      fetchEmployees();
    } else {
      setLoading(false);
    }
  }, [companyId, fetchProjects, fetchEmployees]);

  const fetchAssignments = useCallback(async (projectId: string) => {
    setLoadingAssignments(true);
    try {
      // List all employees and check their project assignments
      const assignmentPromises = employees.map(async (emp) => {
        try {
          const empProjects = await import("@/lib/api/projects").then((m) =>
            m.listEmployeeProjects(emp.id)
          );
          const match = empProjects.find((a) => a.project_id === projectId);
          if (match) {
            return { ...match, employee: emp } as ProjectAssignment;
          }
        } catch {
          // ignore
        }
        return null;
      });
      const results = await Promise.all(assignmentPromises);
      setAssignments(results.filter((a): a is ProjectAssignment => a !== null));
    } catch {
      setAssignments([]);
    } finally {
      setLoadingAssignments(false);
    }
  }, [employees]);

  useEffect(() => {
    if (selectedProject) {
      fetchAssignments(selectedProject.id);
    }
  }, [selectedProject, fetchAssignments]);

  const filteredProjects = useMemo(() => {
    if (statusFilter === "all") return projects;
    return projects.filter((p) => p.status === statusFilter);
  }, [projects, statusFilter]);

  // ── Create handlers ──
  function addNewApp() {
    const trimmed = newAppInput.trim();
    if (trimmed && !newWorkingApps.includes(trimmed)) {
      setNewWorkingApps([...newWorkingApps, trimmed]);
      setNewAppInput("");
    }
  }

  function removeNewApp(app: string) {
    setNewWorkingApps(newWorkingApps.filter((a) => a !== app));
  }

  async function handleCreate() {
    if (!newName.trim()) {
      toast.error("Project name is required");
      return;
    }
    setCreating(true);
    try {
      await createProject({
        company_id: companyId || undefined,
        name: newName.trim(),
        description: newDescription.trim() || undefined,
        working_apps: newWorkingApps.length > 0 ? newWorkingApps : undefined,
      });
      toast.success("Project created");
      setCreateDialogOpen(false);
      resetCreateForm();
      fetchProjects();
    } catch {
      toast.error("Failed to create project");
    } finally {
      setCreating(false);
    }
  }

  function resetCreateForm() {
    setNewName("");
    setNewDescription("");
    setNewWorkingApps([]);
    setNewAppInput("");
  }

  // ── Edit handlers ──
  function openEditDialog(project: Project) {
    setEditingProject(project);
    setEditName(project.name);
    setEditDescription(project.description || "");
    setEditStatus(project.status);
    setEditWorkingApps(project.working_apps || []);
    setEditAppInput("");
    setEditDialogOpen(true);
  }

  function addEditApp() {
    const trimmed = editAppInput.trim();
    if (trimmed && !editWorkingApps.includes(trimmed)) {
      setEditWorkingApps([...editWorkingApps, trimmed]);
      setEditAppInput("");
    }
  }

  function removeEditApp(app: string) {
    setEditWorkingApps(editWorkingApps.filter((a) => a !== app));
  }

  async function handleEdit() {
    if (!editingProject) return;
    if (!editName.trim()) {
      toast.error("Project name is required");
      return;
    }
    setSaving(true);
    try {
      await updateProject(editingProject.id, {
        name: editName.trim(),
        description: editDescription.trim() || undefined,
        status: editStatus,
        working_apps: editWorkingApps,
      });
      toast.success("Project updated");
      setEditDialogOpen(false);
      setEditingProject(null);
      fetchProjects();
    } catch {
      toast.error("Failed to update project");
    } finally {
      setSaving(false);
    }
  }

  // ── Delete handlers ──
  function openDeleteDialog(projectId: string) {
    setDeletingProjectId(projectId);
    setDeleteDialogOpen(true);
  }

  async function handleDelete() {
    if (!deletingProjectId) return;
    setDeleting(true);
    try {
      await deleteProject(deletingProjectId);
      toast.success("Project deleted");
      setDeleteDialogOpen(false);
      setDeletingProjectId(null);
      if (selectedProject?.id === deletingProjectId) {
        setSelectedProject(null);
      }
      fetchProjects();
    } catch {
      toast.error("Failed to delete project");
    } finally {
      setDeleting(false);
    }
  }

  // ── Assign handlers ──
  async function handleAssign() {
    if (!selectedProject || !assignEmployeeId) {
      toast.error("Select an employee");
      return;
    }
    setAssigning(true);
    try {
      await assignEmployeeToProject(selectedProject.id, {
        employee_id: assignEmployeeId,
        is_primary: assignIsPrimary,
      });
      toast.success("Employee assigned");
      setAssignDialogOpen(false);
      setAssignEmployeeId("");
      setAssignIsPrimary(false);
      fetchAssignments(selectedProject.id);
    } catch {
      toast.error("Failed to assign employee");
    } finally {
      setAssigning(false);
    }
  }

  function openUnassignDialog(employeeId: string, employeeName: string) {
    setUnassigningEmployeeId(employeeId);
    setUnassigningEmployeeName(employeeName);
    setUnassignDialogOpen(true);
  }

  async function handleUnassign() {
    if (!selectedProject || !unassigningEmployeeId) return;
    setUnassigning(true);
    try {
      await unassignEmployeeFromProject(selectedProject.id, unassigningEmployeeId);
      toast.success("Employee removed from project");
      setUnassignDialogOpen(false);
      setUnassigningEmployeeId(null);
      fetchAssignments(selectedProject.id);
    } catch {
      toast.error("Failed to remove employee");
    } finally {
      setUnassigning(false);
    }
  }

  // ── Helpers ──
  function getStatusBadge(status: string) {
    switch (status) {
      case "active":
        return (
          <Badge className="bg-emerald-100 text-emerald-700 border-emerald-200 text-[10px] px-1.5 py-0.5">
            <CheckCircle className="mr-1 h-3 w-3" />
            Active
          </Badge>
        );
      case "archived":
        return (
          <Badge variant="secondary" className="text-[10px] px-1.5 py-0.5">
            <XCircle className="mr-1 h-3 w-3" />
            Archived
          </Badge>
        );
      default:
        return <Badge variant="outline" className="text-[10px]">{status}</Badge>;
    }
  }

  // Employees not yet assigned to current project
  const assignedEmployeeIds = useMemo(
    () => new Set(assignments.map((a) => a.employee_id)),
    [assignments]
  );
  const availableEmployees = useMemo(
    () => employees.filter((e) => !assignedEmployeeIds.has(e.id)),
    [employees, assignedEmployeeIds]
  );

  if (!companyId) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Projects</h1>
          <p className="text-muted-foreground">Manage company projects.</p>
        </div>
        <Card>
          <CardContent className="py-12 text-center">
            <p className="text-muted-foreground">No company assigned. Contact a super admin.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  // ── Detail View ──
  if (selectedProject) {
    return (
      <div className="space-y-6">
        <div className="flex items-center gap-3">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setSelectedProject(null)}
          >
            <ArrowLeft className="h-5 w-5" />
          </Button>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 flex-wrap">
              <h1 className="text-2xl font-semibold tracking-tight truncate">
                {selectedProject.name}
              </h1>
              {getStatusBadge(selectedProject.status)}
            </div>
            {selectedProject.description && (
              <p className="text-muted-foreground mt-1">{selectedProject.description}</p>
            )}
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <Button
              variant="outline"
              size="sm"
              onClick={() => openEditDialog(selectedProject)}
            >
              <Pencil className="mr-2 h-4 w-4" />
              Edit
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="text-destructive hover:text-destructive"
              onClick={() => openDeleteDialog(selectedProject.id)}
            >
              <Trash2 className="mr-2 h-4 w-4" />
              Delete
            </Button>
          </div>
        </div>

        {/* Working Apps */}
        {selectedProject.working_apps && selectedProject.working_apps.length > 0 && (
          <Card>
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <Tag className="h-4 w-4 text-muted-foreground" />
                Working Apps
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-2">
                {selectedProject.working_apps.map((app) => (
                  <Badge key={app} variant="outline" className="text-xs">
                    {app}
                  </Badge>
                ))}
              </div>
            </CardContent>
          </Card>
        )}

        {/* Assigned Employees */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between flex-wrap gap-4">
              <div>
                <CardTitle className="text-base flex items-center gap-2">
                  <Users className="h-4 w-4 text-muted-foreground" />
                  Assigned Employees
                </CardTitle>
                <CardDescription>
                  {loadingAssignments
                    ? "Loading..."
                    : `${assignments.length} employee${assignments.length === 1 ? "" : "s"} assigned`}
                </CardDescription>
              </div>
              <Button
                size="sm"
                onClick={() => {
                  setAssignEmployeeId("");
                  setAssignIsPrimary(false);
                  setAssignDialogOpen(true);
                }}
                disabled={availableEmployees.length === 0}
              >
                <Plus className="mr-2 h-4 w-4" />
                Assign Employee
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            {loadingAssignments ? (
              <div className="space-y-3">
                {Array.from({ length: 3 }).map((_, i) => (
                  <Skeleton key={i} className="h-12 w-full" />
                ))}
              </div>
            ) : assignments.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-12 text-center">
                <Users className="h-10 w-10 text-muted-foreground mb-3" />
                <p className="text-muted-foreground font-medium">No employees assigned</p>
                <p className="text-sm text-muted-foreground mt-1">
                  Assign employees to this project to get started.
                </p>
              </div>
            ) : (
              <div className="space-y-2">
                {assignments.map((assignment) => {
                  const emp = assignment.employee || employeeMap.get(assignment.employee_id);
                  const displayName = emp
                    ? `${emp.first_name} ${emp.last_name}`
                    : assignment.employee_id;
                  return (
                    <div
                      key={assignment.id}
                      className="flex items-center justify-between rounded-lg border p-3"
                    >
                      <div className="flex items-center gap-3 min-w-0">
                        <div className="flex h-8 w-8 items-center justify-center rounded-full bg-muted text-xs font-medium shrink-0">
                          {emp
                            ? `${emp.first_name[0]}${emp.last_name[0]}`
                            : "?"}
                        </div>
                        <div className="min-w-0">
                          <p className="text-sm font-medium truncate">{displayName}</p>
                          {emp?.email && (
                            <p className="text-xs text-muted-foreground truncate">
                              {emp.email}
                            </p>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center gap-2 shrink-0">
                        {assignment.is_primary && (
                          <Badge className="bg-blue-100 text-blue-700 border-blue-200 text-[10px] px-1.5 py-0.5">
                            Primary
                          </Badge>
                        )}
                        <Button
                          variant="ghost"
                          size="sm"
                          className="text-destructive hover:text-destructive"
                          onClick={() => openUnassignDialog(assignment.employee_id, displayName)}
                        >
                          <X className="h-4 w-4" />
                        </Button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Edit Dialog (detail view) */}
        <Dialog open={editDialogOpen} onOpenChange={setEditDialogOpen}>
          <DialogContent className="sm:max-w-[520px]">
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Pencil className="h-5 w-5 text-muted-foreground" />
                Edit Project
              </DialogTitle>
              <DialogDescription>Update the project details below.</DialogDescription>
            </DialogHeader>
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="edit-name">
                  Name <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="edit-name"
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  placeholder="Project name"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="edit-description">Description</Label>
                <textarea
                  id="edit-description"
                  value={editDescription}
                  onChange={(e) => setEditDescription(e.target.value)}
                  placeholder="Brief project description"
                  rows={3}
                  className="flex w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 resize-none"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="edit-status">Status</Label>
                <Select value={editStatus} onValueChange={setEditStatus}>
                  <SelectTrigger id="edit-status">
                    <SelectValue placeholder="Select status" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="active">Active</SelectItem>
                    <SelectItem value="archived">Archived</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label>Working Apps</Label>
                <div className="flex gap-2">
                  <Input
                    value={editAppInput}
                    onChange={(e) => setEditAppInput(e.target.value)}
                    placeholder="e.g. github.com, vscode"
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault();
                        addEditApp();
                      }
                    }}
                  />
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={addEditApp}
                    disabled={!editAppInput.trim()}
                  >
                    Add
                  </Button>
                </div>
                {editWorkingApps.length > 0 && (
                  <div className="flex flex-wrap gap-1.5 mt-2">
                    {editWorkingApps.map((app) => (
                      <Badge key={app} variant="secondary" className="text-xs gap-1 pr-1.5">
                        {app}
                        <button
                          type="button"
                          className="ml-0.5 hover:text-destructive"
                          onClick={() => removeEditApp(app)}
                        >
                          <X className="h-3 w-3" />
                        </button>
                      </Badge>
                    ))}
                  </div>
                )}
              </div>
              <DialogFooter>
                <Button
                  variant="outline"
                  onClick={() => setEditDialogOpen(false)}
                  disabled={saving}
                >
                  Cancel
                </Button>
                <Button onClick={handleEdit} disabled={saving || !editName.trim()}>
                  {saving ? "Saving..." : "Save Changes"}
                </Button>
              </DialogFooter>
            </div>
          </DialogContent>
        </Dialog>

        {/* Delete Dialog (detail view) */}
        <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Delete Project</DialogTitle>
              <DialogDescription>
                Are you sure you want to delete this project? This action cannot be undone.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => {
                  setDeleteDialogOpen(false);
                  setDeletingProjectId(null);
                }}
                disabled={deleting}
              >
                Cancel
              </Button>
              <Button variant="destructive" onClick={handleDelete} disabled={deleting}>
                {deleting ? "Deleting..." : "Delete"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Assign Employee Dialog */}
        <Dialog open={assignDialogOpen} onOpenChange={setAssignDialogOpen}>
          <DialogContent className="sm:max-w-[440px]">
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Users className="h-5 w-5 text-muted-foreground" />
                Assign Employee
              </DialogTitle>
              <DialogDescription>
                Select an employee to assign to this project.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="assign-employee">Employee</Label>
                <Select value={assignEmployeeId} onValueChange={setAssignEmployeeId}>
                  <SelectTrigger id="assign-employee">
                    <SelectValue placeholder="Select an employee..." />
                  </SelectTrigger>
                  <SelectContent>
                    {availableEmployees.length === 0 && (
                      <div className="px-2 py-4 text-sm text-muted-foreground text-center">
                        All employees are already assigned
                      </div>
                    )}
                    {availableEmployees.map((emp) => (
                      <SelectItem key={emp.id} value={emp.id}>
                        {emp.first_name} {emp.last_name}
                        {emp.email && (
                          <span className="ml-2 text-xs text-muted-foreground">
                            {emp.email}
                          </span>
                        )}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="assign-primary"
                  checked={assignIsPrimary}
                  onChange={(e) => setAssignIsPrimary(e.target.checked)}
                  className="h-4 w-4 rounded border-border"
                />
                <Label htmlFor="assign-primary" className="text-sm font-normal">
                  Mark as primary project
                </Label>
              </div>
              <DialogFooter>
                <Button
                  variant="outline"
                  onClick={() => setAssignDialogOpen(false)}
                  disabled={assigning}
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleAssign}
                  disabled={assigning || !assignEmployeeId}
                >
                  {assigning ? "Assigning..." : "Assign"}
                </Button>
              </DialogFooter>
            </div>
          </DialogContent>
        </Dialog>

        {/* Unassign Confirmation Dialog */}
        <Dialog open={unassignDialogOpen} onOpenChange={setUnassignDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Remove Employee</DialogTitle>
              <DialogDescription>
                Are you sure you want to remove {unassigningEmployeeName} from this project?
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => {
                  setUnassignDialogOpen(false);
                  setUnassigningEmployeeId(null);
                }}
                disabled={unassigning}
              >
                Cancel
              </Button>
              <Button
                variant="destructive"
                onClick={handleUnassign}
                disabled={unassigning}
              >
                {unassigning ? "Removing..." : "Remove"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    );
  }

  // ── List View ──
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between flex-wrap gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Projects</h1>
          <p className="text-muted-foreground">Manage company projects and employee assignments.</p>
        </div>
        <Button onClick={() => setCreateDialogOpen(true)}>
          <Plus className="mr-2 h-4 w-4" />
          New Project
        </Button>
      </div>

      {/* Status Filter Tabs */}
      <div className="flex items-center gap-2">
        {(["all", "active", "archived"] as const).map((filter) => {
          const count =
            filter === "all"
              ? projects.length
              : projects.filter((p) => p.status === filter).length;
          return (
            <Button
              key={filter}
              variant={statusFilter === filter ? "default" : "outline"}
              size="sm"
              onClick={() => setStatusFilter(filter)}
            >
              {filter.charAt(0).toUpperCase() + filter.slice(1)}
              <Badge
                variant="secondary"
                className="ml-2 text-[10px] px-1.5 py-0"
              >
                {count}
              </Badge>
            </Button>
          );
        })}
      </div>

      {/* Project Cards Grid */}
      {loading ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <Card key={i}>
              <CardHeader>
                <Skeleton className="h-5 w-2/3" />
                <Skeleton className="h-4 w-full mt-2" />
              </CardHeader>
              <CardContent>
                <div className="flex gap-1.5">
                  <Skeleton className="h-5 w-16" />
                  <Skeleton className="h-5 w-20" />
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : filteredProjects.length === 0 ? (
        <Card>
          <CardContent className="py-16 text-center">
            <Folder className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
            <p className="text-muted-foreground font-medium">
              {statusFilter !== "all"
                ? `No ${statusFilter} projects`
                : "No projects yet"}
            </p>
            <p className="text-sm text-muted-foreground mt-1">
              {statusFilter !== "all"
                ? "Try a different filter or create a new project."
                : "Create your first project to get started."}
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredProjects.map((project) => (
            <Card
              key={project.id}
              className="cursor-pointer hover:shadow-md transition-shadow"
              onClick={() => setSelectedProject(project)}
            >
              <CardHeader>
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <CardTitle className="text-base truncate">
                      {project.name}
                    </CardTitle>
                    {project.description && (
                      <CardDescription className="mt-1 line-clamp-2">
                        {project.description}
                      </CardDescription>
                    )}
                  </div>
                  {getStatusBadge(project.status)}
                </div>
              </CardHeader>
              <CardContent>
                {project.working_apps && project.working_apps.length > 0 ? (
                  <div className="flex flex-wrap gap-1.5">
                    {project.working_apps.slice(0, 4).map((app) => (
                      <Badge
                        key={app}
                        variant="outline"
                        className="text-[10px] px-1.5 py-0.5"
                      >
                        <Tag className="mr-1 h-2.5 w-2.5" />
                        {app}
                      </Badge>
                    ))}
                    {project.working_apps.length > 4 && (
                      <Badge variant="secondary" className="text-[10px] px-1.5 py-0.5">
                        +{project.working_apps.length - 4} more
                      </Badge>
                    )}
                  </div>
                ) : (
                  <p className="text-xs text-muted-foreground">No working apps configured</p>
                )}
                <div className="flex items-center justify-end gap-1 mt-3 pt-3 border-t">
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7"
                    onClick={(e) => {
                      e.stopPropagation();
                      openEditDialog(project);
                    }}
                    title="Edit project"
                  >
                    <Pencil className="h-3.5 w-3.5 text-muted-foreground" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7"
                    onClick={(e) => {
                      e.stopPropagation();
                      openDeleteDialog(project.id);
                    }}
                    title="Delete project"
                  >
                    <Trash2 className="h-3.5 w-3.5 text-destructive" />
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Create Project Dialog */}
      <Dialog
        open={createDialogOpen}
        onOpenChange={(open) => {
          setCreateDialogOpen(open);
          if (!open) resetCreateForm();
        }}
      >
        <DialogContent className="sm:max-w-[520px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Folder className="h-5 w-5 text-muted-foreground" />
              Create Project
            </DialogTitle>
            <DialogDescription>
              Add a new project to organize employees and track working apps.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="project-name">
                Name <span className="text-destructive">*</span>
              </Label>
              <Input
                id="project-name"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="e.g. Frontend Revamp"
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="project-description">Description</Label>
              <textarea
                id="project-description"
                value={newDescription}
                onChange={(e) => setNewDescription(e.target.value)}
                placeholder="Brief description of the project"
                rows={3}
                className="flex w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 resize-none"
              />
            </div>
            <div className="space-y-2">
              <Label>Working Apps</Label>
              <div className="flex gap-2">
                <Input
                  value={newAppInput}
                  onChange={(e) => setNewAppInput(e.target.value)}
                  placeholder="e.g. github.com, vscode"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      addNewApp();
                    }
                  }}
                />
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={addNewApp}
                  disabled={!newAppInput.trim()}
                >
                  Add
                </Button>
              </div>
              {newWorkingApps.length > 0 && (
                <div className="flex flex-wrap gap-1.5 mt-2">
                  {newWorkingApps.map((app) => (
                    <Badge key={app} variant="secondary" className="text-xs gap-1 pr-1.5">
                      {app}
                      <button
                        type="button"
                        className="ml-0.5 hover:text-destructive"
                        onClick={() => removeNewApp(app)}
                      >
                        <X className="h-3 w-3" />
                      </button>
                    </Badge>
                  ))}
                </div>
              )}
            </div>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => {
                  setCreateDialogOpen(false);
                  resetCreateForm();
                }}
                disabled={creating}
              >
                Cancel
              </Button>
              <Button onClick={handleCreate} disabled={creating || !newName.trim()}>
                {creating ? "Creating..." : "Create Project"}
              </Button>
            </DialogFooter>
          </div>
        </DialogContent>
      </Dialog>

      {/* Edit Dialog (list view) */}
      <Dialog open={editDialogOpen} onOpenChange={setEditDialogOpen}>
        <DialogContent className="sm:max-w-[520px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Pencil className="h-5 w-5 text-muted-foreground" />
              Edit Project
            </DialogTitle>
            <DialogDescription>Update the project details below.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="edit-name-lv">
                Name <span className="text-destructive">*</span>
              </Label>
              <Input
                id="edit-name-lv"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
                placeholder="Project name"
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-description-lv">Description</Label>
              <textarea
                id="edit-description-lv"
                value={editDescription}
                onChange={(e) => setEditDescription(e.target.value)}
                placeholder="Brief project description"
                rows={3}
                className="flex w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 resize-none"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-status-lv">Status</Label>
              <Select value={editStatus} onValueChange={setEditStatus}>
                <SelectTrigger id="edit-status-lv">
                  <SelectValue placeholder="Select status" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="active">Active</SelectItem>
                  <SelectItem value="archived">Archived</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label>Working Apps</Label>
              <div className="flex gap-2">
                <Input
                  value={editAppInput}
                  onChange={(e) => setEditAppInput(e.target.value)}
                  placeholder="e.g. github.com, vscode"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      addEditApp();
                    }
                  }}
                />
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={addEditApp}
                  disabled={!editAppInput.trim()}
                >
                  Add
                </Button>
              </div>
              {editWorkingApps.length > 0 && (
                <div className="flex flex-wrap gap-1.5 mt-2">
                  {editWorkingApps.map((app) => (
                    <Badge key={app} variant="secondary" className="text-xs gap-1 pr-1.5">
                      {app}
                      <button
                        type="button"
                        className="ml-0.5 hover:text-destructive"
                        onClick={() => removeEditApp(app)}
                      >
                        <X className="h-3 w-3" />
                      </button>
                    </Badge>
                  ))}
                </div>
              )}
            </div>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => setEditDialogOpen(false)}
                disabled={saving}
              >
                Cancel
              </Button>
              <Button onClick={handleEdit} disabled={saving || !editName.trim()}>
                {saving ? "Saving..." : "Save Changes"}
              </Button>
            </DialogFooter>
          </div>
        </DialogContent>
      </Dialog>

      {/* Delete Dialog (list view) */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Project</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this project? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setDeleteDialogOpen(false);
                setDeletingProjectId(null);
              }}
              disabled={deleting}
            >
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleDelete} disabled={deleting}>
              {deleting ? "Deleting..." : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}