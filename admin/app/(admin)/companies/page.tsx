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
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Plus } from "lucide-react";
import { getUser, isSuperAdmin } from "@/lib/auth/session";
import { listCompanies, createCompany } from "@/lib/api/companies";
import type { Company } from "@/lib/api/companies";

const planBadge: Record<string, "default" | "secondary" | "outline"> = {
  free: "outline",
  pro: "secondary",
  enterprise: "default",
};

export default function CompaniesPage() {
  const [companies, setCompanies] = useState<Company[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [newName, setNewName] = useState("");
  const [newPlan, setNewPlan] = useState("free");
  const [creating, setCreating] = useState(false);
  const user = getUser();
  const isSuper = isSuperAdmin();

  useEffect(() => {
    fetchCompanies();
  }, []);

  async function fetchCompanies() {
    try {
      const data = await listCompanies();
      setCompanies(data);
    } catch {
      setCompanies([]);
    } finally {
      setLoading(false);
    }
  }

  async function handleCreateCompany(e: React.FormEvent) {
    e.preventDefault();
    setCreating(true);
    try {
      await createCompany({ name: newName, plan: newPlan });
      setDialogOpen(false);
      setNewName("");
      setNewPlan("free");
      await fetchCompanies();
    } catch {
      setDialogOpen(false);
    } finally {
      setCreating(false);
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Companies</h1>
          <p className="text-muted-foreground">
            {isSuper ? "Manage organizations using AINMS." : "Your company details."}
          </p>
        </div>
        {isSuper && (
          <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
            <DialogTrigger asChild>
              <Button>
                <Plus className="mr-2 h-4 w-4" />
                Add Company
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Create Company</DialogTitle>
                <DialogDescription>
                  Register a new organization in the AINMS platform.
                </DialogDescription>
              </DialogHeader>
              <form onSubmit={handleCreateCompany} className="flex flex-col gap-4">
                <div className="flex flex-col gap-2">
                  <Label htmlFor="company-name">Company Name</Label>
                  <Input
                    id="company-name"
                    value={newName}
                    onChange={(e) => setNewName(e.target.value)}
                    placeholder="Acme Corp"
                    required
                  />
                </div>
                <div className="flex flex-col gap-2">
                  <Label htmlFor="plan">Plan</Label>
                  <select
                    id="plan"
                    value={newPlan}
                    onChange={(e) => setNewPlan(e.target.value)}
                    className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  >
                    <option value="free">Free</option>
                    <option value="pro">Pro</option>
                    <option value="enterprise">Enterprise</option>
                  </select>
                </div>
                <Button type="submit" disabled={creating}>
                  {creating ? "Creating..." : "Create Company"}
                </Button>
              </form>
            </DialogContent>
          </Dialog>
        )}
      </div>
      <Card>
        <CardHeader>
          <CardTitle>{isSuper ? "All Companies" : "Company Details"}</CardTitle>
          <CardDescription>
            {loading ? "Loading..." : `${companies.length} companies registered`}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-10 w-full" />
              ))}
            </div>
          ) : companies.length === 0 ? (
            <p className="text-muted-foreground text-sm py-8 text-center">
              No companies yet. {isSuper ? "Create your first company to get started." : ""}
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Plan</TableHead>
                  <TableHead>Created</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {companies.map((company) => (
                  <TableRow key={company.id}>
                    <TableCell className="font-medium">{company.name}</TableCell>
                    <TableCell>
                      <Badge variant={planBadge[company.plan] || "outline"}>
                        {company.plan}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      {new Date(company.created_at).toLocaleDateString()}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}