import { useState } from "react";
import { Link } from "react-router-dom";
import { format } from "date-fns";
import { Plus } from "lucide-react";
import Breadcrumbs from "@/components/layout/breadcrumbs";
import PageHeader from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import ColorDot from "@/components/ui/color-dot";
import { useHydrometers } from "@/hooks/use-hydrometers";
import { useBrews } from "@/hooks/use-brews";
import { TILT_COLOR_MAP } from "@/lib/tilt-colors";
import type { HydrometerResponse } from "@/types";

export default function HydrometerList() {
  const { data: hydrometers, isLoading } = useHydrometers();
  const { data: activeBrews } = useBrews("Active");
  const [registerOpen, setRegisterOpen] = useState(false);

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
              <Card key={h.id} style={{ borderTopColor: colorInfo?.hex ?? "#868E96", borderTopWidth: 3 }}>
                <CardContent className="pt-5 space-y-3">
                  <div className="flex items-center gap-3">
                    <ColorDot color={h.color} size="lg" />
                    <div>
                      <p className="font-semibold text-lg">{h.color}</p>
                      {h.name && (
                        <p className="text-sm text-muted-foreground">{h.name}</p>
                      )}
                    </div>
                  </div>

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
    </div>
  );
}
