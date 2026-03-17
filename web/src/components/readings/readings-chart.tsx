import { useState, useMemo, useRef, useCallback } from "react";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  TimeScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  type ChartOptions,
  type ChartData,
  type Plugin,
} from "chart.js";
import { Line } from "react-chartjs-2";
import annotationPlugin from "chartjs-plugin-annotation";
import "chartjs-adapter-date-fns";
import { format } from "date-fns";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useReadings } from "@/hooks/use-readings";
import { useBrewEvents } from "@/hooks/use-brew-events";
import { useBrewAnalytics } from "@/hooks/use-brew-analytics";
import type { BrewEventType, BrewEventResponse } from "@/types";
import { resolveColor } from "@/lib/chart-theme";

ChartJS.register(
  CategoryScale,
  LinearScale,
  TimeScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  annotationPlugin,
);

type TimeRange = "24h" | "7d" | "30d" | "all";

const RANGE_HOURS: Record<TimeRange, number | null> = {
  "24h": 24,
  "7d": 168,
  "30d": 720,
  all: null,
};

const EVENT_COLORS: Record<BrewEventType, string> = {
  yeast_pitch: "#2f9e44",
  dry_hop: "#e67700",
  fermentation_complete: "#087f5b",
  diacetyl_rest: "#e9c46a",
  cold_crash: "#4dabf7",
  fining_addition: "#f06595",
  transfer: "#4c6ef5",
  packaged: "#7048e8",
  gravity_sample: "#0ca678",
  tasting_note: "#f59f00",
  temperature_change: "#ae3ec9",
  note: "#868e96",
};

const EVENT_LABELS: Record<BrewEventType, string> = {
  yeast_pitch: "Pitch",
  dry_hop: "Dry Hop",
  fermentation_complete: "FG",
  diacetyl_rest: "D-Rest",
  cold_crash: "Cold Crash",
  fining_addition: "Finings",
  transfer: "Transfer",
  packaged: "Packaged",
  gravity_sample: "Sample",
  tasting_note: "Taste",
  temperature_change: "Temp ↑",
  note: "Note",
};

interface ReadingsChartProps {
  brewId: string;
  targetFg?: number | null;
  predictedFgDate?: string | null;
}

interface EventTooltipState {
  event: BrewEventResponse;
  x: number;
  y: number;
}

export default function ReadingsChart({ brewId, targetFg, predictedFgDate }: ReadingsChartProps) {
  const [range, setRange] = useState<TimeRange>("7d");
  const [hoveredEvent, setHoveredEvent] = useState<EventTooltipState | null>(null);
  const chartWrapperRef = useRef<HTMLDivElement>(null);

  const since = useMemo(() => {
    const hours = RANGE_HOURS[range];
    if (!hours) return undefined;
    const d = new Date();
    d.setHours(d.getHours() - hours);
    return d.toISOString();
  }, [range]);

  const { data: readings, isLoading } = useReadings({ brewId, since });
  const { data: events } = useBrewEvents(brewId);
  const { data: analytics } = useBrewAnalytics(brewId);

  const gravityPoints = useMemo(() => {
    if (!readings) return [];
    return readings
      .slice()
      .sort((a, b) => new Date(a.recordedAt).getTime() - new Date(b.recordedAt).getTime())
      .map((r) => ({ x: new Date(r.recordedAt).getTime(), y: r.gravity }));
  }, [readings]);

  const tempPoints = useMemo(() => {
    if (!readings) return [];
    return readings
      .slice()
      .sort((a, b) => new Date(a.recordedAt).getTime() - new Date(b.recordedAt).getTime())
      .map((r) => ({ x: new Date(r.recordedAt).getTime(), y: r.temperatureF }));
  }, [readings]);

  const visibleEvents = useMemo(() => {
    if (!events || gravityPoints.length === 0) return [];
    const minTs = gravityPoints[0].x;
    const maxTs = gravityPoints[gravityPoints.length - 1].x;
    return events.filter((e) => {
      const ts = new Date(e.eventTime).getTime();
      return ts >= minTs && ts <= maxTs;
    });
  }, [events, gravityPoints]);

  const visibleGaps = useMemo(() => {
    if (!analytics?.gaps || gravityPoints.length === 0) return [];
    const minTs = gravityPoints[0].x;
    const maxTs = gravityPoints[gravityPoints.length - 1].x;
    return analytics.gaps.filter((g) => {
      const startTs = new Date(g.startAt).getTime();
      const endTs = new Date(g.endAt).getTime();
      return endTs >= minTs && startTs <= maxTs;
    });
  }, [analytics, gravityPoints]);

  const tickUnit = range === "24h" ? "hour" : "day";

  const annotations = useMemo(() => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const result: Record<string, any> = {};

    if (targetFg != null) {
      result["targetFg"] = {
        type: "line",
        yScaleID: "yGravity",
        yMin: targetFg,
        yMax: targetFg,
        borderColor: "#2F9E44",
        borderWidth: 2,
        borderDash: [6, 4],
        label: {
          display: true,
          content: `Target FG: ${targetFg.toFixed(3)}`,
          position: "end",
          color: "#2F9E44",
          backgroundColor: "transparent",
          font: { size: 11 },
        },
      };
    }

    if (predictedFgDate != null) {
      result["predictedFg"] = {
        type: "line",
        xScaleID: "x",
        xMin: new Date(predictedFgDate).getTime(),
        xMax: new Date(predictedFgDate).getTime(),
        borderColor: "#9c36b5",
        borderWidth: 2,
        borderDash: [5, 3],
        label: {
          display: true,
          content: "Predicted FG",
          position: "start",
          color: "#9c36b5",
          backgroundColor: "transparent",
          font: { size: 10 },
        },
      };
    }

    visibleGaps.forEach((g, i) => {
      result[`gap-${i}`] = {
        type: "box",
        xScaleID: "x",
        xMin: new Date(g.startAt).getTime(),
        xMax: new Date(g.endAt).getTime(),
        backgroundColor: "rgba(239,68,68,0.12)",
        borderWidth: 0,
      };
    });

    visibleEvents.forEach((ev) => {
      result[`event-${ev.id}`] = {
        type: "line",
        xScaleID: "x",
        xMin: new Date(ev.eventTime).getTime(),
        xMax: new Date(ev.eventTime).getTime(),
        borderColor: EVENT_COLORS[ev.eventType],
        borderWidth: 1.5,
        borderDash: [4, 3],
      };
    });

    return result;
  }, [targetFg, predictedFgDate, visibleGaps, visibleEvents]);

  const eventDiamondPlugin = useCallback((): Plugin<"line"> => ({
    id: "eventDiamonds",
    afterDraw(chart) {
      if (visibleEvents.length === 0) return;
      const ctx = chart.ctx;
      const xScale = chart.scales["x"];
      const yScale = chart.scales["yGravity"];
      if (!xScale || !yScale) return;
      const topY = yScale.top + 14;

      visibleEvents.forEach((ev) => {
        const px = xScale.getPixelForValue(new Date(ev.eventTime).getTime());
        const color = EVENT_COLORS[ev.eventType];
        const size = 7;
        ctx.save();
        ctx.beginPath();
        ctx.moveTo(px, topY - size);
        ctx.lineTo(px + size, topY);
        ctx.lineTo(px, topY + size);
        ctx.lineTo(px - size, topY);
        ctx.closePath();
        ctx.fillStyle = color;
        ctx.strokeStyle = "white";
        ctx.lineWidth = 1.5;
        ctx.fill();
        ctx.stroke();
        ctx.restore();
      });
    },
  }), [visibleEvents]);

  const handleChartMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (visibleEvents.length === 0) { setHoveredEvent(null); return; }
    const canvas = e.currentTarget;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const chartInstance = (canvas as any)._chartjs?.chart ?? (canvas as any).__chartjs_chart__;

    if (!chartInstance) { setHoveredEvent(null); return; }
    const xScale = chartInstance.scales["x"];
    const yScale = chartInstance.scales["yGravity"];
    if (!xScale || !yScale) { setHoveredEvent(null); return; }
    const topY = yScale.top + 14;
    const size = 7;

    const wrapper = chartWrapperRef.current;
    if (!wrapper) return;
    const wrapperRect = wrapper.getBoundingClientRect();

    for (const ev of visibleEvents) {
      const px = xScale.getPixelForValue(new Date(ev.eventTime).getTime());
      if (Math.abs(mx - px) <= size + 2 && Math.abs(my - topY) <= size + 2) {
        setHoveredEvent({
          event: ev,
          x: e.clientX - wrapperRect.left,
          y: e.clientY - wrapperRect.top,
        });
        return;
      }
    }
    setHoveredEvent(null);
  }, [visibleEvents]);

  const chartData: ChartData<"line"> = useMemo(() => ({
    datasets: [
      {
        label: "Gravity (SG)",
        data: gravityPoints,
        borderColor: "#1971C2",
        backgroundColor: "transparent",
        borderWidth: 2,
        pointRadius: 0,
        pointHitRadius: 6,
        tension: 0.3,
        yAxisID: "yGravity",
        parsing: false,
      },
      {
        label: "Temperature (°F)",
        data: tempPoints,
        borderColor: "#E8590C",
        backgroundColor: "transparent",
        borderWidth: 2,
        pointRadius: 0,
        pointHitRadius: 6,
        tension: 0.3,
        yAxisID: "yTemp",
        parsing: false,
      },
    ],
  }), [gravityPoints, tempPoints]);

  const gravityMin = gravityPoints.length > 0 ? Math.min(...gravityPoints.map((p) => p.y)) : 1.0;
  const gravityMax = gravityPoints.length > 0 ? Math.max(...gravityPoints.map((p) => p.y)) : 1.1;
  const tempMin = tempPoints.length > 0 ? Math.min(...tempPoints.map((p) => p.y)) : 32;
  const tempMax = tempPoints.length > 0 ? Math.max(...tempPoints.map((p) => p.y)) : 100;

  const chartOptions: ChartOptions<"line"> = useMemo(() => ({
    responsive: true,
    maintainAspectRatio: false,
    animation: false,
    interaction: {
      mode: "index",
      intersect: false,
    },
    plugins: {
      legend: {
        position: "bottom",
        labels: {
          boxWidth: 12,
          font: { size: 11 },
          color: resolveColor("--foreground"),
        },
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
          title: (items) => `Time: ${format(new Date(Number(items[0]?.parsed.x)), "MMM d HH:mm")}`,
          label: (item) => {
            if (item.parsed.y == null) return;
            if (item.datasetIndex === 0) return ` Gravity: ${item.parsed.y.toFixed(3)} SG`;
            return ` Temp: ${item.parsed.y.toFixed(1)}°F`;
          },
        },
      },
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      annotation: { annotations } as any,
    },
    scales: {
      x: {
        type: "time",
        time: {
          unit: tickUnit,
          displayFormats: { hour: "HH:mm", day: "MMM d", month: "MMM d" },
          tooltipFormat: "MMM d HH:mm",
        },
        ticks: {
          maxTicksLimit: 8,
          font: { size: 11 },
          color: resolveColor("--muted-foreground"),
          maxRotation: 0,
        },
        grid: { color: "rgba(128,128,128,0.1)" },
      },
      yGravity: {
        position: "left",
        min: Math.floor(gravityMin * 1000 - 1) / 1000,
        max: Math.ceil(gravityMax * 1000 + 1) / 1000,
        ticks: {
          font: { size: 11 },
          color: "#1971C2",
          callback: (v) => Number(v).toFixed(3),
        },
        grid: { color: "rgba(128,128,128,0.1)" },
      },
      yTemp: {
        position: "right",
        min: Math.floor(tempMin - 2),
        max: Math.ceil(tempMax + 2),
        ticks: {
          font: { size: 11 },
          color: "#E8590C",
          callback: (v) => `${Number(v).toFixed(1)}°F`,
        },
        grid: { drawOnChartArea: false },
      },
    },
  }), [annotations, tickUnit, gravityMin, gravityMax, tempMin, tempMax]);

  const diamondPlugin = useMemo(() => eventDiamondPlugin(), [eventDiamondPlugin]);

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0">
        <CardTitle className="text-base">Readings Chart</CardTitle>
        <div className="flex gap-1">
          {(["24h", "7d", "30d", "all"] as TimeRange[]).map((r) => (
            <Button
              key={r}
              variant={range === r ? "default" : "outline"}
              size="sm"
              onClick={() => setRange(r)}
            >
              {r === "all" ? "All" : r}
            </Button>
          ))}
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <Skeleton className="h-72 w-full" />
        ) : gravityPoints.length === 0 ? (
          <div className="flex items-center justify-center h-72 text-muted-foreground">
            No readings for this time range
          </div>
        ) : (
          <div ref={chartWrapperRef} className="relative" style={{ height: 300 }}>
            <Line
              data={chartData}
              options={chartOptions}
              plugins={[diamondPlugin]}
              onMouseMove={handleChartMouseMove}
              onMouseLeave={() => setHoveredEvent(null)}
            />
            {hoveredEvent && (() => {
              const ev = hoveredEvent.event;
              const color = EVENT_COLORS[ev.eventType];
              const label = EVENT_LABELS[ev.eventType];
              const left = Math.min(hoveredEvent.x + 12, (chartWrapperRef.current?.clientWidth ?? 400) - 200);
              return (
                <div
                  className="absolute z-50 pointer-events-none rounded-md border bg-card text-card-foreground shadow-md px-3 py-2 text-xs max-w-[190px]"
                  style={{ left, top: hoveredEvent.y - 10 }}
                >
                  <div className="flex items-center gap-1.5 font-semibold mb-1" style={{ color }}>
                    <span style={{ display: "inline-block", width: 8, height: 8, background: color, transform: "rotate(45deg)" }} />
                    {label}
                  </div>
                  <div className="text-muted-foreground">{format(new Date(ev.eventTime), "MMM d, yyyy HH:mm")}</div>
                  {ev.notes && <div className="mt-1 text-foreground/80 line-clamp-3">{ev.notes}</div>}
                </div>
              );
            })()}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
