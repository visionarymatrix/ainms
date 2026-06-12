"use client";

import { cn } from "@/lib/utils";

interface ConnectionStatusDotProps {
  /** Connection status: "online", "idle", "offline", or any custom string */
  status: "online" | "idle" | "offline" | string;
  /** Size of the dot. Defaults to "sm" (2.5×2.5, matching employees detail dialog). */
  size?: "sm" | "md" | "lg";
  className?: string;
}

/**
 * A status indicator dot with optional ping animation for online/idle states.
 *
 * Size mapping:
 * - sm: h-2.5 w-2.5 (matches employees detail dialog device list)
 * - md: h-3 w-3
 * - lg: h-4 w-4
 *
 * Colors:
 * - online → emerald-500 (with ping animation)
 * - idle  → amber-500 (with ping animation)
 * - offline → gray-400 (no animation)
 * - unknown → gray-300 (no animation)
 */
export function ConnectionStatusDot({
  status,
  size = "sm",
  className,
}: ConnectionStatusDotProps) {
  const sizeClasses: Record<string, string> = {
    sm: "h-2.5 w-2.5",
    md: "h-3 w-3",
    lg: "h-4 w-4",
  };

  const colorMap: Record<string, string> = {
    online: "bg-emerald-500",
    idle: "bg-amber-500",
    offline: "bg-gray-400",
  };

  const pingColorMap: Record<string, string> = {
    online: "bg-emerald-500",
    idle: "bg-amber-500",
  };

  const showPing = status === "online" || status === "idle";

  return (
    <span className={cn("relative inline-flex", sizeClasses[size] || sizeClasses.sm, className)}>
      {showPing && (
        <span
          className={cn(
            "absolute inline-flex h-full w-full animate-ping rounded-full opacity-40",
            pingColorMap[status]
          )}
        />
      )}
      <span
        className={cn(
          "relative inline-flex rounded-full",
          sizeClasses[size] || sizeClasses.sm,
          colorMap[status] || "bg-gray-300"
        )}
      />
    </span>
  );
}