import { useState, useCallback, useMemo } from "react";
import { Link } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { Beer, Thermometer, Activity, BarChart3, Plus, RefreshCw, TrendingUp, TrendingDown, Minus, WifiOff } from "lucide-react";
import { format, formatDistanceToNow } from "date-fns";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import PageHeader from "@/components/layout/page-header";
import { useBrews } from "@/hooks/use-brews";
import { useHydrometers } from "@/hooks/use-hydrometers";
import { useReadings } from "@/hooks/use-readings";
import { useBrewAnalytics } from "@/hooks/use-brew-analytics";
import type { BrewResponse } from "@/types";
import RecentReadingsChart from "@/components/dashboard/recent-readings-chart";
import ColorDot from "@/components/ui/color-dot";
import { OFFLINE_THRESHOLD_MINUTES } from "@/lib/constants";

function isStale(recordedAt: string): boolean {
  return Date.now() - new Date(recordedAt).getTime() > OFFLINE_THRESHOLD_MINUTES * 60 * 1000;
}

const REFRESH_INTERVAL = 30_000;

function ActiveBrewCard({ brew }: { brew: BrewResponse }) {
  const { data: analytics, isLoading: analyticsLoading } = useBrewAnalytics(brew.id);
  const { data: recentReadings } = useReadings({ brewId: brew.id, limit: 3 });

  const trend = useMemo(() => {
    if (!recentReadings || recentReadings.length < 2) return "flat";
    const sorted = recentReadings
      .slice()
      .sort((a, b) => new Date(a.recordedAt).getTime() - new Date(b.recordedAt).getTime());
    const diff = sorted[sorted.length - 1].gravity - sorted[0].gravity;
    if (diff > 0.001) return "up";
    if (diff < -0.001) return "down";
    return "flat";
  }, [recentReadings]);

  const color = brew.latestReading?.color;
  const lastReadingAt = brew.latestReading ? new Date(brew.latestReading.recordedAt) : null;

  return (
    <Link to={`/brews/${brew.id}`}>
      <Card className="hover:border-primary/50 transition-colors cursor-pointer">
        <CardContent className="pt-5 pb-4">
          <div className="flex items-start justify-between mb-3">
            <div className="flex items-center gap-2 flex-wrap">
              {color && <ColorDot color={color} />}
              <span className="font-semibold">{brew.name}</span>
              {brew.style && (
                <Badge variant="outline" className="text-xs">{brew.style}</Badge>
              )}
              {lastReadingAt && isStale(brew.latestReading!.recordedAt) && (
                <span
                  className="flex items-center gap-1 text-xs text-red-500"
                  title={`Last seen ${formatDistanceToNow(lastReadingAt, { addSuffix: true })} — may be offline`}
                >
                  <span className="relative flex h-2 w-2">
                    <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75" />
                    <span className="relative inline-flex rounded-full h-2 w-2 bg-red-500" />
                  </span>
                  <WifiOff className="h-3 w-3" />
                </span>
              )}
            </div>
            <div className="flex items-center gap-1 text-muted-foreground">
              {trend === "up" && <TrendingUp className="h-4 w-4 text-red-500" />}
              {trend === "down" && <TrendingDown className="h-4 w-4 text-green-500" />}
              {trend === "flat" && <Minus className="h-4 w-4" />}
            </div>
          </div>

          {analyticsLoading ? (
            <div className="grid grid-cols-2 gap-2">
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
            </div>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
              <div>
                <p className="text-xs text-muted-foreground">Gravity</p>
                <p className="text-sm font-semibold">
                  {analytics?.currentGravity?.toFixed(3) ?? brew.latestReading?.gravity.toFixed(3) ?? "—"}
                </p>
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Temp</p>
                <p className="text-sm font-semibold">
                  {analytics?.currentTempF?.toFixed(1) ?? brew.latestReading?.temperatureF.toFixed(1) ?? "—"}°F
                </p>
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Live ABV</p>
                <p className="text-sm font-semibold">
                  {analytics?.liveAbv != null ? `${analytics.liveAbv.toFixed(1)}%` : "—"}
                </p>
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Attenuation</p>
                <p className="text-sm font-semibold">
                  {analytics?.apparentAttenuation != null ? `${analytics.apparentAttenuation.toFixed(0)}%` : "—"}
                </p>
              </div>
            </div>
          )}

          {lastReadingAt && (
            <p className="text-xs text-muted-foreground mt-3">
              Last reading {formatDistanceToNow(lastReadingAt, { addSuffix: true })}
            </p>
          )}
        </CardContent>
      </Card>
    </Link>
  );
}

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
  const queryClient = useQueryClient();
  const [refreshing, setRefreshing] = useState(false);
  const [lastRefreshed, setLastRefreshed] = useState(() => new Date());

  const { data: activeBrews, isLoading: brewsLoading } = useBrews("Active", {
    refetchInterval: REFRESH_INTERVAL,
  });
  const { data: hydrometers, isLoading: hydrometersLoading } = useHydrometers();
  const { data: readings, isLoading: readingsLoading } = useReadings({ limit: 1 });

  const todayStart = useMemo(() => {
    const d = new Date();
    d.setHours(0, 0, 0, 0);
    return d.toISOString();
  }, []);
  const { data: readingsToday } = useReadings({ since: todayStart });

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    await queryClient.invalidateQueries();
    setLastRefreshed(new Date());
    setTimeout(() => setRefreshing(false), 600);
  }, [queryClient]);

  const latestReading = readings?.[0];

  return (
    <div>
      <PageHeader
        title="Dashboard"
        description="Overview of your brewing activity."
        actions={
          <div className="flex items-center gap-3">
            <span className="text-xs text-muted-foreground">
              Updated {format(lastRefreshed, "HH:mm:ss")}
            </span>
            <Button variant="ghost" size="icon" onClick={handleRefresh}>
              <RefreshCw
                className={`h-4 w-4 ${refreshing ? "animate-spin" : ""}`}
              />
              <span className="sr-only">Refresh</span>
            </Button>
          </div>
        }
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
              ? `${latestReading.gravity.toFixed(3)} SG / ${latestReading.temperatureF.toFixed(1)}°F`
              : "—"
          }
          description={latestReading ? `${latestReading.color} hydrometer` : "No readings yet"}
          icon={Activity}
          isLoading={readingsLoading}
        />
        <StatCard
          title="Readings Today"
          value={String(readingsToday?.length ?? 0)}
          description="From all hydrometers"
          icon={BarChart3}
          isLoading={readingsLoading}
        />
      </div>

      <div className="mt-8">
        <h2 className="text-lg font-semibold mb-4">Active Brews</h2>
        {brewsLoading ? (
          <div className="grid gap-4 sm:grid-cols-2">
            <Skeleton className="h-36 w-full" />
            <Skeleton className="h-36 w-full" />
          </div>
        ) : activeBrews && activeBrews.length > 0 ? (
          <div className="grid gap-4 sm:grid-cols-2">
            {activeBrews
              .slice()
              .sort((a, b) => {
                const aTime = a.latestReading ? new Date(a.latestReading.recordedAt).getTime() : 0;
                const bTime = b.latestReading ? new Date(b.latestReading.recordedAt).getTime() : 0;
                return bTime - aTime;
              })
              .map((brew: BrewResponse) => (
                <ActiveBrewCard key={brew.id} brew={brew} />
              ))}
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

      <RecentReadingsChart />
    </div>
  );
}
