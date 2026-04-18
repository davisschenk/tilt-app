import { useState, useMemo, useRef, useEffect, useCallback } from "react";
import ReactECharts from "echarts-for-react";
import type { EChartsOption } from "echarts";
import { format } from "date-fns";
import { CalendarPlus, Milestone } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useReadings } from "@/hooks/use-readings";
import { useBrewEvents } from "@/hooks/use-brew-events";
import { useBrewAnalytics } from "@/hooks/use-brew-analytics";
import type { BrewEventType, BrewEventResponse } from "@/types";
import { useEChartsTheme } from "@/lib/echarts-theme";
import { CreateEventDialog } from "@/components/brew/brew-event-log";

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
  nutrient_addition: "#74b816",
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
  nutrient_addition: "Nutrient",
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
  const [range, setRange] = useState<TimeRange>("all");
  const [eventTooltip, setEventTooltip] = useState<EventTooltipState | null>(null);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; isoTime: string } | null>(null);
  const [addEventOpen, setAddEventOpen] = useState(false);
  const [addEventTime, setAddEventTime] = useState<string | undefined>(undefined);
  const [showEvents, setShowEvents] = useState(true);
  const [containerWidth, setContainerWidth] = useState(800);
  const containerRef = useRef<HTMLDivElement>(null);
  const echartsRef = useRef<ReactECharts>(null);
  const theme = useEChartsTheme();

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width ?? el.clientWidth;
      setContainerWidth(width);
      echartsRef.current?.getEchartsInstance().resize();
    });
    ro.observe(el);
    setContainerWidth(el.clientWidth);
    return () => ro.disconnect();
  }, []);

  const handleContextMenu = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    const chart = echartsRef.current?.getEchartsInstance();
    if (!chart) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const offsetX = e.clientX - rect.left;
    const offsetY = e.clientY - rect.top;
    const coords = chart.convertFromPixel({ gridIndex: 0 }, [offsetX, offsetY]);
    if (!coords) return;
    const ts = (coords as [number, number])[0];
    if (!ts || isNaN(ts)) return;
    chart.setOption({ tooltip: { show: false } }, false);
    setContextMenu({ x: offsetX, y: offsetY, isoTime: new Date(ts).toISOString() });
  }, []);

  useEffect(() => {
    if (!contextMenu) return;
    const close = () => {
      setContextMenu(null);
      echartsRef.current?.getEchartsInstance().setOption({ tooltip: { show: true } }, false);
    };
    window.addEventListener("click", close);
    window.addEventListener("keydown", close);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("keydown", close);
    };
  }, [contextMenu]);

  const { xMin, xMax, since } = useMemo(() => {
    const now = Date.now();
    const hours = RANGE_HOURS[range];
    if (!hours) return { xMin: undefined, xMax: undefined, since: undefined };
    const start = now - hours * 60 * 60 * 1000;
    return {
      xMin: start,
      xMax: now,
      since: new Date(start).toISOString(),
    };
  }, [range]);

  const { data: readings, isLoading } = useReadings({ brewId, since });
  const { data: events } = useBrewEvents(brewId);
  const { data: analytics } = useBrewAnalytics(brewId);

  const gravityData = useMemo(() => {
    if (!readings) return [];
    return readings
      .slice()
      .sort((a, b) => new Date(a.recordedAt).getTime() - new Date(b.recordedAt).getTime())
      .map((r): [number, number] => [new Date(r.recordedAt).getTime(), r.gravity]);
  }, [readings]);

  const tempData = useMemo(() => {
    if (!readings) return [];
    return readings
      .slice()
      .sort((a, b) => new Date(a.recordedAt).getTime() - new Date(b.recordedAt).getTime())
      .map((r): [number, number] => [new Date(r.recordedAt).getTime(), r.temperatureF]);
  }, [readings]);

  const visibleEvents = useMemo(() => {
    if (!events || gravityData.length === 0) return [];
    const minTs = gravityData[0][0];
    const maxTs = gravityData[gravityData.length - 1][0];
    return events.filter((e) => {
      const ts = new Date(e.eventTime).getTime();
      return ts >= minTs && ts <= maxTs;
    });
  }, [events, gravityData]);

  const visibleGaps = useMemo(() => {
    if (!analytics?.gaps || gravityData.length === 0) return [];
    const minTs = gravityData[0][0];
    const maxTs = gravityData[gravityData.length - 1][0];
    return analytics.gaps.filter((g) => {
      const startTs = new Date(g.startAt).getTime();
      const endTs = new Date(g.endAt).getTime();
      return endTs >= minTs && startTs <= maxTs;
    });
  }, [analytics, gravityData]);

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const onEvents = useMemo<Record<string, (params: any) => void>>(() => ({
    mouseover: (params) => {
      if (params?.seriesName === "Events" && params.event) {
        const ev = visibleEvents[params.dataIndex as number];
        if (ev) {
          setEventTooltip({
            event: ev,
            x: params.event.offsetX as number,
            y: params.event.offsetY as number,
          });
          echartsRef.current?.getEchartsInstance().setOption({ tooltip: { show: false } }, false);
        }
      }
    },
    mouseout: (params) => {
      if (params?.seriesName === "Events") {
        setEventTooltip(null);
        echartsRef.current?.getEchartsInstance().setOption({ tooltip: { show: true } }, false);
      }
    },
  }), [visibleEvents]);

  const option = useMemo((): EChartsOption => {
    const gravityValues = gravityData.map(([, v]) => v);
    const tempValues = tempData.map(([, v]) => v);
    const gMin = gravityValues.length > 0 ? Math.min(...gravityValues) : 1.0;
    const gMax = gravityValues.length > 0 ? Math.max(...gravityValues) : 1.1;
    const tMin = tempValues.length > 0 ? Math.min(...tempValues) : 32;
    const tMax = tempValues.length > 0 ? Math.max(...tempValues) : 100;

    const axisLabelFormatter = (val: number) => {
      const d = new Date(val);
      return range === "24h" ? format(d, "HH:mm") : format(d, "MMM d");
    };

    const staticMarkLines: unknown[] = [];
    if (targetFg != null) {
      staticMarkLines.push({
        name: "targetFg",
        yAxis: targetFg,
        lineStyle: { color: "#2F9E44", type: "dashed", width: 2 },
        label: {
          show: true,
          position: "insideEndTop",
          formatter: `Target FG: ${targetFg.toFixed(3)}`,
          color: "#2F9E44",
          fontSize: 11,
        },
      });
    }
    if (predictedFgDate != null) {
      staticMarkLines.push({
        name: "predictedFg",
        xAxis: new Date(predictedFgDate).getTime(),
        lineStyle: { color: "#9c36b5", type: "dashed", width: 2 },
        label: {
          show: true,
          position: "insideStartTop",
          formatter: "Predicted FG",
          color: "#9c36b5",
          fontSize: 10,
        },
      });
    }

    const gapMarkAreaData = visibleGaps.map((g) => [
      { xAxis: new Date(g.startAt).getTime() },
      { xAxis: new Date(g.endAt).getTime() },
    ]);

    const eventScatterData = showEvents
      ? visibleEvents.map((ev) => ({
          value: [new Date(ev.eventTime).getTime(), 0.5] as [number, number],
          itemStyle: { color: EVENT_COLORS[ev.eventType] },
        }))
      : [];

    // When range === "all", xMin/xMax are undefined and each axis would auto-fit
    // to its own series' data extent. Derive explicit bounds from gravity data so
    // both axes share the same range and the event diamonds align with the lines.
    const effectiveXMin = xMin ?? (gravityData.length > 0 ? gravityData[0][0] : undefined);
    const effectiveXMax = xMax ?? (gravityData.length > 0 ? gravityData[gravityData.length - 1][0] : undefined);

    const sharedXAxisConfig = {
      type: "time" as const,
      min: effectiveXMin,
      max: effectiveXMax,
      splitLine: { lineStyle: { color: theme.gridColor } },
      axisLine: { lineStyle: { color: theme.borderColor } },
      axisTick: { lineStyle: { color: theme.borderColor } },
    };

    return {
      backgroundColor: "transparent",
      animation: false,
      tooltip: {
        trigger: "axis",
        axisPointer: { type: "none" },
        backgroundColor: theme.popoverBg,
        borderColor: theme.borderColor,
        borderWidth: 1,
        padding: 10,
        textStyle: { color: theme.popoverFg, fontFamily: theme.fontFamily, fontSize: 13 },
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        formatter: (params: any) => {
          if (!Array.isArray(params) || params.length === 0) return "";
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          const dataItems = (params as any[]).filter((p) => p.seriesName !== "Events");
          if (dataItems.length === 0) return "";
          const ts: number = dataItems[0].axisValue;
          const title = format(new Date(ts), "MMM d, yyyy · HH:mm");
          let html = `<div style="font-size:12px;color:${theme.mutedColor};margin-bottom:6px">${title}</div>`;
          for (const item of dataItems as Array<{ seriesName: string; value: [number, number]; color: string }>) {
            if (item.value == null || item.value[1] == null) continue;
            const val = item.value[1];
            const isGravity = item.seriesName === "Gravity";
            const formatted = isGravity ? `${val.toFixed(3)} SG` : `${val.toFixed(1)}°F`;
            html += `<div style="display:flex;align-items:center;gap:8px;margin-bottom:2px">
              <span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:${item.color}"></span>
              <span>${item.seriesName}</span>
              <span style="margin-left:auto;font-weight:600">${formatted}</span>
            </div>`;
          }
          return html;
        },
      },
      legend: {
        data: ["Gravity", "Temperature"],
        bottom: 4,
        icon: "circle",
        textStyle: { color: theme.textColor, fontFamily: theme.fontFamily, fontSize: containerWidth < 640 ? 10 : 12 },
      },
      // Two grids: thin event strip at top / main chart below
      grid: containerWidth < 640
        ? [
            { left: 46, right: 34, top: 36, bottom: 56 },
            { left: 46, right: 34, top: 18, height: 12 },
          ]
        : [
            { left: 72, right: 64, top: 52, bottom: 48 },
            { left: 72, right: 64, top: 28, height: 14 },
          ],
      xAxis: [
        {
          gridIndex: 0,
          ...sharedXAxisConfig,
          axisLabel: { color: theme.mutedColor, fontFamily: theme.fontFamily, fontSize: containerWidth < 640 ? 9 : 11, formatter: axisLabelFormatter },
        },
        {
          // Event strip x-axis — hidden, just for positioning
          gridIndex: 1,
          ...sharedXAxisConfig,
          show: false,
        },
      ],
      yAxis: [
        {
          gridIndex: 0,
          name: "SG",
          nameTextStyle: { color: "#1971C2", fontFamily: theme.fontFamily, fontSize: containerWidth < 640 ? 9 : 11 },
          min: Math.floor(gMin * 1000 - 1) / 1000,
          max: Math.ceil(gMax * 1000 + 1) / 1000,
          splitNumber: containerWidth < 640 ? 3 : 5,
          axisLabel: {
            color: "#1971C2",
            fontFamily: theme.fontFamily,
            fontSize: containerWidth < 640 ? 9 : 11,
            formatter: (val: number) => val.toFixed(3),
          },
          splitLine: { lineStyle: { color: theme.gridColor } },
          axisLine: { lineStyle: { color: theme.borderColor } },
        },
        {
          gridIndex: 0,
          name: "°F",
          position: "right" as const,
          nameTextStyle: { color: "#E8590C", fontFamily: theme.fontFamily, fontSize: containerWidth < 640 ? 9 : 11 },
          min: Math.floor(tMin - 2),
          max: Math.ceil(tMax + 2),
          splitNumber: containerWidth < 640 ? 3 : 5,
          axisLabel: {
            color: "#E8590C",
            fontFamily: theme.fontFamily,
            fontSize: containerWidth < 640 ? 9 : 11,
            formatter: (val: number) => containerWidth < 640 ? `${val.toFixed(0)}°` : `${val.toFixed(1)}°F`,
          },
          splitLine: { show: false },
          axisLine: { lineStyle: { color: theme.borderColor } },
        },
        {
          // Event strip y-axis — hidden, fixed 0–1 range
          gridIndex: 1,
          min: 0,
          max: 1,
          show: false,
        },
      ],
      dataZoom: [
        { type: "inside", xAxisIndex: [0, 1] },
      ],
      series: [
        {
          name: "Gravity",
          type: "line",
          xAxisIndex: 0,
          yAxisIndex: 0,
          data: gravityData,
          lineStyle: { color: "#1971C2", width: 2 },
          itemStyle: { color: "#1971C2" },
          showSymbol: false,
          smooth: true,
          markLine: {
            silent: true,
            symbol: ["none", "none"],
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            data: [
              ...(showEvents ? visibleEvents.map((ev) => ({
                xAxis: new Date(ev.eventTime).getTime(),
                lineStyle: {
                  color: EVENT_COLORS[ev.eventType],
                  type: "dashed" as const,
                  width: 2,
                },
                label: { show: false },
              })) : []),
              ...staticMarkLines,
            ] as any,
          },
          markArea: gapMarkAreaData.length > 0
            ? {
                silent: true,
                itemStyle: { color: "rgba(239,68,68,0.12)", borderWidth: 0 },
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                data: gapMarkAreaData as any,
              }
            : undefined,
        },
        {
          name: "Temperature",
          type: "line",
          xAxisIndex: 0,
          yAxisIndex: 1,
          data: tempData,
          lineStyle: { color: "#E8590C", width: 2 },
          itemStyle: { color: "#E8590C" },
          showSymbol: false,
          smooth: true,
        },
        {
          // Colored diamond markers in the thin event strip above the chart
          name: "Events",
          type: "scatter",
          xAxisIndex: 1,
          yAxisIndex: 2,
          data: eventScatterData,
          symbol: "diamond",
          symbolSize: 14,
          emphasis: {
            scale: true,
            itemStyle: { borderColor: theme.bgColor, borderWidth: 2 },
          },
          // Excluded from axis tooltip; hover handled via onEvents
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          tooltip: { show: false } as any,
        },
      ],
    };
  }, [
    theme,
    gravityData,
    tempData,
    visibleEvents,
    visibleGaps,
    targetFg,
    predictedFgDate,
    xMin,
    xMax,
    range,
    containerWidth,
    showEvents,
  ]);

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-base">Readings Chart</CardTitle>
        <div className="flex flex-wrap gap-1 justify-end">
          <Button
            variant={showEvents ? "secondary" : "outline"}
            size="sm"
            className="h-7 px-2 text-xs"
            onClick={() => setShowEvents((v) => !v)}
            title={showEvents ? "Hide event lines" : "Show event lines"}
          >
            <Milestone className="h-3 w-3 mr-1" />
            Events
          </Button>
          <div className="flex gap-1">
            {(["24h", "7d", "30d", "all"] as TimeRange[]).map((r) => (
              <Button
                key={r}
                variant={range === r ? "default" : "outline"}
                size="sm"
                className="h-7 px-2 text-xs"
                onClick={() => setRange(r)}
              >
                {r === "all" ? "All" : r}
              </Button>
            ))}
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <Skeleton className="h-[280px] sm:h-[420px] w-full" />
        ) : gravityData.length === 0 ? (
          <div className="flex items-center justify-center h-[280px] sm:h-[420px] text-muted-foreground">
            No readings for this time range
          </div>
        ) : (
          <div ref={containerRef} className="relative" onContextMenu={handleContextMenu}>
            <ReactECharts
              ref={echartsRef}
              option={option}
              style={{ height: containerWidth < 640 ? 280 : 420 }}
              notMerge
              onEvents={onEvents}
            />
            {contextMenu && (
              <div
                className="absolute z-50 min-w-[160px] rounded-md border bg-popover text-popover-foreground shadow-md py-1"
                style={{ left: contextMenu.x, top: contextMenu.y }}
                onClick={(e) => e.stopPropagation()}
              >
                <button
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent hover:text-accent-foreground"
                  onClick={() => {
                    setAddEventTime(contextMenu.isoTime);
                    setAddEventOpen(true);
                    setContextMenu(null);
                  }}
                >
                  <CalendarPlus className="h-4 w-4" />
                  Add event at {format(new Date(contextMenu.isoTime), "MMM d, HH:mm")}
                </button>
              </div>
            )}
            {eventTooltip && (() => {
              const ev = eventTooltip.event;
              const color = EVENT_COLORS[ev.eventType];
              const label = EVENT_LABELS[ev.eventType];
              const containerWidth = containerRef.current?.clientWidth ?? 400;
              const tipWidth = 220;
              const rawLeft = eventTooltip.x + 14;
              const left = Math.min(rawLeft, containerWidth - tipWidth - 8);
              return (
                <div
                  className="absolute z-50 pointer-events-none rounded-lg border bg-popover text-popover-foreground shadow-lg"
                  style={{ left, top: eventTooltip.y - 10, width: tipWidth }}
                >
                  <div
                    className="flex items-center gap-2 px-3 pt-3 pb-2 border-b"
                    style={{ borderColor: `${color}40` }}
                  >
                    <span
                      style={{
                        display: "inline-block",
                        width: 10,
                        height: 10,
                        background: color,
                        transform: "rotate(45deg)",
                        borderRadius: 1,
                        flexShrink: 0,
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
                    {ev.label && (
                      <div className="text-xs font-medium text-foreground">
                        {ev.label}
                      </div>
                    )}
                    {ev.notes && (
                      <div className="text-xs text-muted-foreground leading-relaxed line-clamp-4">
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
      <CreateEventDialog
        brewId={brewId}
        open={addEventOpen}
        onOpenChange={setAddEventOpen}
        initialEventTime={addEventTime}
      />
    </Card>
  );
}
