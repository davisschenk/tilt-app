import { useMemo } from "react";
import { formatDistanceToNow } from "date-fns";
import { TrendingUp, TrendingDown, Minus } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useReadings } from "@/hooks/use-readings";

interface FermentationStatsProps {
  brewId: string;
  og?: number | null;
}

export default function FermentationStats({ brewId, og }: FermentationStatsProps) {
  const { data: readings, isLoading } = useReadings({ brewId });

  const stats = useMemo(() => {
    if (!readings || readings.length === 0) return null;

    const sorted = readings
      .slice()
      .sort((a, b) => new Date(b.recordedAt).getTime() - new Date(a.recordedAt).getTime());

    const latest = sorted[0];
    const currentGravity = latest.gravity;

    let attenuation: number | null = null;
    let estimatedAbv: number | null = null;
    if (og && og > 1.0) {
      attenuation = ((og - currentGravity) / (og - 1.0)) * 100;
      estimatedAbv = (og - currentGravity) * 131.25;
    }

    let tempTrend: "up" | "down" | "steady" = "steady";
    if (sorted.length >= 3) {
      const recent3 = sorted.slice(0, 3).map((r) => r.temperatureF);
      const diff = recent3[0] - recent3[2];
      if (diff > 0.5) tempTrend = "up";
      else if (diff < -0.5) tempTrend = "down";
    }

    const timeSince = formatDistanceToNow(new Date(latest.recordedAt), {
      addSuffix: true,
    });

    return { currentGravity, attenuation, estimatedAbv, tempTrend, timeSince, latestTemp: latest.temperatureF };
  }, [readings, og]);

  if (isLoading) {
    return (
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5 mb-6">
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-20" />
        ))}
      </div>
    );
  }

  if (!stats) return null;

  const TrendIcon =
    stats.tempTrend === "up"
      ? TrendingUp
      : stats.tempTrend === "down"
        ? TrendingDown
        : Minus;

  return (
    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5 mb-6">
      <Card>
        <CardContent className="pt-4 pb-4">
          <p className="text-xs text-muted-foreground">Current Gravity</p>
          <p className="text-xl font-bold">{stats.currentGravity.toFixed(3)}</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="pt-4 pb-4">
          <p className="text-xs text-muted-foreground">Apparent Attenuation</p>
          <p className="text-xl font-bold">
            {stats.attenuation != null ? `${stats.attenuation.toFixed(1)}%` : "—"}
          </p>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="pt-4 pb-4">
          <p className="text-xs text-muted-foreground">Estimated ABV</p>
          <p className="text-xl font-bold">
            {stats.estimatedAbv != null ? `${stats.estimatedAbv.toFixed(1)}%` : "—"}
          </p>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="pt-4 pb-4">
          <p className="text-xs text-muted-foreground">Temperature Trend</p>
          <div className="flex items-center gap-2">
            <TrendIcon className="h-5 w-5" />
            <span className="text-xl font-bold">{stats.latestTemp.toFixed(1)}°F</span>
          </div>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="pt-4 pb-4">
          <p className="text-xs text-muted-foreground">Last Reading</p>
          <p className="text-lg font-bold">{stats.timeSince}</p>
        </CardContent>
      </Card>
    </div>
  );
}
