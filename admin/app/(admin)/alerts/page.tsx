import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export default function AlertsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Alerts</h1>
        <p className="text-muted-foreground">
          View and manage active alerts across the fleet.
        </p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Alerts Feed</CardTitle>
          <CardDescription>
            Real-time alerts from monitored devices and rules.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground text-sm">
            Alerts feed will be available here.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}