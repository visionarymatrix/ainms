"use client";

import { useEffect, useState, useCallback } from "react";
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
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { api } from "@/lib/api/client";
import { getUser, getCompanyId } from "@/lib/auth/session";
import {
  listInstallTokens,
  generateInstallToken,
  revokeInstallToken,
  type InstallToken,
} from "@/lib/api/install-tokens";
import { listEmployees, type Employee } from "@/lib/api/employees";
import {
  Key,
  Plus,
  Copy,
  Eye,
  EyeOff,
  AlertCircle,
  CheckCircle,
  XCircle,
  Clock,
  Terminal,
  RefreshCw,
  Search,
  Terminal as TerminalIcon,
} from "lucide-react";
import { toast } from "sonner";

interface EmployeeMap {
  [id: string]: Employee;
}

type TokenStatus = "active" | "used" | "expired" | "revoked";
type FilterStatus = "all" | TokenStatus;

function timeAgo(dateStr: string | null): string {
  if (!dateStr) return "Never";
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return "just now";
  if (diffMin < 60) return `${diffMin} min ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  if (diffDay < 30) return `${diffDay}d ago`;
  const diffMonth = Math.floor(diffDay / 30);
  if (diffMonth < 12) return `${diffMonth}mo ago`;
  const diffYear = Math.floor(diffMonth / 12);
  return `${diffYear}y ago`;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "Never";
  return new Date(dateStr).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function getTokenStatus(token: InstallToken): TokenStatus {
  if (token.revoked_at) return "revoked";
  if (token.used_at) return "used";
  if (token.expires_at && new Date(token.expires_at) < new Date()) return "expired";
  return "active";
}

function getStatusBadge(status: TokenStatus): { label: string; className: string } {
  switch (status) {
    case "active":
      return {
        label: "Active",
        className: "bg-emerald-100 text-emerald-800 hover:bg-emerald-100 border-emerald-200",
      };
    case "used":
      return {
        label: "Used",
        className: "bg-blue-100 text-blue-800 hover:bg-blue-100 border-blue-200",
      };
    case "expired":
      return {
        label: "Expired",
        className: "bg-gray-100 text-gray-800 hover:bg-gray-100 border-gray-200",
      };
    case "revoked":
      return {
        label: "Revoked",
        className: "bg-red-100 text-red-800 hover:bg-red-100 border-red-200",
      };
    default:
      return { label: status, className: "" };
  }
}

function maskToken(token: string): string {
  if (token.length <= 8) return token;
  return `${token.slice(0, 4)}...${token.slice(-4)}`;
}

function copyToClipboard(text: string, label: string) {
  navigator.clipboard.writeText(text);
  toast.success(`${label} copied to clipboard`);
}

export default function InstallTokensPage() {
  const [tokens, setTokens] = useState<InstallToken[]>([]);
  const [employees, setEmployees] = useState<EmployeeMap>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterStatus, setFilterStatus] = useState<FilterStatus>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [revealedTokens, setRevealedTokens] = useState<Set<string>>(new Set());

  const [generateDialogOpen, setGenerateDialogOpen] = useState(false);
  const [selectedEmployeeId, setSelectedEmployeeId] = useState<string>("");
  const [description, setDescription] = useState("");
  const [expiresIn, setExpiresIn] = useState<string>("7d");
  const [generating, setGenerating] = useState(false);
  const [employeeSelectOpen, setEmployeeSelectOpen] = useState(false);

  const [successDialogOpen, setSuccessDialogOpen] = useState(false);
  const [generatedToken, setGeneratedToken] = useState<InstallToken | null>(null);
  const [showFullToken, setShowFullToken] = useState(false);

  const [revokeDialogOpen, setRevokeDialogOpen] = useState(false);
  const [tokenToRevoke, setTokenToRevoke] = useState<string | null>(null);
  const [revoking, setRevoking] = useState(false);

  const companyId = getCompanyId();
  const user = getUser();

  const fetchTokens = useCallback(async () => {
    try {
      const data = await listInstallTokens(companyId || undefined);
      setTokens(data || []);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load tokens");
      setTokens([]);
    } finally {
      setLoading(false);
    }
  }, [companyId]);

  const fetchEmployees = useCallback(async () => {
    if (!companyId) return;
    try {
      const data = await listEmployees(companyId);
      const empMap: EmployeeMap = {};
      data.forEach((emp) => {
        empMap[emp.id] = emp;
      });
      setEmployees(empMap);
    } catch (err) {
      console.error("Failed to fetch employees:", err);
    }
  }, [companyId]);

  useEffect(() => {
    fetchTokens();
    fetchEmployees();
  }, [fetchTokens, fetchEmployees]);

  useEffect(() => {
    const interval = setInterval(fetchTokens, 30000);
    return () => clearInterval(interval);
  }, [fetchTokens]);

  async function handleGenerateToken(e: React.FormEvent) {
    e.preventDefault();
    if (!companyId || !selectedEmployeeId) return;

    setGenerating(true);
    try {
      const req = {
        employee_id: selectedEmployeeId,
        company_id: companyId,
        description: description || undefined,
        expires_in: expiresIn === "never" ? undefined : expiresIn,
      };
      const token = await generateInstallToken(req);
      setGeneratedToken(token);
      setGenerateDialogOpen(false);
      setSuccessDialogOpen(true);
      setShowFullToken(false);
      await fetchTokens();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to generate token");
    } finally {
      setGenerating(false);
    }
  }

  async function handleRevoke() {
    if (!tokenToRevoke) return;
    setRevoking(true);
    try {
      await revokeInstallToken(tokenToRevoke);
      toast.success("Token revoked successfully");
      await fetchTokens();
      setRevokeDialogOpen(false);
      setTokenToRevoke(null);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to revoke token");
    } finally {
      setRevoking(false);
    }
  }

  function openRevokeDialog(tokenId: string) {
    setTokenToRevoke(tokenId);
    setRevokeDialogOpen(true);
  }

  function toggleTokenReveal(tokenId: string) {
    setRevealedTokens((prev) => {
      const next = new Set(prev);
      if (next.has(tokenId)) {
        next.delete(tokenId);
      } else {
        next.add(tokenId);
      }
      return next;
    });
  }

  const filteredTokens = tokens.filter((token) => {
    const status = getTokenStatus(token);
    if (filterStatus !== "all" && status !== filterStatus) return false;
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      const employee = employees[token.employee_id];
      const empName = employee ? `${employee.first_name} ${employee.last_name}`.toLowerCase() : "";
      return (
        token.token.toLowerCase().includes(query) ||
        token.description.toLowerCase().includes(query) ||
        empName.includes(query)
      );
    }
    return true;
  });

  const activeCount = tokens.filter((t) => getTokenStatus(t) === "active").length;
  const usedCount = tokens.filter((t) => getTokenStatus(t) === "used").length;
  const expiredCount = tokens.filter((t) => getTokenStatus(t) === "expired").length;
  const revokedCount = tokens.filter((t) => getTokenStatus(t) === "revoked").length;

  function resetGenerateForm() {
    setSelectedEmployeeId("");
    setDescription("");
    setExpiresIn("7d");
  }

  function parseInstallCommands(installCmd: string): { linux: string; windows: string } {
    const defaults = {
      linux: `curl -fsSL http://173.249.47.143:8440/v1/install.sh | sudo bash -s -- --token ${installCmd.split("--token ")[1] || ""}`,
      windows: `powershell -c "iwr http://173.249.47.143:8440/v1/install.ps1 | iex" -- -token ${installCmd.split("--token ")[1]?.split(" ")[0] || ""}`,
    };

    const tokenMatch = installCmd.match(/--token\s+(\S+)/);
    const token = tokenMatch ? tokenMatch[1] : "";

    if (token) {
      return {
        linux: `curl -fsSL http://173.249.47.143:8440/v1/install.sh | sudo bash -s -- --token ${token}`,
        windows: `powershell -c "iwr http://173.249.47.143:8440/v1/install.ps1 | iex" -- -token ${token}`,
      };
    }

    return defaults;
  }

  const commands = generatedToken ? parseInstallCommands(generatedToken.install_cmd) : { linux: "", windows: "" };

  return (
    <TooltipProvider delayDuration={200}>
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">Install Tokens</h1>
            <p className="text-muted-foreground">
              Generate one-command install tokens for device enrollment.
            </p>
          </div>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={fetchTokens}
              disabled={loading}
            >
              <RefreshCw className={`mr-2 h-4 w-4 ${loading ? "animate-spin" : ""}`} />
              {loading ? "Refreshing..." : "Refresh"}
            </Button>
            <Dialog
              open={generateDialogOpen}
              onOpenChange={(open) => {
                setGenerateDialogOpen(open);
                if (open) resetGenerateForm();
              }}
            >
              <DialogTrigger asChild>
                <Button>
                  <Plus className="mr-2 h-4 w-4" />
                  Generate Token
                </Button>
              </DialogTrigger>
              <DialogContent className="sm:max-w-[500px]">
                <DialogHeader>
                  <DialogTitle>Generate Install Token</DialogTitle>
                  <DialogDescription>
                    Create a new install token for an employee. The token can be used to enroll a device with a single command.
                  </DialogDescription>
                </DialogHeader>
                <form onSubmit={handleGenerateToken} className="flex flex-col gap-4">
                  <div className="flex flex-col gap-2">
                    <Label htmlFor="employee">Employee</Label>
                    <Select
                      value={selectedEmployeeId}
                      onValueChange={setSelectedEmployeeId}
                      open={employeeSelectOpen}
                      onOpenChange={setEmployeeSelectOpen}
                    >
                      <SelectTrigger id="employee">
                        <SelectValue placeholder="Select an employee" />
                      </SelectTrigger>
                      <SelectContent>
                        {Object.values(employees).map((emp) => (
                          <SelectItem key={emp.id} value={emp.id}>
                            {emp.first_name} {emp.last_name} ({emp.employee_id})
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="flex flex-col gap-2">
                    <Label htmlFor="description">Description (optional)</Label>
                    <Input
                      id="description"
                      value={description}
                      onChange={(e) => setDescription(e.target.value)}
                      placeholder="e.g., John's laptop"
                    />
                  </div>
                  <div className="flex flex-col gap-2">
                    <Label htmlFor="expires">Expires In</Label>
                    <Select value={expiresIn} onValueChange={setExpiresIn}>
                      <SelectTrigger id="expires">
                        <SelectValue placeholder="Select expiry" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="24h">24 hours</SelectItem>
                        <SelectItem value="7d">7 days</SelectItem>
                        <SelectItem value="30d">30 days</SelectItem>
                        <SelectItem value="never">Never</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <DialogFooter className="mt-2">
                    <Button
                      type="button"
                      variant="outline"
                      onClick={() => setGenerateDialogOpen(false)}
                    >
                      Cancel
                    </Button>
                    <Button
                      type="submit"
                      disabled={!selectedEmployeeId || generating}
                    >
                      {generating ? "Generating..." : "Generate Token"}
                    </Button>
                  </DialogFooter>
                </form>
              </DialogContent>
            </Dialog>
          </div>
        </div>

        <Card className="border-blue-200 bg-blue-50/50">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-blue-900">
              <TerminalIcon className="h-5 w-5 text-blue-600" />
              How to Install
            </CardTitle>
            <CardDescription className="text-blue-700">
              The new one-command install flow makes device enrollment effortless.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-4 md:grid-cols-3">
              <div className="flex items-start gap-3">
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-blue-100 text-blue-700 font-semibold text-sm">
                  1
                </div>
                <div>
                  <p className="font-medium text-blue-900">Generate Token</p>
                  <p className="text-sm text-blue-700">
                    Admin generates a token for an employee
                  </p>
                </div>
              </div>
              <div className="flex items-start gap-3">
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-blue-100 text-blue-700 font-semibold text-sm">
                  2
                </div>
                <div>
                  <p className="font-medium text-blue-900">Share Command</p>
                  <p className="text-sm text-blue-700">
                    Employee receives the install command
                  </p>
                </div>
              </div>
              <div className="flex items-start gap-3">
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-blue-100 text-blue-700 font-semibold text-sm">
                  3
                </div>
                <div>
                  <p className="font-medium text-blue-900">Run & Done</p>
                  <p className="text-sm text-blue-700">
                    Employee runs one command, agent installs automatically
                  </p>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <div className="grid gap-4 md:grid-cols-4">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                Total Tokens
              </CardTitle>
            </CardHeader>
            <CardContent>
              {loading ? (
                <Skeleton className="h-9 w-16" />
              ) : (
                <div className="text-3xl font-bold">{tokens.length}</div>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                Active
              </CardTitle>
            </CardHeader>
            <CardContent>
              {loading ? (
                <Skeleton className="h-9 w-16" />
              ) : (
                <div className="text-3xl font-bold text-emerald-600">{activeCount}</div>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                Used
              </CardTitle>
            </CardHeader>
            <CardContent>
              {loading ? (
                <Skeleton className="h-9 w-16" />
              ) : (
                <div className="text-3xl font-bold text-blue-600">{usedCount}</div>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                Revoked
              </CardTitle>
            </CardHeader>
            <CardContent>
              {loading ? (
                <Skeleton className="h-9 w-16" />
              ) : (
                <div className="text-3xl font-bold text-red-600">{revokedCount}</div>
              )}
            </CardContent>
          </Card>
        </div>

        {error && (
          <Card className="border-red-200 bg-red-50">
            <CardContent className="pt-4 flex items-center gap-2">
              <AlertCircle className="h-4 w-4 text-red-800" />
              <p className="text-sm text-red-800">{error}</p>
            </CardContent>
          </Card>
        )}

        <Card>
          <CardHeader>
            <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
              <div>
                <CardTitle>Token List</CardTitle>
                <CardDescription>
                  {loading ? "Loading..." : `${filteredTokens.length} tokens`}
                </CardDescription>
              </div>
              <div className="flex gap-2">
                <div className="relative">
                  <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
                  <Input
                    placeholder="Search tokens..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    className="pl-9 w-[200px]"
                  />
                </div>
                <Select value={filterStatus} onValueChange={(v) => setFilterStatus(v as FilterStatus)}>
                  <SelectTrigger className="w-[130px]">
                    <SelectValue placeholder="Filter by status" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All</SelectItem>
                    <SelectItem value="active">Active</SelectItem>
                    <SelectItem value="used">Used</SelectItem>
                    <SelectItem value="expired">Expired</SelectItem>
                    <SelectItem value="revoked">Revoked</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="space-y-3">
                {[1, 2, 3].map((i) => (
                  <Skeleton key={i} className="h-12 w-full" />
                ))}
              </div>
            ) : filteredTokens.length === 0 ? (
              <div className="text-center py-12">
                <Key className="mx-auto h-12 w-12 text-muted-foreground/50" />
                <p className="mt-4 text-lg font-medium text-muted-foreground">
                  No tokens found
                </p>
                <p className="text-sm text-muted-foreground">
                  {searchQuery || filterStatus !== "all"
                    ? "Try adjusting your filters"
                    : "Generate your first token to get started"}
                </p>
              </div>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Token</TableHead>
                    <TableHead>Employee</TableHead>
                    <TableHead>Description</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Created</TableHead>
                    <TableHead>Expires</TableHead>
                    <TableHead className="text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filteredTokens.map((token) => {
                    const status = getTokenStatus(token);
                    const statusBadge = getStatusBadge(status);
                    const employee = employees[token.employee_id];
                    const isRevealed = revealedTokens.has(token.id);

                    return (
                      <TableRow key={token.id}>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <code className="text-xs bg-muted px-1.5 py-0.5 rounded font-mono">
                              {isRevealed ? token.token : maskToken(token.token)}
                            </code>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-6 w-6"
                              onClick={() => toggleTokenReveal(token.id)}
                            >
                              {isRevealed ? (
                                <EyeOff className="h-3 w-3" />
                              ) : (
                                <Eye className="h-3 w-3" />
                              )}
                            </Button>
                          </div>
                        </TableCell>
                        <TableCell className="font-medium">
                          {employee
                            ? `${employee.first_name} ${employee.last_name}`
                            : token.employee_id.slice(0, 8)}...
                        </TableCell>
                        <TableCell>{token.description || "—"}</TableCell>
                        <TableCell>
                          <Badge variant="outline" className={statusBadge.className}>
                            {statusBadge.label}
                          </Badge>
                        </TableCell>
                        <TableCell>{timeAgo(token.created_at)}</TableCell>
                        <TableCell>
                          {token.expires_at ? formatDate(token.expires_at) : "Never"}
                        </TableCell>
                        <TableCell className="text-right">
                          <div className="flex justify-end gap-1">
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  className="h-8 w-8"
                                  onClick={() => copyToClipboard(token.install_cmd, "Install command")}
                                >
                                  <Copy className="h-4 w-4" />
                                </Button>
                              </TooltipTrigger>
                              <TooltipContent>Copy install command</TooltipContent>
                            </Tooltip>
                            {status === "active" && (
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button
                                    variant="ghost"
                                    size="icon"
                                    className="h-8 w-8 text-destructive hover:text-destructive"
                                    onClick={() => openRevokeDialog(token.id)}
                                  >
                                    <XCircle className="h-4 w-4" />
                                  </Button>
                                </TooltipTrigger>
                                <TooltipContent>Revoke token</TooltipContent>
                              </Tooltip>
                            )}
                          </div>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>

        <Dialog open={successDialogOpen} onOpenChange={setSuccessDialogOpen}>
          <DialogContent className="sm:max-w-[600px]">
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <CheckCircle className="h-5 w-5 text-emerald-600" />
                Token Generated
              </DialogTitle>
              <DialogDescription>
                Share this install command with the employee. The token will not be shown again.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4">
              <Card className="border-amber-200 bg-amber-50">
                <CardContent className="pt-4">
                  <div className="flex items-start gap-2">
                    <AlertCircle className="h-4 w-4 text-amber-600 mt-0.5 shrink-0" />
                    <p className="text-sm text-amber-800">
                      <strong>Important:</strong> Copy this command now. For security, the full token will not be displayed again.
                    </p>
                  </div>
                </CardContent>
              </Card>

              <div className="space-y-2">
                <Label>Token</Label>
                <div className="flex items-center gap-2">
                  <code className="flex-1 bg-muted px-3 py-2 rounded text-sm font-mono">
                    {showFullToken ? generatedToken?.token : maskToken(generatedToken?.token || "")}
                  </code>
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => setShowFullToken(!showFullToken)}
                  >
                    {showFullToken ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                  </Button>
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => copyToClipboard(generatedToken?.token || "", "Token")}
                  >
                    <Copy className="h-4 w-4" />
                  </Button>
                </div>
              </div>

              <div className="space-y-2">
                <Label>Install Command</Label>
                <Tabs defaultValue="linux" className="w-full">
                  <TabsList className="grid w-full grid-cols-2">
                    <TabsTrigger value="linux">Linux / macOS</TabsTrigger>
                    <TabsTrigger value="windows">Windows</TabsTrigger>
                  </TabsList>
                  <TabsContent value="linux" className="mt-2">
                    <div className="relative">
                      <pre className="bg-slate-950 text-slate-50 p-4 rounded-md overflow-x-auto text-sm font-mono">
                        <code>{commands.linux}</code>
                      </pre>
                      <Button
                        variant="secondary"
                        size="sm"
                        className="absolute top-2 right-2"
                        onClick={() => copyToClipboard(commands.linux, "Linux install command")}
                      >
                        <Copy className="mr-2 h-3 w-3" />
                        Copy
                      </Button>
                    </div>
                  </TabsContent>
                  <TabsContent value="windows" className="mt-2">
                    <div className="relative">
                      <pre className="bg-slate-950 text-slate-50 p-4 rounded-md overflow-x-auto text-sm font-mono">
                        <code>{commands.windows}</code>
                      </pre>
                      <Button
                        variant="secondary"
                        size="sm"
                        className="absolute top-2 right-2"
                        onClick={() => copyToClipboard(commands.windows, "Windows install command")}
                      >
                        <Copy className="mr-2 h-3 w-3" />
                        Copy
                      </Button>
                    </div>
                  </TabsContent>
                </Tabs>
              </div>
            </div>
            <DialogFooter>
              <Button onClick={() => setSuccessDialogOpen(false)}>Done</Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <Dialog open={revokeDialogOpen} onOpenChange={setRevokeDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Revoke Token</DialogTitle>
              <DialogDescription>
                Are you sure you want to revoke this token? This action cannot be undone and the token will no longer be usable for device enrollment.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setRevokeDialogOpen(false)}>
                Cancel
              </Button>
              <Button
                variant="destructive"
                onClick={handleRevoke}
                disabled={revoking}
              >
                {revoking ? "Revoking..." : "Revoke Token"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </TooltipProvider>
  );
}
