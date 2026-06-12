export function getDeviceStatusBadge(status: string): { label: string; className: string } {
  switch (status) {
    case "pending":
      return { label: "Pending", className: "bg-amber-100 text-amber-800 hover:bg-amber-100 border-amber-200" };
    case "active":
      return { label: "Active", className: "bg-emerald-100 text-emerald-800 hover:bg-emerald-100 border-emerald-200" };
    case "rejected":
      return { label: "Rejected", className: "bg-red-100 text-red-800 hover:bg-red-100 border-red-200" };
    default:
      return { label: status, className: "" };
  }
}

export function getTokenStatusBadge(status: string): { label: string; className: string } {
  switch (status) {
    case "active":
      return { label: "Active", className: "bg-emerald-100 text-emerald-800 hover:bg-emerald-100 border-emerald-200" };
    case "used":
      return { label: "Used", className: "bg-blue-100 text-blue-800 hover:bg-blue-100 border-blue-200" };
    case "expired":
      return { label: "Expired", className: "bg-gray-100 text-gray-800 hover:bg-gray-100 border-gray-200" };
    case "revoked":
      return { label: "Revoked", className: "bg-red-100 text-red-800 hover:bg-red-100 border-red-200" };
    default:
      return { label: status, className: "" };
  }
}

export function getClassificationBadge(classification: string): { label: string; className: string } {
  switch (classification) {
    case "productive":
      return { label: "Productive", className: "bg-emerald-100 text-emerald-800 hover:bg-emerald-100 border-emerald-200" };
    case "unproductive":
      return { label: "Unproductive", className: "bg-red-100 text-red-800 hover:bg-red-100 border-red-200" };
    case "neutral":
      return { label: "Neutral", className: "bg-blue-100 text-blue-800 hover:bg-blue-100 border-blue-200" };
    default:
      return { label: classification, className: "" };
  }
}

export function getScreenshotStatusBadgeVariant(status: string): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "completed": return "default";
    case "pending": return "secondary";
    case "failed": return "destructive";
    default: return "outline";
  }
}

export function getScreenshotStatusLabel(status: string): string {
  switch (status) {
    case "completed":
      return "Completed";
    case "pending":
      return "Pending";
    case "failed":
      return "Failed";
    default:
      return status;
  }
}

export function getConnectionStatusBadge(
  status: "online" | "idle" | "offline" | string
): { label: string; variant: "default" | "outline" | "secondary" | "destructive" } {
  switch (status) {
    case "online":
      return { label: "online", variant: "default" };
    case "idle":
      return { label: "idle", variant: "secondary" };
    case "offline":
      return { label: "offline", variant: "destructive" };
    default:
      return { label: "unknown", variant: "secondary" };
  }
}

export function getEmployeeStatusBadgeVariant(
  status: string
): "default" | "destructive" | "outline" {
  switch (status) {
    case "active":
      return "default";
    case "suspended":
      return "destructive";
    default:
      return "outline";
  }
}