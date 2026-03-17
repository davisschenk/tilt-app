import { useMemo } from "react";
import {
  Chart as ChartJS,
  TimeScale,
  LinearScale,
  PointElement,
  LineElement,
  Tooltip,
  Legend,
  type ChartData,
  type ChartOptions,
} from "chart.js";
import { Line } from "react-chartjs-2";
import "chartjs-adapter-date-fns";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useReadings } from "@/hooks/use-readings";
import { resolveColor } from "@/lib/chart-theme";

ChartJS.register(TimeScale, LinearScale, PointElement, LineElement, Tooltip, Legend);

const BREW_COLORS: Record<string, string> = {
  Red: "#E03131",
  Green: "#2F9E44",
  Black: "#495057",
  Purple: "#7048E8",
  Orange: "#E8590C",
  Blue: "#1971C2",
  Yellow: "#F08C00",
  Pink: "#D6336C",
};

export default function RecentReadingsChart() {
  const since = useMemo(() => {
    const d = new Date();
    d.setHours(d.getHours() - 24);
    return d.toISOString();
  }, []);

  const { data: readings, isLoading } = useReadings({ since });

  const { datasets, colorNames } = useMemo(() => {
    if (!readings || readings.length === 0) return { datasets: [], colorNames: [] };

    const byColor = new Map<string, { x: number; y: number }[]>();
    for (const r of readings) {
      const pts = byColor.get(r.color) ?? [];
      pts.push({ x: new Date(r.recordedAt).getTime(), y: r.gravity });
      byColor.set(r.color, pts);
    }

    const colorNames = Array.from(byColor.keys());
    const datasets = colorNames.map((color) => ({
      label: color,
      data: (byColor.get(color) ?? []).sort((a, b) => a.x - b.x),
      borderColor: BREW_COLORS[color] ?? "#868E96",
      backgroundColor: "transparent",
      borderWidth: 2,
      pointRadius: 0,
      pointHitRadius: 6,
      tension: 0.3,
      parsing: false as const,
    }));

    return { datasets, colorNames };
  }, [readings]);

  const chartData: ChartData<"line"> = useMemo(() => ({ datasets }), [datasets]);

  const chartOptions: ChartOptions<"line"> = useMemo(() => ({
    responsive: true,
    maintainAspectRatio: false,
    animation: false,
    interaction: { mode: "index", intersect: false },
    plugins: {
      legend: {
        position: "bottom",
        labels: { boxWidth: 12, font: { size: 11 }, color: resolveColor("--foreground") },
      },
      tooltip: {
        backgroundColor: resolveColor("--card"),
        borderColor: resolveColor("--border"),
        borderWidth: 1,
        titleColor: resolveColor("--muted-foreground"),
        bodyColor: resolveColor("--foreground"),
        titleFont: { size: 11 },
        bodyFont: { size: 11 },
        callbacks: {
          title: (items) => `Time: ${new Date(Number(items[0]?.parsed.x)).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`,
          label: (item) => item.parsed.y != null ? ` ${item.dataset.label}: ${item.parsed.y.toFixed(3)} SG` : "",
        },
      },
    },
    scales: {
      x: {
        type: "time",
        time: { unit: "hour", displayFormats: { hour: "HH:mm" } },
        ticks: { maxTicksLimit: 8, font: { size: 11 }, color: resolveColor("--muted-foreground"), maxRotation: 0 },
        grid: { color: "rgba(128,128,128,0.1)" },
      },
      y: {
        ticks: {
          font: { size: 11 },
          color: resolveColor("--muted-foreground"),
          callback: (v) => Number(v).toFixed(3),
        },
        grid: { color: "rgba(128,128,128,0.1)" },
      },
    },
  }), [colorNames]);

  return (
    <Card className="mt-8">
      <CardHeader>
        <CardTitle className="text-lg">Recent Readings (24h)</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <Skeleton className="h-64 w-full" />
        ) : datasets.length === 0 ? (
          <div className="flex items-center justify-center h-64 text-muted-foreground">
            No readings in the last 24 hours
          </div>
        ) : (
          <div style={{ height: 300 }}>
            <Line data={chartData} options={chartOptions} />
          </div>
        )}
      </CardContent>
    </Card>
  );
}
