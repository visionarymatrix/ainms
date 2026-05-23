"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  LayoutDashboard,
  Building2,
  Users,
  Monitor,
  Shield,
  Bell,
  Camera,
  AlertTriangle,
  Activity,
} from "lucide-react";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { isSuperAdmin } from "@/lib/auth/session";

const allNavItems = [
  { title: "Dashboard", href: "/dashboard", icon: LayoutDashboard, roles: ["super_admin", "company_admin"] },
  { title: "Companies", href: "/companies", icon: Building2, roles: ["super_admin"] },
  { title: "Employees", href: "/employees", icon: Users, roles: ["super_admin", "company_admin"] },
  { title: "Devices", href: "/devices", icon: Monitor, roles: ["super_admin", "company_admin"] },
  { title: "Activity", href: "/activity", icon: Activity, roles: ["super_admin", "company_admin"] },
  { title: "App Rules", href: "/app-rules", icon: Shield, roles: ["super_admin"] },
  { title: "Alert Rules", href: "/alert-rules", icon: Bell, roles: ["super_admin"] },
  { title: "Screenshots", href: "/screenshots", icon: Camera, roles: ["super_admin"] },
  { title: "Alerts", href: "/alerts", icon: AlertTriangle, roles: ["super_admin", "company_admin"] },
];

export function AdminSidebar() {
  const pathname = usePathname();
  const isSuper = isSuperAdmin();

  const navItems = allNavItems.filter((item) =>
    isSuper ? true : item.roles.includes("company_admin")
  );

  return (
    <Sidebar>
      <SidebarHeader className="border-b border-border px-4 py-4">
        <Link href="/dashboard" className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-md bg-primary text-primary-foreground font-bold text-sm">
            A
          </div>
          <span className="font-semibold text-lg tracking-tight">AINMS</span>
        </Link>
      </SidebarHeader>
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>Navigation</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.map((item) => (
                <SidebarMenuItem key={item.href}>
                  <SidebarMenuButton asChild isActive={pathname === item.href}>
                    <Link href={item.href}>
                      <item.icon className="h-4 w-4" />
                      <span>{item.title}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
    </Sidebar>
  );
}