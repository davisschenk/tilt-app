import { useState } from "react";
import { format } from "date-fns";
import { Plus, Pencil, Trash2, Zap, Globe, ExternalLink } from "lucide-react";
import Breadcrumbs from "@/components/layout/breadcrumbs";
import PageHeader from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useAlertTargets, useTestFireTarget } from "@/hooks/use-alert-targets";
import CreateAlertTargetDialog from "@/components/alerts/create-alert-target-dialog";
import EditAlertTargetDialog from "@/components/alerts/edit-alert-target-dialog";
import DeleteAlertTargetDialog from "@/components/alerts/delete-alert-target-dialog";
import { toast } from "sonner";
import type { AlertTargetResponse, WebhookFormat } from "@/types";

const FORMAT_CONFIG: Record<WebhookFormat, { label: string; className: string }> = {
  discord: { label: "Discord", className: "bg-indigo-100 text-indigo-800 dark:bg-indigo-900 dark:text-indigo-200" },
  slack: { label: "Slack", className: "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200" },
  generic_json: { label: "Generic JSON", className: "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200" },
};

function FormatBadge({ format }: { format: WebhookFormat }) {
  const config = FORMAT_CONFIG[format] ?? FORMAT_CONFIG.generic_json;
  return <Badge variant="secondary" className={config.className}>{config.label}</Badge>;
}

function truncateUrl(url: string, max = 50) {
  if (url.length <= max) return url;
  return url.slice(0, max) + "…";
}

export default function AlertTargets() {
  const { data: targets, isLoading } = useAlertTargets();
  const testFire = useTestFireTarget();
  const [createOpen, setCreateOpen] = useState(false);
  const [editTarget, setEditTarget] = useState<AlertTargetResponse | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<AlertTargetResponse | null>(null);

  function handleTestFire(target: AlertTargetResponse) {
    testFire.mutate(target.id, {
      onSuccess: (result) => {
        if (result.ok) {
          toast.success(`Test sent to "${target.name}" — status ${result.statusCode}`);
        } else {
          toast.error(`Test failed for "${target.name}": ${result.error}`);
        }
      },
      onError: () => {
        toast.error(`Failed to send test to "${target.name}"`);
      },
    });
  }

  return (
    <div>
      <Breadcrumbs />
      <PageHeader
        title="Alert Targets"
        description="Manage your webhook destinations for alerts."
        actions={
          <Button onClick={() => setCreateOpen(true)}>
            <Plus className="mr-2 h-4 w-4" />
            Add Target
          </Button>
        }
      />

      {isLoading ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-48" />
          ))}
        </div>
      ) : targets && targets.length > 0 ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {targets.map((t: AlertTargetResponse) => (
            <Card key={t.id} className={!t.enabled ? "opacity-60" : ""}>
              <CardContent className="pt-5 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Globe className="h-4 w-4 text-muted-foreground" />
                    <p className="font-semibold text-lg">{t.name}</p>
                  </div>
                  <FormatBadge format={t.format} />
                </div>

                <div className="text-sm">
                  <p className="text-muted-foreground">URL</p>
                  <p className="font-mono text-xs break-all" title={t.url}>
                    {truncateUrl(t.url)}
                    <ExternalLink className="inline ml-1 h-3 w-3 text-muted-foreground" />
                  </p>
                </div>

                <div className="grid grid-cols-2 gap-2 text-sm">
                  <div>
                    <p className="text-muted-foreground">Status</p>
                    <p className="font-medium">
                      {t.enabled ? (
                        <span className="text-green-600 dark:text-green-400">Enabled</span>
                      ) : (
                        <span className="text-red-600 dark:text-red-400">Disabled</span>
                      )}
                    </p>
                  </div>
                  <div>
                    <p className="text-muted-foreground">Created</p>
                    <p className="font-medium">{format(new Date(t.createdAt), "MMM d, yyyy")}</p>
                  </div>
                </div>

                {t.secretHeader && (
                  <div className="text-sm">
                    <p className="text-muted-foreground">Auth Header</p>
                    <p className="font-medium text-xs">••••••••</p>
                  </div>
                )}

                <div className="flex gap-2 pt-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => handleTestFire(t)}
                    disabled={testFire.isPending}
                  >
                    <Zap className="mr-1 h-3 w-3" />
                    Test
                  </Button>
                  <Button variant="outline" size="sm" onClick={() => setEditTarget(t)}>
                    <Pencil className="mr-1 h-3 w-3" />
                    Edit
                  </Button>
                  <Button variant="outline" size="sm" onClick={() => setDeleteTarget(t)}>
                    <Trash2 className="mr-1 h-3 w-3" />
                    Delete
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center rounded-md border border-dashed p-12 text-center">
          <p className="text-muted-foreground mb-4">No alert targets configured</p>
          <Button onClick={() => setCreateOpen(true)}>
            <Plus className="mr-2 h-4 w-4" />
            Add Target
          </Button>
        </div>
      )}

      <CreateAlertTargetDialog open={createOpen} onOpenChange={setCreateOpen} />
      {editTarget && (
        <EditAlertTargetDialog
          target={editTarget}
          open={!!editTarget}
          onOpenChange={(open) => { if (!open) setEditTarget(null); }}
        />
      )}
      {deleteTarget && (
        <DeleteAlertTargetDialog
          targetId={deleteTarget.id}
          targetName={deleteTarget.name}
          open={!!deleteTarget}
          onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}
        />
      )}
    </div>
  );
}
