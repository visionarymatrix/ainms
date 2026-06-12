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
  Key,
  BarChart3,
  Package,
  ShieldCheck,
  BarChart2,
  Settings,
  Crosshair,
  Folder,
  Brain,
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

interface NavItem {
  title: string;
  href: string;
  icon: React.ComponentType<{ className?: string }>;
  roles: string[];
}

const navGroups: { label: string; items: NavItem[] }[] = [
  {
    label: "Overview",
    items: [
      { title: "Dashboard", href: "/dashboard", icon: LayoutDashboard, roles: ["super_admin", "company_admin"] },
      { title: "Companies", href: "/companies", icon: Building2, roles: ["super_admin"] },
    ],
  },
  {
    label: "People",
    items: [
      { title: "Employees", href: "/employees", icon: Users, roles: ["super_admin", "company_admin"] },
      { title: "Roles", href: "/roles", icon: Shield, roles: ["super_admin", "company_admin"] },
      { title: "Projects", href: "/projects", icon: Folder, roles: ["super_admin", "company_admin"] },
    ],
  },
  {
    label: "Monitoring",
    items: [
      { title: "Devices", href: "/devices", icon: Monitor, roles: ["super_admin", "company_admin"] },
      { title: "Activity", href: "/activity", icon: Activity, roles: ["super_admin", "company_admin"] },
      { title: "Activity Summaries", href: "/activity-summaries", icon: BarChart3, roles: ["super_admin", "company_admin"] },
      { title: "AI Activity", href: "/ai-activity", icon: Brain, roles: ["super_admin", "company_admin"] },
      { title: "Screenshots", href: "/screenshots", icon: Camera, roles: ["super_admin"] },
      { title: "Targeted Screenshots", href: "/targeted-screenshots", icon: Crosshair, roles: ["super_admin"] },
      { title: "Alerts", href: "/alerts", icon: AlertTriangle, roles: ["super_admin", "company_admin"] },
    ],
  },
  {
    label: "Policy & Compliance",
    items: [
      { title: "App Validations", href: "/app-validations", icon: ShieldCheck, roles: ["super_admin", "company_admin"] },
      { title: "Installed Apps", href: "/installed-apps", icon: Package, roles: ["super_admin", "company_admin"] },
      { title: "App Analytics", href: "/analytics", icon: BarChart2, roles: ["super_admin", "company_admin"] },
      { title: "App Rules", href: "/app-rules", icon: Shield, roles: ["super_admin"] },
      { title: "Alert Rules", href: "/alert-rules", icon: Bell, roles: ["super_admin"] },
    ],
  },
  {
    label: "System",
    items: [
      { title: "Install Tokens", href: "/install-tokens", icon: Key, roles: ["super_admin", "company_admin"] },
    ],
  },
];

export function AdminSidebar() {
  const pathname = usePathname();
  const isSuper = isSuperAdmin();

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
        {navGroups.map((group) => {
          const filteredItems = group.items.filter((item) =>
            isSuper ? true : item.roles.includes("company_admin")
          );
          if (filteredItems.length === 0) return null;
          return (
            <SidebarGroup key={group.label}>
              <SidebarGroupLabel>{group.label}</SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu>
                  {filteredItems.map((item) => (
                    <SidebarMenuItem key={item.href}>
                      <SidebarMenuButton asChild isActive={pathname === item.href || pathname.startsWith(item.href + "/")}>
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
          );
        })}
      </SidebarContent>
    </Sidebar>
  );
}