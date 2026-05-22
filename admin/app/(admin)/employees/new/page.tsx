"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { getCompanyId } from "@/lib/auth/session";
import { registerEmployee } from "@/lib/api/employees";

export default function NewEmployeePage() {
  const router = useRouter();
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [email, setEmail] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState(false);
  const [createdId, setCreatedId] = useState("");
  const companyId = getCompanyId();

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!companyId) {
      setError("No company assigned to your account.");
      return;
    }
    setError("");
    setLoading(true);
    try {
      const emp = await registerEmployee(companyId, {
        first_name: firstName,
        last_name: lastName,
        email: email || undefined,
      });
      setCreatedId(emp.employee_id);
      setSuccess(true);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : "Failed to register employee";
      setError(msg);
    } finally {
      setLoading(false);
    }
  }

  if (success) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Employee Created</h1>
          <p className="text-muted-foreground">
            The employee has been successfully registered.
          </p>
        </div>
        <Card className="max-w-lg">
          <CardContent className="py-8 text-center space-y-4">
            <p className="text-lg font-medium">Employee ID: <span className="font-mono">{createdId}</span></p>
            <p className="text-sm text-muted-foreground">
              Share this ID with the employee for their records.
            </p>
            <div className="flex gap-2 justify-center pt-4">
              <Button onClick={() => router.push("/employees")}>Back to Employees</Button>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">New Employee</h1>
        <p className="text-muted-foreground">
          Register a new employee in the system.
        </p>
      </div>
      <Card className="max-w-lg">
        <CardHeader>
          <CardTitle>Employee Details</CardTitle>
          <CardDescription>
            Enter the employee information below.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {error && (
            <div className="mb-4 rounded-md border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive">
              {error}
            </div>
          )}
          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
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
              <Label htmlFor="email">Email (optional)</Label>
              <Input
                id="email"
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="jane@acme.com"
              />
            </div>
            <div className="flex gap-2 pt-4">
              <Button type="submit" disabled={loading}>
                {loading ? "Creating..." : "Create Employee"}
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={() => router.push("/employees")}
              >
                Cancel
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}