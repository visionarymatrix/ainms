import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export default function AppRulesPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">App Rules</h1>
        <p className="text-muted-foreground">
          Configure application classification and productivity rules.
        </p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Classification Rules</CardTitle>
          <CardDescription>
            Define which applications are productive, neutral, or unproductive.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground text-sm">
            App classification rules will be configurable here.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}