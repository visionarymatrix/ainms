import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export default function AlertRulesPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Alert Rules</h1>
        <p className="text-muted-foreground">
          Configure alert thresholds and notification rules.
        </p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Threshold Rules</CardTitle>
          <CardDescription>
            Define conditions that trigger alerts for employees and devices.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground text-sm">
            Alert threshold rules will be configurable here.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}