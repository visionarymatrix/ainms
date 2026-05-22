import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export default function ScreenshotsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Screenshots</h1>
        <p className="text-muted-foreground">
          View and manage device screenshots.
        </p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Captured Screenshots</CardTitle>
          <CardDescription>
            Browse screenshots captured from monitored devices.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground text-sm">
            Screenshot gallery will be available here.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}