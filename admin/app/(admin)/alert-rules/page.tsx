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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { getCompanyId } from "@/lib/auth/session";
import {
  listRoles,
  listAlertRules,
  createAlertRule,
  deleteAlertRule,
  type Role,
  type AlertRule,
} from "@/lib/api/roles";

const popupTypeBadge: Record<string, "default" | "secondary" | "destructive"> = {
  toast: "default",
  modal: "secondary",
  soft_block: "destructive",
};

export default function AlertRulesPage() {
  const [roles, setRoles] = useState<Role[]>([]);
  const [selectedRoleId, setSelectedRoleId] = useState<string | null>(null);
  const [alertRules, setAlertRules] = useState<AlertRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [selectedRuleId, setSelectedRuleId] = useState<string | null>(null);
  const [formCategory, setFormCategory] = useState("");
  const [formThresholdMin, setFormThresholdMin] = useState("");
  const [formPopupType, setFormPopupType] = useState("toast");
  const [creating, setCreating] = useState(false);
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
      setRoles([]);
    }
  }

  useEffect(() => {
    if (selectedRoleId) {
      fetchAlertRules(selectedRoleId);
    } else {
      setAlertRules([]);
      setLoading(false);
    }
  }, [selectedRoleId]);

  async function fetchAlertRules(roleId: string) {
    setLoading(true);
    try {
      const data = await listAlertRules(roleId);
      setAlertRules(data);
    } catch {
      setAlertRules([]);
    } finally {
      setLoading(false);
    }
  }

  async function handleCreateAlertRule(e: React.FormEvent) {
    e.preventDefault();
    if (!selectedRoleId) return;
    setCreating(true);
    try {
      const thresholdNum = Number(formThresholdMin);
      if (isNaN(thresholdNum) || thresholdNum < 1) {
        toast.error("Threshold must be a positive number");
        return;
      }
      await createAlertRule(selectedRoleId, {
        category: formCategory,
        threshold_min: thresholdNum,
        popup_type: formPopupType,
      });
      setDialogOpen(false);
      setFormCategory("");
      setFormThresholdMin("");
      setFormPopupType("toast");
      await fetchAlertRules(selectedRoleId);
      toast.success("Alert rule created");
    } catch {
      toast.error("Failed to create alert rule");
    } finally {
      setCreating(false);
    }
  }

  async function handleDeleteAlertRule() {
    if (!selectedRoleId || !selectedRuleId) return;
    try {
      await deleteAlertRule(selectedRoleId, selectedRuleId);
      await fetchAlertRules(selectedRoleId);
      setDeleteDialogOpen(false);
      setSelectedRuleId(null);
      toast.success("Alert rule deleted");
    } catch {
      toast.error("Failed to delete alert rule");
    }
  }

  if (!companyId) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Alert Rules</h1>
          <p className="text-muted-foreground">
            Configure alert thresholds and notification rules.
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
          <h1 className="text-2xl font-semibold tracking-tight">Alert Rules</h1>
          <p className="text-muted-foreground">
            Configure alert thresholds and notification rules.
          </p>
        </div>
      </div>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Alert Rules</CardTitle>
              <CardDescription>
                {loading
                  ? "Loading..."
                  : selectedRoleId
                    ? `${alertRules.length} alert rules`
                    : "Select a role to view alert rules"}
              </CardDescription>
            </div>
            <div className="flex items-center gap-3">
              <Select
                value={selectedRoleId ?? ""}
                onValueChange={(value) => setSelectedRoleId(value || null)}
              >
                <SelectTrigger className="w-[200px]">
                  <SelectValue placeholder="Select a role" />
                </SelectTrigger>
                <SelectContent>
                  {roles.map((role) => (
                    <SelectItem key={role.id} value={role.id}>
                      {role.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {selectedRoleId && (
                <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
                  <DialogTrigger asChild>
                    <Button>
                      <Plus className="mr-2 h-4 w-4" />
                      Add Rule
                    </Button>
                  </DialogTrigger>
                  <DialogContent>
                    <DialogHeader>
                      <DialogTitle>New Alert Rule</DialogTitle>
                      <DialogDescription>
                        Create an alert rule for this role.
                      </DialogDescription>
                    </DialogHeader>
                    <form onSubmit={handleCreateAlertRule} className="flex flex-col gap-4">
                      <div className="flex flex-col gap-2">
                        <Label htmlFor="category">Category</Label>
                        <Input
                          id="category"
                          value={formCategory}
                          onChange={(e) => setFormCategory(e.target.value)}
                          placeholder="e.g. social_media"
                          required
                        />
                      </div>
                      <div className="flex flex-col gap-2">
                        <Label htmlFor="threshold_min">Threshold (minutes)</Label>
                        <Input
                          id="threshold_min"
                          type="number"
                          min={1}
                          value={formThresholdMin}
                          onChange={(e) => setFormThresholdMin(e.target.value)}
                          placeholder="30"
                          required
                        />
                      </div>
                      <div className="flex flex-col gap-2">
                        <Label htmlFor="popup_type">Popup Type</Label>
                        <Select
                          value={formPopupType}
                          onValueChange={(value) => setFormPopupType(value)}
                        >
                          <SelectTrigger>
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="toast">Toast</SelectItem>
                            <SelectItem value="modal">Modal</SelectItem>
                            <SelectItem value="soft_block">Soft Block</SelectItem>
                          </SelectContent>
                        </Select>
                      </div>
                      <Button type="submit" disabled={creating}>
                        {creating ? "Creating..." : "Create Rule"}
                      </Button>
                    </form>
                  </DialogContent>
                </Dialog>
              )}
            </div>
          </div>
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
              Select a role to view and manage alert rules.
            </p>
          ) : alertRules.length === 0 ? (
            <p className="text-muted-foreground text-sm py-8 text-center">
              No alert rules for this role yet. Add your first rule to get started.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Category</TableHead>
                  <TableHead>Threshold (minutes)</TableHead>
                  <TableHead>Popup Type</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {alertRules.map((rule) => (
                  <TableRow key={rule.id}>
                    <TableCell className="font-medium">{rule.category}</TableCell>
                    <TableCell>{rule.threshold_min}</TableCell>
                    <TableCell>
                      <Badge variant={popupTypeBadge[rule.popup_type] || "outline"}>
                        {rule.popup_type}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          setSelectedRuleId(rule.id);
                          setDeleteDialogOpen(true);
                        }}
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
            <DialogTitle>Delete Alert Rule</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this alert rule? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialogOpen(false)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleDeleteAlertRule}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
