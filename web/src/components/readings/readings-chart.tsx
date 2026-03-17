import { useState, useMemo, useRef, useCallback, useEffect } from "react";
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
  type Chart,
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
import { resolveColor, resolveFont } from "@/lib/chart-theme";

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

function applyChartDefaults() {
  const font = resolveFont();
  ChartJS.defaults.font.family = font;
  ChartJS.defaults.font.size = 12;
}

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
  anchorX: number;
  anchorY: number;
}

export default function ReadingsChart({ brewId, targetFg, predictedFgDate }: ReadingsChartProps) {
  const [range, setRange] = useState<TimeRange>("7d");
  const [hoveredEvent, setHoveredEvent] = useState<EventTooltipState | null>(null);
  const chartWrapperRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<Chart<"line"> | null>(null);
  const DIAMOND_SIZE = 8;
  const DIAMOND_TOP_PADDING = 36;

  useEffect(() => { applyChartDefaults(); }, []);

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
    afterDraw(chart: Chart<"line">) {
      if (visibleEvents.length === 0) return;
      const ctx = chart.ctx;
      const xScale = chart.scales["x"];
      if (!xScale) return;
      const centerY = DIAMOND_TOP_PADDING / 2;

      visibleEvents.forEach((ev) => {
        const px = xScale.getPixelForValue(new Date(ev.eventTime).getTime());
        if (px < xScale.left || px > xScale.right) return;
        const color = EVENT_COLORS[ev.eventType];
        const s = DIAMOND_SIZE;
        ctx.save();
        ctx.beginPath();
        ctx.moveTo(px, centerY - s);
        ctx.lineTo(px + s, centerY);
        ctx.lineTo(px, centerY + s);
        ctx.lineTo(px - s, centerY);
        ctx.closePath();
        ctx.fillStyle = color;
        ctx.strokeStyle = resolveColor("--background");
        ctx.lineWidth = 2;
        ctx.fill();
        ctx.stroke();
        ctx.restore();
      });
    },
  }), [visibleEvents, DIAMOND_TOP_PADDING, DIAMOND_SIZE]);

  const handleChartMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (visibleEvents.length === 0) { setHoveredEvent(null); return; }
    const chart = chartRef.current;
    if (!chart) { setHoveredEvent(null); return; }
    const canvas = e.currentTarget;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const xScale = chart.scales["x"];
    if (!xScale) { setHoveredEvent(null); return; }
    const centerY = DIAMOND_TOP_PADDING / 2;
    const hitRadius = DIAMOND_SIZE + 4;

    const wrapper = chartWrapperRef.current;
    if (!wrapper) return;
    const wrapperRect = wrapper.getBoundingClientRect();

    for (const ev of visibleEvents) {
      const px = xScale.getPixelForValue(new Date(ev.eventTime).getTime());
      if (px < xScale.left || px > xScale.right) continue;
      if (Math.abs(mx - px) <= hitRadius && Math.abs(my - centerY) <= hitRadius) {
        setHoveredEvent({
          event: ev,
          anchorX: px + (wrapper.getBoundingClientRect().left - wrapperRect.left),
          anchorY: centerY,
        });
        return;
      }
    }
    setHoveredEvent(null);
  }, [visibleEvents, DIAMOND_TOP_PADDING, DIAMOND_SIZE]);

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
    layout: {
      padding: { top: DIAMOND_TOP_PADDING },
    },
    interaction: {
      mode: "index",
      intersect: false,
    },
    plugins: {
      legend: {
        position: "bottom",
        labels: {
          boxWidth: 12,
          usePointStyle: true,
          pointStyleWidth: 16,
          font: { size: 12, family: resolveFont() },
          color: resolveColor("--foreground"),
          padding: 16,
        },
      },
      tooltip: {
        enabled: !hoveredEvent,
        displayColors: true,
        usePointStyle: true,
        boxWidth: 8,
        boxHeight: 8,
        backgroundColor: resolveColor("--popover"),
        borderColor: resolveColor("--border"),
        borderWidth: 1,
        titleColor: resolveColor("--muted-foreground"),
        bodyColor: resolveColor("--popover-foreground"),
        padding: 10,
        caretSize: 5,
        cornerRadius: 8,
        titleFont: { size: 12, family: resolveFont() },
        bodyFont: { size: 13, family: resolveFont() },
        callbacks: {
          title: (items) => format(new Date(Number(items[0]?.parsed.x)), "MMM d, yyyy · HH:mm"),
          label: (item) => {
            if (item.parsed.y == null) return;
            if (item.datasetIndex === 0) return `Gravity    ${item.parsed.y.toFixed(3)} SG`;
            return `Temp        ${item.parsed.y.toFixed(1)}°F`;
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
          font: { size: 11, family: resolveFont() },
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
          font: { size: 11, family: resolveFont() },
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
          font: { size: 11, family: resolveFont() },
          color: "#E8590C",
          callback: (v) => `${Number(v).toFixed(1)}°F`,
        },
        grid: { drawOnChartArea: false },
      },
    },
  }), [annotations, tickUnit, gravityMin, gravityMax, tempMin, tempMax, hoveredEvent, DIAMOND_TOP_PADDING]);

  const diamondPlugin = useMemo(() => eventDiamondPlugin(), [eventDiamondPlugin]);

  useEffect(() => {
    if (chartRef.current) chartRef.current.update("none");
  }, [hoveredEvent]);

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
              ref={chartRef}
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
              const wrapperW = chartWrapperRef.current?.clientWidth ?? 400;
              const tipW = 210;
              const raw = hoveredEvent.anchorX - tipW / 2;
              const left = Math.max(4, Math.min(raw, wrapperW - tipW - 4));
              const top = hoveredEvent.anchorY + DIAMOND_SIZE + 6;
              return (
                <div
                  className="absolute z-50 pointer-events-none rounded-lg border bg-popover text-popover-foreground shadow-lg"
                  style={{ left, top, width: tipW }}
                >
                  <div
                    className="flex items-center gap-2 px-3 pt-3 pb-2 border-b"
                    style={{ borderColor: `${color}40` }}
                  >
                    <span
                      className="shrink-0"
                      style={{
                        display: "inline-block",
                        width: 10,
                        height: 10,
                        background: color,
                        transform: "rotate(45deg)",
                        borderRadius: 1,
                      }}
                    />
                    <span className="text-sm font-semibold leading-none" style={{ color }}>
                      {label}
                    </span>
                    <span className="ml-auto text-xs text-muted-foreground whitespace-nowrap">
                      {format(new Date(ev.eventTime), "MMM d")}
                    </span>
                  </div>
                  <div className="px-3 py-2 space-y-1">
                    <div className="text-xs text-muted-foreground">
                      {format(new Date(ev.eventTime), "HH:mm")}
                    </div>
                    {ev.notes && (
                      <div className="text-xs text-foreground leading-relaxed line-clamp-4">
                        {ev.notes}
                      </div>
                    )}
                  </div>
                </div>
              );
            })()}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
