"use client";

import { useRouter } from "next/navigation";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { Badge } from "@/components/ui/badge";
import { getUser, clearAuth } from "@/lib/auth/session";
import { listCompanies } from "@/lib/api/companies";
import { useEffect, useState } from "react";

export function Topbar() {
  const router = useRouter();
  const user = getUser();
  const [companyName, setCompanyName] = useState<string>("");

  useEffect(() => {
    if (user?.company_id) {
      listCompanies()
        .then((companies) => {
          const company = companies.find((c) => c.id === user.company_id);
          if (company) setCompanyName(company.name);
        })
        .catch(() => {});
    }
  }, [user?.company_id]);

  const initials = user?.name
    ? user.name
        .split(" ")
        .map((n) => n[0])
        .join("")
    : "?";

  function handleSignOut() {
    clearAuth();
    window.location.href = "/login";
  }

  return (
    <header className="flex h-14 items-center justify-between border-b border-border bg-background px-4">
      <div className="flex items-center gap-2">
        <SidebarTrigger />
      </div>
      <div className="flex items-center gap-4">
        {companyName && (
          <span className="text-sm text-muted-foreground">{companyName}</span>
        )}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button className="flex items-center gap-2 rounded-md p-1.5 hover:bg-accent transition-colors">
              <Avatar className="h-8 w-8">
                <AvatarFallback className="text-xs font-medium">
                  {initials}
                </AvatarFallback>
              </Avatar>
              <span className="text-sm font-medium hidden md:inline">
                {user?.name || "User"}
              </span>
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-48">
            <div className="px-2 py-1.5">
              <p className="text-sm font-medium">{user?.name}</p>
              <p className="text-xs text-muted-foreground">{user?.email}</p>
              <Badge variant="secondary" className="mt-1 text-xs">
                {user?.role === "super_admin" ? "Super Admin" : "Company Admin"}
              </Badge>
            </div>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={handleSignOut}>Sign out</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </header>
  );
}