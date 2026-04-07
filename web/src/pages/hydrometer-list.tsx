import { useState } from "react";
import { Link } from "react-router-dom";
import { format, formatDistanceToNow } from "date-fns";
import { Plus, Pencil, Trash2, PowerOff, Power } from "lucide-react";
import { OFFLINE_THRESHOLD_MINUTES } from "@/lib/constants";
import Breadcrumbs from "@/components/layout/breadcrumbs";
import PageHeader from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import ColorDot from "@/components/ui/color-dot";
import { useHydrometers, useUpdateHydrometer } from "@/hooks/use-hydrometers";
import { useBrews } from "@/hooks/use-brews";
import * as toast from "@/lib/toast";
import { TILT_COLOR_MAP } from "@/lib/tilt-colors";
import RegisterHydrometerDialog from "@/components/hydrometer/register-hydrometer-dialog";
import EditHydrometerDialog from "@/components/hydrometer/edit-hydrometer-dialog";
import DeleteHydrometerDialog from "@/components/hydrometer/delete-hydrometer-dialog";
import { Badge } from "@/components/ui/badge";
import type { HydrometerResponse } from "@/types";

function isStale(recordedAt: string): boolean {
  const ageMs = Date.now() - new Date(recordedAt).getTime();
  return ageMs > OFFLINE_THRESHOLD_MINUTES * 60 * 1000;
}

export default function HydrometerList() {
  const { data: hydrometers, isLoading } = useHydrometers();
  const { data: activeBrews } = useBrews("Active");
  const [registerOpen, setRegisterOpen] = useState(false);
  const [editTarget, setEditTarget] = useState<HydrometerResponse | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<HydrometerResponse | null>(null);
  const [togglingId, setTogglingId] = useState<string | null>(null);
  const updateHydrometer = useUpdateHydrometer(togglingId ?? "");

  function handleToggleDisabled(h: HydrometerResponse) {
    setTogglingId(h.id);
    updateHydrometer.mutate(
      { isDisabled: !h.isDisabled },
      {
        onSuccess: () => toast.success(h.isDisabled ? "Hydrometer enabled" : "Hydrometer disabled"),
        onError: () => toast.error("Failed to update hydrometer"),
        onSettled: () => setTogglingId(null),
      },
    );
  }

  function getActiveBrew(hydrometerId: string) {
    return activeBrews?.find((b) => b.hydrometerId === hydrometerId);
  }

  return (
    <div>
      <Breadcrumbs />
      <PageHeader
        title="Hydrometers"
        description="Manage your Tilt hydrometers."
        actions={
          <Button onClick={() => setRegisterOpen(!registerOpen)}>
            <Plus className="mr-2 h-4 w-4" />
            Register Hydrometer
          </Button>
        }
      />

      {isLoading ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-48" />
          ))}
        </div>
      ) : hydrometers && hydrometers.length > 0 ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {hydrometers.map((h: HydrometerResponse) => {
            const colorInfo = (TILT_COLOR_MAP as Record<string, typeof TILT_COLOR_MAP.Red>)[h.color];
            const activeBrew = getActiveBrew(h.id);
            return (
              <Card key={h.id} className={h.isDisabled ? "opacity-60" : ""} style={{ borderTopColor: colorInfo?.hex ?? "#868E96", borderTopWidth: 3 }}>
                <CardContent className="pt-5 space-y-3">
                  <div className="flex items-center gap-3">
                    <ColorDot color={h.color} size="lg" />
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <p className="font-semibold text-lg">{h.color}</p>
                        {h.isDisabled ? (
                          <Badge variant="secondary" className="text-xs px-1.5 py-0.5">Disabled</Badge>
                        ) : h.latestReading && isStale(h.latestReading.recordedAt) ? (
                          <Badge className="bg-red-500 text-white text-xs px-1.5 py-0.5">Offline</Badge>
                        ) : h.latestReading ? (
                          <Badge className="bg-green-500 text-white text-xs px-1.5 py-0.5">Live</Badge>
                        ) : null}
                      </div>
                      {h.name && (
                        <p className="text-sm text-muted-foreground">{h.name}</p>
                      )}
                    </div>
                  </div>

                  {h.latestReading && (
                    <div className="grid grid-cols-2 gap-2 text-sm">
                      <div>
                        <p className="text-muted-foreground">Gravity</p>
                        <p className="font-medium">{h.latestReading.gravity.toFixed(3)} SG</p>
                      </div>
                      <div>
                        <p className="text-muted-foreground">Temperature</p>
                        <p className="font-medium">{h.latestReading.temperatureF.toFixed(1)}°F</p>
                      </div>
                    </div>
                  )}

                  <div className="grid grid-cols-2 gap-2 text-sm">
                    <div>
                      <p className="text-muted-foreground">Temp Offset</p>
                      <p className="font-medium">{h.tempOffsetF?.toFixed(1) ?? "0.0"}°F</p>
                    </div>
                    <div>
                      <p className="text-muted-foreground">Gravity Offset</p>
                      <p className="font-medium">{h.gravityOffset?.toFixed(3) ?? "0.000"}</p>
                    </div>
                  </div>

                  <div className="text-sm">
                    <p className="text-muted-foreground">Registered</p>
                    <p className="font-medium">
                      {format(new Date(h.createdAt), "MMM d, yyyy")}
                    </p>
                  </div>

                  {activeBrew && (
                    <div className="text-sm">
                      <p className="text-muted-foreground">Active Brew</p>
                      <Link
                        to={`/brews/${activeBrew.id}`}
                        className="font-medium text-primary hover:underline"
                      >
                        {activeBrew.name}
                      </Link>
                    </div>
                  )}

                  {h.latestReading && (
                    <div className="text-sm">
                      <p className="text-muted-foreground">Last Reading</p>
                      <p className="font-medium">
                        {formatDistanceToNow(new Date(h.latestReading.recordedAt), { addSuffix: true })}
                      </p>
                    </div>
                  )}

                  <div className="flex gap-2 pt-2 flex-wrap">
                    <Button variant="outline" size="sm" onClick={() => setEditTarget(h)}>
                      <Pencil className="mr-1 h-3 w-3" />
                      Edit
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleToggleDisabled(h)}
                      disabled={togglingId === h.id}
                    >
                      {h.isDisabled ? (
                        <><Power className="mr-1 h-3 w-3" />Enable</>
                      ) : (
                        <><PowerOff className="mr-1 h-3 w-3" />Disable</>
                      )}
                    </Button>
                    <Button variant="outline" size="sm" onClick={() => setDeleteTarget(h)}>
                      <Trash2 className="mr-1 h-3 w-3" />
                      Delete
                    </Button>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center rounded-md border border-dashed p-12 text-center">
          <p className="text-muted-foreground mb-4">No hydrometers registered</p>
          <Button onClick={() => setRegisterOpen(!registerOpen)}>
            <Plus className="mr-2 h-4 w-4" />
            Register Hydrometer
          </Button>
        </div>
      )}
      <RegisterHydrometerDialog open={registerOpen} onOpenChange={setRegisterOpen} />
      {editTarget && (
        <EditHydrometerDialog
          hydrometer={editTarget}
          open={!!editTarget}
          onOpenChange={(open) => { if (!open) setEditTarget(null); }}
        />
      )}
      {deleteTarget && (
        <DeleteHydrometerDialog
          hydrometerId={deleteTarget.id}
          hydrometerColor={deleteTarget.color}
          hydrometerName={deleteTarget.name}
          hasActiveBrew={!!getActiveBrew(deleteTarget.id)}
          open={!!deleteTarget}
          onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}
        />
      )}
    </div>
  );
}
