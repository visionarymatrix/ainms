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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { getCompanyId } from "@/lib/auth/session";
import {
  listRoles,
  listAppClassifications,
  createAppClassification,
  deleteAppClassification,
  type Role,
  type AppClassification,
} from "@/lib/api/roles";

const categoryBadgeClass: Record<string, string> = {
  productive: "bg-emerald-100 text-emerald-700 hover:bg-emerald-100",
  neutral: "bg-blue-100 text-blue-700 hover:bg-blue-100",
  unproductive: "bg-amber-100 text-amber-700 hover:bg-amber-100",
  entertainment: "bg-red-100 text-red-700 hover:bg-red-100",
  communication: "bg-purple-100 text-purple-700 hover:bg-purple-100",
};

const categoryOptions = [
  { value: "productive", label: "Productive" },
  { value: "neutral", label: "Neutral" },
  { value: "unproductive", label: "Unproductive" },
  { value: "entertainment", label: "Entertainment" },
  { value: "communication", label: "Communication" },
];

export default function AppRulesPage() {
  const [roles, setRoles] = useState<Role[]>([]);
  const [selectedRoleId, setSelectedRoleId] = useState<string>("");
  const [classifications, setClassifications] = useState<AppClassification[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [selectedClassificationId, setSelectedClassificationId] = useState<string | null>(null);
  const [formAppName, setFormAppName] = useState("");
  const [formCategory, setFormCategory] = useState("");
  const [creating, setCreating] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const companyId = getCompanyId();

  useEffect(() => {
    if (companyId) {
      fetchRoles();
    } else {
      setLoading(false);
    }
  }, [companyId]);

  useEffect(() => {
    if (selectedRoleId) {
      fetchClassifications(selectedRoleId);
    } else {
      setClassifications([]);
      setLoading(false);
    }
  }, [selectedRoleId]);

  async function fetchRoles() {
    if (!companyId) return;
    try {
      const data = await listRoles(companyId);
      setRoles(data);
    } catch {
      setRoles([]);
      toast.error("Failed to load roles");
    }
  }

  async function fetchClassifications(roleId: string) {
    setLoading(true);
    try {
      const data = await listAppClassifications(roleId);
      setClassifications(data);
    } catch {
      setClassifications([]);
      toast.error("Failed to load classifications");
    } finally {
      setLoading(false);
    }
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!selectedRoleId || !formAppName.trim() || !formCategory) return;
    setCreating(true);
    try {
      await createAppClassification(selectedRoleId, {
        app_name: formAppName.trim(),
        category: formCategory,
      });
      setDialogOpen(false);
      setFormAppName("");
      setFormCategory("");
      await fetchClassifications(selectedRoleId);
      toast.success("Classification created");
    } catch {
      toast.error("Failed to create classification");
    } finally {
      setCreating(false);
    }
  }

  async function handleDelete() {
    if (!selectedRoleId || !selectedClassificationId) return;
    setDeleting(true);
    try {
      await deleteAppClassification(selectedRoleId, selectedClassificationId);
      setDeleteDialogOpen(false);
      setSelectedClassificationId(null);
      await fetchClassifications(selectedRoleId);
      toast.success("Classification deleted");
    } catch {
      toast.error("Failed to delete classification");
    } finally {
      setDeleting(false);
    }
  }

  function openDeleteDialog(classificationId: string) {
    setSelectedClassificationId(classificationId);
    setDeleteDialogOpen(true);
  }

  if (!companyId) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">App Rules</h1>
          <p className="text-muted-foreground">
            Configure application classification and productivity rules.
          </p>
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
          <h1 className="text-2xl font-semibold tracking-tight">App Rules</h1>
          <p className="text-muted-foreground">
            Configure application classification and productivity rules.
          </p>
        </div>
        <div className="flex items-center gap-3">
          <Select value={selectedRoleId} onValueChange={setSelectedRoleId}>
            <SelectTrigger className="w-56">
              <SelectValue placeholder="Filter by role" />
            </SelectTrigger>
            <SelectContent>
              {roles.map((role) => (
                <SelectItem key={role.id} value={role.id}>
                  {role.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
            <DialogTrigger asChild>
              <Button disabled={!selectedRoleId}>
                <Plus className="mr-2 h-4 w-4" />
                Add Classification
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Add Classification</DialogTitle>
                <DialogDescription>
                  Map an application to a category for the selected role.
                </DialogDescription>
              </DialogHeader>
              <form onSubmit={handleCreate} className="flex flex-col gap-4">
                <div className="flex flex-col gap-2">
                  <Label htmlFor="app-name">Application Name</Label>
                  <Input
                    id="app-name"
                    value={formAppName}
                    onChange={(e) => setFormAppName(e.target.value)}
                    placeholder="e.g. Slack"
                    required
                  />
                </div>
                <div className="flex flex-col gap-2">
                  <Label htmlFor="category">Category</Label>
                  <Select value={formCategory} onValueChange={setFormCategory}>
                    <SelectTrigger id="category">
                      <SelectValue placeholder="Select category" />
                    </SelectTrigger>
                    <SelectContent>
                      {categoryOptions.map((opt) => (
                        <SelectItem key={opt.value} value={opt.value}>
                          {opt.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <Button type="submit" disabled={creating || !formCategory}>
                  {creating ? "Adding..." : "Add Classification"}
                </Button>
              </form>
            </DialogContent>
          </Dialog>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Classifications</CardTitle>
          <CardDescription>
            {loading
              ? "Loading..."
              : selectedRoleId
              ? `${classifications.length} classifications for selected role`
              : "Select a role to view classifications"}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-10 w-full" />
              ))}
            </div>
          ) : !selectedRoleId ? (
            <p className="text-muted-foreground text-sm py-8 text-center">
              Choose a role from the filter to see classifications.
            </p>
          ) : classifications.length === 0 ? (
            <p className="text-muted-foreground text-sm py-8 text-center">
              No classifications for this role yet.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>App Name</TableHead>
                  <TableHead>Category</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {classifications.map((c) => (
                  <TableRow key={c.id}>
                    <TableCell className="font-medium">{c.app_name}</TableCell>
                    <TableCell>
                      <Badge
                        variant="secondary"
                        className={categoryBadgeClass[c.category] || ""}
                      >
                        {c.category}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => openDeleteDialog(c.id)}
                      >
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
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
            <DialogTitle>Delete Classification</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this classification? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialogOpen(false)}>
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
