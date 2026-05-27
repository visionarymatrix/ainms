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
import { Plus, Pencil, Trash2, Shield } from "lucide-react";
import { toast } from "sonner";
import { getCompanyId } from "@/lib/auth/session";
import {
  listRoles,
  createRole,
  updateRole,
  deleteRole,
  type Role,
} from "@/lib/api/roles";

export default function RolesPage() {
  const [roles, setRoles] = useState<Role[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingRole, setEditingRole] = useState<Role | null>(null);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [deletingRoleId, setDeletingRoleId] = useState<string | null>(null);

  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [workDescription, setWorkDescription] = useState("");
  const [allowedCategoriesInput, setAllowedCategoriesInput] = useState("");
  const [blockedCategoriesInput, setBlockedCategoriesInput] = useState("");
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const companyId = getCompanyId();

  useEffect(() => {
    if (companyId) {
      fetchRoles();
    } else {
      setLoading(false);
    }
  }, [companyId]);

  async function fetchRoles() {
    if (!companyId) return;
    try {
      const data = await listRoles(companyId);
      setRoles(data);
    } catch {
      toast.error("Failed to load roles");
      setRoles([]);
    } finally {
      setLoading(false);
    }
  }

  function resetForm() {
    setName("");
    setDescription("");
    setWorkDescription("");
    setAllowedCategoriesInput("");
    setBlockedCategoriesInput("");
    setEditingRole(null);
  }

  function openCreateDialog() {
    resetForm();
    setDialogOpen(true);
  }

  function openEditDialog(role: Role) {
    setEditingRole(role);
    setName(role.name);
    setDescription(role.description || "");
    setWorkDescription(role.work_description || "");
    setAllowedCategoriesInput(role.allowed_categories?.join(", ") || "");
    setBlockedCategoriesInput(role.blocked_categories?.join(", ") || "");
    setDialogOpen(true);
  }

  async function handleSave(e: React.FormEvent) {
    e.preventDefault();
    if (!companyId) return;
    if (!name.trim()) {
      toast.error("Name is required");
      return;
    }

    setSaving(true);
    try {
      const payload = {
        name: name.trim(),
        description: description.trim() || undefined,
        work_description: workDescription.trim() || undefined,
        allowed_categories: allowedCategoriesInput
          .split(",")
          .map((s) => s.trim())
          .filter((s) => s.length > 0),
        blocked_categories: blockedCategoriesInput
          .split(",")
          .map((s) => s.trim())
          .filter((s) => s.length > 0),
      };

      if (editingRole) {
        await updateRole(editingRole.id, payload);
        toast.success("Role updated successfully");
      } else {
        await createRole(companyId, payload);
        toast.success("Role created successfully");
      }

      setDialogOpen(false);
      resetForm();
      await fetchRoles();
    } catch {
      toast.error(editingRole ? "Failed to update role" : "Failed to create role");
    } finally {
      setSaving(false);
    }
  }

  function openDeleteDialog(roleId: string) {
    setDeletingRoleId(roleId);
    setDeleteDialogOpen(true);
  }

  async function handleDelete() {
    if (!deletingRoleId) return;
    setDeleting(true);
    try {
      await deleteRole(deletingRoleId);
      toast.success("Role deleted successfully");
      setDeleteDialogOpen(false);
      setDeletingRoleId(null);
      await fetchRoles();
    } catch {
      toast.error("Failed to delete role");
    } finally {
      setDeleting(false);
    }
  }

  if (!companyId) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Roles</h1>
          <p className="text-muted-foreground">Manage roles across companies.</p>
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
          <h1 className="text-2xl font-semibold tracking-tight">Roles</h1>
          <p className="text-muted-foreground">Manage roles in your company.</p>
        </div>
        <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogTrigger asChild>
            <Button onClick={openCreateDialog}>
              <Plus className="mr-2 h-4 w-4" />
              New Role
            </Button>
          </DialogTrigger>
          <DialogContent className="sm:max-w-[520px]">
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Shield className="h-5 w-5 text-muted-foreground" />
                {editingRole ? "Edit Role" : "Create Role"}
              </DialogTitle>
              <DialogDescription>
                {editingRole
                  ? "Update the role details below."
                  : "Define a new role with categories and descriptions."}
              </DialogDescription>
            </DialogHeader>
            <form onSubmit={handleSave} className="flex flex-col gap-4">
              <div className="flex flex-col gap-2">
                <Label htmlFor="role-name">
                  Name <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="role-name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g. Software Engineer"
                  required
                />
              </div>
              <div className="flex flex-col gap-2">
                <Label htmlFor="role-description">Description</Label>
                <textarea
                  id="role-description"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="Brief description of this role"
                  rows={3}
                  className="flex w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 resize-none"
                />
              </div>
              <div className="flex flex-col gap-2">
                <Label htmlFor="role-work-description">Work Description</Label>
                <textarea
                  id="role-work-description"
                  value={workDescription}
                  onChange={(e) => setWorkDescription(e.target.value)}
                  placeholder="Detailed work description"
                  rows={3}
                  className="flex w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 resize-none"
                />
              </div>
              <div className="flex flex-col gap-2">
                <Label htmlFor="role-allowed">Allowed Categories</Label>
                <Input
                  id="role-allowed"
                  value={allowedCategoriesInput}
                  onChange={(e) => setAllowedCategoriesInput(e.target.value)}
                  placeholder="Comma-separated, e.g. productivity, communication"
                />
              </div>
              <div className="flex flex-col gap-2">
                <Label htmlFor="role-blocked">Blocked Categories</Label>
                <Input
                  id="role-blocked"
                  value={blockedCategoriesInput}
                  onChange={(e) => setBlockedCategoriesInput(e.target.value)}
                  placeholder="Comma-separated, e.g. entertainment, social"
                />
              </div>
              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => {
                    setDialogOpen(false);
                    resetForm();
                  }}
                  disabled={saving}
                >
                  Cancel
                </Button>
                <Button type="submit" disabled={saving}>
                  {saving
                    ? editingRole
                      ? "Saving..."
                      : "Creating..."
                    : editingRole
                    ? "Save Changes"
                    : "Create Role"}
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Roles</CardTitle>
          <CardDescription>
            {loading ? "Loading..." : `${roles.length} roles defined`}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-10 w-full" />
              ))}
            </div>
          ) : roles.length === 0 ? (
            <p className="text-muted-foreground text-sm py-8 text-center">
              No roles yet. Create your first role to get started.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Description</TableHead>
                  <TableHead>Work Description</TableHead>
                  <TableHead>Allowed Categories</TableHead>
                  <TableHead>Blocked Categories</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {roles.map((role) => (
                  <TableRow key={role.id} className="hover:bg-muted/50">
                    <TableCell className="font-medium">
                      {role.name}
                    </TableCell>
                    <TableCell className="max-w-[200px] truncate">
                      {role.description || "—"}
                    </TableCell>
                    <TableCell className="max-w-[200px] truncate">
                      {role.work_description || "—"}
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-wrap gap-1">
                        {role.allowed_categories && role.allowed_categories.length > 0 ? (
                          role.allowed_categories.map((cat, idx) => (
                            <Badge key={`${role.id}-a-${idx}`} variant="secondary" className="text-xs">
                              {cat}
                            </Badge>
                          ))
                        ) : (
                          <span className="text-muted-foreground text-xs">—</span>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-wrap gap-1">
                        {role.blocked_categories && role.blocked_categories.length > 0 ? (
                          role.blocked_categories.map((cat, idx) => (
                            <Badge key={`${role.id}-b-${idx}`} variant="destructive" className="text-xs">
                              {cat}
                            </Badge>
                          ))
                        ) : (
                          <span className="text-muted-foreground text-xs">—</span>
                        )}
                      </div>
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex items-center justify-end gap-1">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => openEditDialog(role)}
                        >
                          <Pencil className="h-4 w-4 text-muted-foreground" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => openDeleteDialog(role.id)}
                        >
                          <Trash2 className="h-4 w-4 text-destructive" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Role</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this role? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setDeleteDialogOpen(false);
                setDeletingRoleId(null);
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
