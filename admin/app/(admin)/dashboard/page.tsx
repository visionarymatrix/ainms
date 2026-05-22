"use client";

import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Monitor, Users, Building2, AlertTriangle } from "lucide-react";
import { getUser } from "@/lib/auth/session";
import { listCompanies } from "@/lib/api/companies";
import { listEmployees } from "@/lib/api/employees";
import { api } from "@/lib/api/client";

interface Device {
  id: string;
  status: string;
  last_heartbeat: string | null;
}

interface Stats {
  totalCompanies: number;
  totalEmployees: number;
  totalDevices: number;
  onlineDevices: number;
  alerts: number;
}

export default function DashboardPage() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [loading, setLoading] = useState(true);
  const user = getUser();

  useEffect(() => {
    async function fetchData() {
      try {
        const companies = await listCompanies();
        const companyId = user?.company_id;
        let employeeCount = 0;

        if (companyId) {
          const employees = await listEmployees(companyId);
          employeeCount = employees.length;
        } else if (companies.length > 0) {
          for (const c of companies.slice(0, 10)) {
            try {
              const emps = await listEmployees(c.id);
              employeeCount += emps.length;
            } catch {
              continue;
            }
          }
        }

        let totalDevices = 0;
        let onlineDevices = 0;
        try {
          const devices = await api.get<Device[]>("/v1/devices/status");
          totalDevices = devices?.length ?? 0;
          onlineDevices = (devices || []).filter((d) => {
            if (!d.last_heartbeat) return false;
            return (Date.now() - new Date(d.last_heartbeat).getTime()) / 60000 < 5;
          }).length;
        } catch {
          // devices endpoint may fail for company_admin if no access
        }

        setStats({
          totalCompanies: companies.length,
          totalEmployees: employeeCount,
          totalDevices,
          onlineDevices,
          alerts: 0,
        });
      } catch {
        setStats(null);
      } finally {
        setLoading(false);
      }
    }
    fetchData();
  }, []);

  const cards = [
    {
      title: user?.role === "super_admin" ? "Total Companies" : "Company",
      value: stats?.totalCompanies ?? 0,
      icon: Building2,
      description: user?.role === "super_admin" ? "Registered organizations" : "Your organization",
    },
    {
      title: "Employees",
      value: stats?.totalEmployees ?? 0,
      icon: Users,
      description: "Registered employees",
    },
    {
      title: "Devices",
      value: `${stats?.totalDevices ?? 0}${stats?.onlineDevices ? ` (${stats.onlineDevices} online)` : ""}`,
      icon: Monitor,
      description: "Enrolled devices",
    },
    {
      title: "Alerts",
      value: stats?.alerts ?? 0,
      icon: AlertTriangle,
      description: "Active alerts",
    },
  ];

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Dashboard</h1>
        <p className="text-muted-foreground">
          {user?.role === "super_admin"
            ? "Platform overview and monitoring."
            : "Overview of your workplace monitoring."}
        </p>
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        {cards.map((card) => (
          <Card key={card.title}>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                {card.title}
              </CardTitle>
              <card.icon className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              {loading ? (
                <Skeleton className="h-9 w-16" />
              ) : (
                <div className="text-3xl font-bold">{card.value}</div>
              )}
              <p className="text-xs text-muted-foreground mt-1">
                {card.description}
              </p>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}