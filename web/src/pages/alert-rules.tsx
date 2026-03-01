import { useState } from "react";
import { formatDistanceToNow } from "date-fns";
import { Plus, Pencil, Trash2 } from "lucide-react";
import Breadcrumbs from "@/components/layout/breadcrumbs";
import PageHeader from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useAlertRules } from "@/hooks/use-alert-rules";
import { useAlertTargets } from "@/hooks/use-alert-targets";
import type { AlertRuleResponse, AlertMetric, AlertOperator } from "@/types";

const METRIC_LABELS: Record<AlertMetric, string> = {
  gravity: "Gravity",
  temperature_f: "Temperature (°F)",
};

const OPERATOR_SYMBOLS: Record<AlertOperator, string> = {
  lte: "≤",
  gte: "≥",
  lt: "<",
  gt: ">",
  eq: "=",
};

function formatCondition(metric: AlertMetric, operator: AlertOperator, threshold: number): string {
  const metricLabel = METRIC_LABELS[metric] ?? metric;
  const opSymbol = OPERATOR_SYMBOLS[operator] ?? operator;
  const value = metric === "gravity" ? threshold.toFixed(3) : threshold.toFixed(1);
  return `${metricLabel} ${opSymbol} ${value}`;
}

export default function AlertRules() {
  const { data: rules, isLoading } = useAlertRules();
  const { data: targets } = useAlertTargets();
  const [_createOpen, setCreateOpen] = useState(false);

  const targetMap = new Map(targets?.map((t) => [t.id, t.name]) ?? []);

  return (
    <div>
      <Breadcrumbs />
      <PageHeader
        title="Alert Rules"
        description="Configure conditions that trigger webhook notifications."
        actions={
          <Button onClick={() => setCreateOpen(true)}>
            <Plus className="mr-2 h-4 w-4" />
            Add Rule
          </Button>
        }
      />

      {isLoading ? (
        <div className="space-y-2">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-12 w-full" />
          ))}
        </div>
      ) : rules && rules.length > 0 ? (
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Condition</TableHead>
                <TableHead>Target</TableHead>
                <TableHead>Cooldown</TableHead>
                <TableHead>Last Triggered</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="w-[100px]">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rules.map((r: AlertRuleResponse) => (
                <TableRow key={r.id} className={!r.enabled ? "opacity-50" : ""}>
                  <TableCell className="font-medium">{r.name}</TableCell>
                  <TableCell>
                    <code className="text-xs bg-muted px-1.5 py-0.5 rounded">
                      {formatCondition(r.metric, r.operator, r.threshold)}
                    </code>
                  </TableCell>
                  <TableCell className="text-sm">
                    {targetMap.get(r.alertTargetId) ?? (
                      <span className="text-muted-foreground italic">Unknown</span>
                    )}
                  </TableCell>
                  <TableCell className="text-sm">{r.cooldownMinutes}m</TableCell>
                  <TableCell className="text-sm">
                    {r.lastTriggeredAt ? (
                      formatDistanceToNow(new Date(r.lastTriggeredAt), { addSuffix: true })
                    ) : (
                      <span className="text-muted-foreground">Never</span>
                    )}
                  </TableCell>
                  <TableCell>
                    {r.enabled ? (
                      <Badge variant="secondary" className="bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">
                        Enabled
                      </Badge>
                    ) : (
                      <Badge variant="secondary" className="bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200">
                        Disabled
                      </Badge>
                    )}
                  </TableCell>
                  <TableCell>
                    <div className="flex gap-1">
                      <Button variant="ghost" size="icon" className="h-8 w-8" disabled>
                        <Pencil className="h-3 w-3" />
                      </Button>
                      <Button variant="ghost" size="icon" className="h-8 w-8" disabled>
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center rounded-md border border-dashed p-12 text-center">
          <p className="text-muted-foreground mb-4">No alert rules configured</p>
          <Button onClick={() => setCreateOpen(true)}>
            <Plus className="mr-2 h-4 w-4" />
            Add Rule
          </Button>
        </div>
      )}
    </div>
  );
}
