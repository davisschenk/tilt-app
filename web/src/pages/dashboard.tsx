import { Link } from "react-router-dom";
import { Beer, Thermometer, Activity, BarChart3, Plus } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import PageHeader from "@/components/layout/page-header";
import { useBrews } from "@/hooks/use-brews";
import { useHydrometers } from "@/hooks/use-hydrometers";
import { useReadings } from "@/hooks/use-readings";
import type { BrewResponse } from "@/types";

function StatCard({
  title,
  value,
  description,
  icon: Icon,
  isLoading,
}: {
  title: string;
  value: string;
  description?: string;
  icon: React.ComponentType<{ className?: string }>;
  isLoading: boolean;
}) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
        <Icon className="h-4 w-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <Skeleton className="h-8 w-20" />
        ) : (
          <div className="text-2xl font-bold">{value}</div>
        )}
        {description && (
          <p className="text-xs text-muted-foreground mt-1">{description}</p>
        )}
      </CardContent>
    </Card>
  );
}

export default function Dashboard() {
  const { data: activeBrews, isLoading: brewsLoading } = useBrews("Active");
  const { data: hydrometers, isLoading: hydrometersLoading } = useHydrometers();
  const { data: readings, isLoading: readingsLoading } = useReadings({ limit: 1 });

  const today = new Date();
  today.setHours(0, 0, 0, 0);

  const latestReading = readings?.[0];

  return (
    <div>
      <PageHeader
        title="Dashboard"
        description="Overview of your brewing activity."
      />
      <div className="grid gap-4 sm:grid-cols-2">
        <StatCard
          title="Active Brews"
          value={String(activeBrews?.length ?? 0)}
          icon={Beer}
          isLoading={brewsLoading}
        />
        <StatCard
          title="Total Hydrometers"
          value={String(hydrometers?.length ?? 0)}
          icon={Thermometer}
          isLoading={hydrometersLoading}
        />
        <StatCard
          title="Latest Reading"
          value={
            latestReading
              ? `${latestReading.gravity.toFixed(3)} SG / ${latestReading.temperatureF}°F`
              : "—"
          }
          description={latestReading ? `${latestReading.color} hydrometer` : "No readings yet"}
          icon={Activity}
          isLoading={readingsLoading}
        />
        <StatCard
          title="Readings Today"
          value="—"
          description="From all hydrometers"
          icon={BarChart3}
          isLoading={readingsLoading}
        />
      </div>

      <div className="mt-8">
        <h2 className="text-lg font-semibold mb-4">Active Brews</h2>
        {brewsLoading ? (
          <div className="space-y-3">
            <Skeleton className="h-12 w-full" />
            <Skeleton className="h-12 w-full" />
          </div>
        ) : activeBrews && activeBrews.length > 0 ? (
          <div className="space-y-2">
            {activeBrews.map((brew: BrewResponse) => {
              const daysActive = brew.startDate
                ? Math.floor(
                    (Date.now() - new Date(brew.startDate).getTime()) /
                      (1000 * 60 * 60 * 24),
                  )
                : 0;
              const currentGravity = brew.latestReading?.gravity;
              const color = brew.latestReading?.color;

              return (
                <Link
                  key={brew.id}
                  to={`/brews/${brew.id}`}
                  className="flex items-center justify-between rounded-md border p-3 hover:bg-muted/50 transition-colors"
                >
                  <div className="flex items-center gap-3">
                    {color && (
                      <span
                        className="inline-block h-3 w-3 rounded-full"
                        style={{
                          backgroundColor:
                            color === "Red" ? "#E03131" :
                            color === "Green" ? "#2F9E44" :
                            color === "Black" ? "#495057" :
                            color === "Purple" ? "#7048E8" :
                            color === "Orange" ? "#E8590C" :
                            color === "Blue" ? "#1971C2" :
                            color === "Yellow" ? "#F08C00" :
                            color === "Pink" ? "#D6336C" : "#868E96",
                        }}
                      />
                    )}
                    <span className="font-medium">{brew.name}</span>
                  </div>
                  <div className="flex items-center gap-4 text-sm text-muted-foreground">
                    {currentGravity !== undefined && (
                      <span>{currentGravity.toFixed(3)} SG</span>
                    )}
                    <span>{daysActive}d</span>
                  </div>
                </Link>
              );
            })}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center rounded-md border border-dashed p-8 text-center">
            <p className="text-muted-foreground mb-4">No active brews</p>
            <Button asChild>
              <Link to="/brews/new">
                <Plus className="mr-2 h-4 w-4" />
                Start a Brew
              </Link>
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
