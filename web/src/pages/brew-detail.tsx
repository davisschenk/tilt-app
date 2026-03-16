import { useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { format, formatDistanceToNow } from "date-fns";
import { Pencil, CheckCircle, Archive, Trash2, PartyPopper, Bell, ChevronDown, ChevronUp, Plus } from "lucide-react";
import Breadcrumbs from "@/components/layout/breadcrumbs";
import PageHeader from "@/components/layout/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { useBrew, useUpdateBrew } from "@/hooks/use-brews";
import { useHydrometer } from "@/hooks/use-hydrometers";
import { useAlertRules } from "@/hooks/use-alert-rules";
import { useAlertTargets } from "@/hooks/use-alert-targets";
import EditBrewDialog from "@/components/brew/edit-brew-dialog";
import DeleteBrewDialog from "@/components/brew/delete-brew-dialog";
import CreateAlertRuleDialog from "@/components/alerts/create-alert-rule-dialog";
import ReadingsChart from "@/components/readings/readings-chart";
import ReadingsTable from "@/components/readings/readings-table";
import FermentationStats from "@/components/readings/fermentation-stats";
import BrewNotes from "@/components/brew/brew-notes";
import * as toast from "@/lib/toast";
import type { AlertMetric, AlertOperator } from "@/types";

const METRIC_LABELS: Record<AlertMetric, string> = {
  gravity: "Gravity",
  temperature_f: "Temperature (°F)",
  gravity_plateau: "Gravity Plateau",
};

const OPERATOR_SYMBOLS: Record<AlertOperator, string> = {
  lte: "≤",
  gte: "≥",
  lt: "<",
  gt: ">",
  eq: "=",
  plateau: "plateau",
};

const STATUS_VARIANT: Record<string, "default" | "secondary" | "outline"> = {
  Active: "default",
  Completed: "secondary",
  Archived: "outline",
};

function StatItem({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-sm text-muted-foreground">{label}</p>
      <p className="text-lg font-semibold">{value}</p>
    </div>
  );
}

export default function BrewDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: brew, isLoading } = useBrew(id!);
  const updateBrew = useUpdateBrew(id!);
  const { data: hydrometer } = useHydrometer(brew?.hydrometerId ?? "");
  const [editOpen, setEditOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [alertsExpanded, setAlertsExpanded] = useState(false);
  const [addAlertOpen, setAddAlertOpen] = useState(false);
  const { data: alertRules } = useAlertRules(id);
  const { data: alertTargets } = useAlertTargets();

  function handleStatusChange(status: "Completed" | "Archived") {
    updateBrew.mutate(
      { status },
      {
        onSuccess: () => toast.success(`Brew marked as ${status}`),
        onError: () => toast.error(`Failed to update brew status`),
      },
    );
  }

  function handleFinishBrew() {
    const fg = brew?.latestReading?.gravity;
    const og = brew?.og;
    const abv = og != null && fg != null ? (og - fg) * 131.25 : null;
    updateBrew.mutate(
      {
        status: "Completed" as const,
        fg: fg ?? null,
        abv: abv != null ? parseFloat(abv.toFixed(1)) : null,
        endDate: new Date().toISOString(),
      },
      {
        onSuccess: () => toast.success(`Brew finished${fg ? ` with FG ${fg.toFixed(3)}` : ""}${abv != null ? ` — ${abv.toFixed(1)}% ABV` : ""}`),
        onError: () => toast.error("Failed to finish brew"),
      },
    );
  }

  if (isLoading) {
    return (
      <div>
        <Breadcrumbs />
        <Skeleton className="h-10 w-64 mb-4" />
        <Skeleton className="h-48 w-full" />
      </div>
    );
  }

  if (!brew) {
    return (
      <div>
        <Breadcrumbs />
        <PageHeader title="Brew Not Found" />
        <p className="text-muted-foreground">This brew does not exist.</p>
        <Button variant="outline" className="mt-4" onClick={() => navigate("/brews")}>
          Back to Brews
        </Button>
      </div>
    );
  }

  return (
    <div>
      <Breadcrumbs />
      <PageHeader
        title={brew.name}
        description={brew.style ?? undefined}
        actions={
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={() => setEditOpen(true)}>
              <Pencil className="mr-2 h-4 w-4" />
              Edit
            </Button>
            {brew.status === "Active" && (
              <Button
                variant="outline"
                size="sm"
                onClick={handleFinishBrew}
                disabled={updateBrew.isPending}
              >
                <CheckCircle className="mr-2 h-4 w-4" />
                Finish Brew
              </Button>
            )}
            {brew.status !== "Archived" && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleStatusChange("Archived")}
                disabled={updateBrew.isPending}
              >
                <Archive className="mr-2 h-4 w-4" />
                Archive
              </Button>
            )}
            <Button variant="destructive" size="sm" onClick={() => setDeleteOpen(true)}>
              <Trash2 className="mr-2 h-4 w-4" />
              Delete
            </Button>
          </div>
        }
      />

      <div className="flex items-center gap-2 mb-6">
        <Badge variant={STATUS_VARIANT[brew.status] ?? "default"}>
          {brew.status}
        </Badge>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Brew Stats</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 gap-4">
              <StatItem label="OG" value={brew.og?.toFixed(3) ?? "—"} />
              <StatItem label="FG" value={brew.fg?.toFixed(3) ?? "—"} />
              <StatItem label="Target FG" value={brew.targetFg?.toFixed(3) ?? "—"} />
              <StatItem label="ABV" value={brew.abv != null ? `${brew.abv.toFixed(1)}%` : "—"} />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <div>
              <p className="text-sm text-muted-foreground">Hydrometer</p>
              <p className="font-medium">
                {hydrometer?.color ?? brew.latestReading?.color ?? "Unknown"}
              </p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Start Date</p>
              <p className="font-medium">
                {brew.startDate
                  ? format(new Date(brew.startDate), "MMM d, yyyy")
                  : "—"}
              </p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">End Date</p>
              <p className="font-medium">
                {brew.endDate
                  ? format(new Date(brew.endDate), "MMM d, yyyy")
                  : "—"}
              </p>
            </div>
          </CardContent>
        </Card>
      </div>

      <BrewNotes brewId={brew.id} notes={brew.notes ?? null} />

      <EditBrewDialog brew={brew} open={editOpen} onOpenChange={setEditOpen} />
      <DeleteBrewDialog brewId={brew.id} brewName={brew.name} open={deleteOpen} onOpenChange={setDeleteOpen} />

      <Separator className="my-8" />

      <div>
        <h2 className="text-lg font-semibold mb-4">Readings</h2>
        <FermentationStats brewId={brew.id} og={brew.og} />
        {brew.status === "Active" &&
          brew.targetFg != null &&
          brew.latestReading &&
          brew.latestReading.gravity <= brew.targetFg && (
            <div className="flex items-center gap-3 rounded-md border border-green-300 bg-green-50 p-4 mb-6 dark:border-green-800 dark:bg-green-950">
              <PartyPopper className="h-5 w-5 text-green-600 shrink-0" />
              <div className="flex-1">
                <p className="font-medium text-green-800 dark:text-green-200">
                  Target gravity reached!
                </p>
                <p className="text-sm text-green-700 dark:text-green-300">
                  Consider completing this brew.
                </p>
              </div>
              <Button
                size="sm"
                onClick={handleFinishBrew}
                disabled={updateBrew.isPending}
              >
                <CheckCircle className="mr-2 h-4 w-4" />
                Finish Brew
              </Button>
            </div>
          )}
        <ReadingsChart brewId={brew.id} targetFg={brew.targetFg} />
        <ReadingsTable brewId={brew.id} />
      </div>

      <Separator className="my-8" />

      <div>
        <div className="flex items-center justify-between mb-4">
          <button
            type="button"
            className="flex items-center gap-2 text-lg font-semibold hover:text-primary transition-colors"
            onClick={() => setAlertsExpanded(!alertsExpanded)}
          >
            <Bell className="h-5 w-5" />
            Recent Alerts
            {alertsExpanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
            {alertRules && alertRules.length > 0 && (
              <Badge variant="secondary" className="ml-1">{alertRules.length}</Badge>
            )}
          </button>
          <Button variant="outline" size="sm" onClick={() => setAddAlertOpen(true)}>
            <Plus className="mr-1 h-3 w-3" />
            Add Alert
          </Button>
        </div>

        {alertsExpanded && (
          <div className="space-y-3">
            {(() => {
              const targetMap = new Map(alertTargets?.map((t) => [t.id, t.name]) ?? []);
              const triggered = alertRules
                ?.filter((r) => r.lastTriggeredAt)
                .sort((a, b) => new Date(b.lastTriggeredAt!).getTime() - new Date(a.lastTriggeredAt!).getTime())
                .slice(0, 5) ?? [];

              if (triggered.length === 0) {
                return (
                  <p className="text-muted-foreground text-sm py-4">No alerts fired yet.</p>
                );
              }

              return triggered.map((r) => {
                const metricLabel = METRIC_LABELS[r.metric] ?? r.metric;
                const opSymbol = OPERATOR_SYMBOLS[r.operator] ?? r.operator;
                const value = r.metric === "gravity" ? r.threshold.toFixed(3) : r.threshold.toFixed(1);
                return (
                  <Card key={r.id}>
                    <CardContent className="py-3 flex items-center justify-between">
                      <div>
                        <p className="font-medium text-sm">{r.name}</p>
                        <p className="text-xs text-muted-foreground">
                          {metricLabel} {opSymbol} {value} → {targetMap.get(r.alertTargetId) ?? "Unknown target"}
                        </p>
                      </div>
                      <p className="text-xs text-muted-foreground">
                        {formatDistanceToNow(new Date(r.lastTriggeredAt!), { addSuffix: true })}
                      </p>
                    </CardContent>
                  </Card>
                );
              });
            })()}
          </div>
        )}
      </div>

      <CreateAlertRuleDialog open={addAlertOpen} onOpenChange={setAddAlertOpen} defaultBrewId={brew.id} />
    </div>
  );
}
