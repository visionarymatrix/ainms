"use client";

import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import {
  getDeviceStatusBadge,
  getTokenStatusBadge,
  getClassificationBadge,
  getConnectionStatusBadge,
  getScreenshotStatusLabel,
  getScreenshotStatusBadgeVariant,
  getEmployeeStatusBadgeVariant,
} from "@/lib/utils/badges";

// ── Device/Fleet status badge ──────────────────────────────────────────────
// Matches: devices/page.tsx <Badge variant="outline" className={statusBadge.className}>
interface DeviceStatusBadgeProps {
  status: string;
  className?: string;
}

export function DeviceStatusBadge({ status, className }: DeviceStatusBadgeProps) {
  const { label, className: badgeClassName } = getDeviceStatusBadge(status);
  return (
    <Badge variant="outline" className={cn(badgeClassName, className)}>
      {label}
    </Badge>
  );
}

// ── Token status badge ─────────────────────────────────────────────────────
interface TokenStatusBadgeProps {
  status: string;
  className?: string;
}

export function TokenStatusBadge({ status, className }: TokenStatusBadgeProps) {
  const { label, className: badgeClassName } = getTokenStatusBadge(status);
  return (
    <Badge variant="outline" className={cn(badgeClassName, className)}>
      {label}
    </Badge>
  );
}

// ── Classification badge (productive/unproductive/neutral) ─────────────────
// Matches: devices/page.tsx <Badge className={badge.className}>{badge.label}</Badge>
// Note: no variant="outline" — matches original usage
interface ClassificationBadgeProps {
  classification: string;
  className?: string;
}

export function ClassificationBadge({ classification, className }: ClassificationBadgeProps) {
  const { label, className: badgeClassName } = getClassificationBadge(classification);
  return (
    <Badge className={cn(badgeClassName, className)}>
      {label}
    </Badge>
  );
}

// ── Connection status badge (online/idle/offline) ──────────────────────────
// Matches: devices/page.tsx <Badge variant={connectionStatus.variant}>{connectionStatus.label}</Badge>
interface ConnectionStatusBadgeProps {
  status: "online" | "idle" | "offline" | string;
  className?: string;
}

export function ConnectionStatusBadge({ status, className }: ConnectionStatusBadgeProps) {
  const { label, variant } = getConnectionStatusBadge(status);
  return (
    <Badge variant={variant} className={className}>
      {label}
    </Badge>
  );
}

// ── Employee status badge ──────────────────────────────────────────────────
// Matches: employees/page.tsx <Badge variant={statusBadge[emp.status] || "outline"}>
interface EmployeeStatusBadgeProps {
  status: string;
  className?: string;
}

export function EmployeeStatusBadge({ status, className }: EmployeeStatusBadgeProps) {
  const variant = getEmployeeStatusBadgeVariant(status);
  return (
    <Badge variant={variant} className={className}>
      {status}
    </Badge>
  );
}

// ── Screenshot status badge ────────────────────────────────────────────────
// Matches: screenshots/page.tsx <Badge variant={getStatusBadgeVariant(ss.status)} className="...">{getStatusLabel(ss.status)}</Badge>
interface ScreenshotStatusBadgeProps {
  status: string;
  className?: string;
}

export function ScreenshotStatusBadge({ status, className }: ScreenshotStatusBadgeProps) {
  const variant = getScreenshotStatusBadgeVariant(status);
  const label = getScreenshotStatusLabel(status);
  return (
    <Badge variant={variant} className={cn("text-[10px] px-1.5 py-0.5 shrink-0", className)}>
      {label}
    </Badge>
  );
}