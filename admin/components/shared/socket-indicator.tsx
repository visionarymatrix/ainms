"use client";

import { cn } from "@/lib/utils";

interface SocketIndicatorProps {
  isConnected: boolean;
  className?: string;
}

/**
 * Displays a socket connection status indicator with a pulsing dot
 * and "Socket connected" / "Socket disconnected" label.
 *
 * Used across employees, devices, and screenshots pages.
 */
export function SocketIndicator({ isConnected, className }: SocketIndicatorProps) {
  return (
    <div className={cn("flex items-center gap-1.5 text-xs text-muted-foreground", className)}>
      <span
        className={cn(
          "relative inline-flex h-2 w-2 rounded-full",
          isConnected ? "bg-emerald-500" : "bg-red-500"
        )}
      >
        {isConnected && (
          <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-500 opacity-40" />
        )}
      </span>
      {isConnected ? "Socket connected" : "Socket disconnected"}
    </div>
  );
}