import { useState, useMemo, useRef } from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  Legend,
  ReferenceLine,
  ReferenceArea,
  ResponsiveContainer,
  Customized,
} from "recharts";
import { format } from "date-fns";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useReadings } from "@/hooks/use-readings";
import { useBrewEvents } from "@/hooks/use-brew-events";
import { useBrewAnalytics } from "@/hooks/use-brew-analytics";
import type { BrewEventType, BrewEventResponse } from "@/types";

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

  const tickFormat = range === "24h" ? "HH:mm" : "MMM d HH:mm";

  const chartData = useMemo(() => {
    if (!readings || readings.length === 0) return [];
    return readings
      .slice()
      .sort((a, b) => new Date(a.recordedAt).getTime() - new Date(b.recordedAt).getTime())
      .map((r) => ({
        timestamp: new Date(r.recordedAt).getTime(),
        gravity: r.gravity,
        temperature: r.temperatureF,
      }));
  }, [readings]);

  const visibleEvents = useMemo(() => {
    if (!events || chartData.length === 0) return [];
    const minTs = chartData[0].timestamp;
    const maxTs = chartData[chartData.length - 1].timestamp;
    return events.filter((e) => {
      const ts = new Date(e.eventTime).getTime();
      return ts >= minTs && ts <= maxTs;
    });
  }, [events, chartData]);

  const visibleGaps = useMemo(() => {
    if (!analytics?.gaps || chartData.length === 0) return [];
    const minTs = chartData[0].timestamp;
    const maxTs = chartData[chartData.length - 1].timestamp;
    return analytics.gaps
      .filter((g) => {
        const startTs = new Date(g.startAt).getTime();
        const endTs = new Date(g.endAt).getTime();
        return endTs >= minTs && startTs <= maxTs;
      })
      .map((g) => ({
        x1: new Date(g.startAt).getTime(),
        x2: new Date(g.endAt).getTime(),
        durationMinutes: g.durationMinutes,
      }));
  }, [analytics, chartData]);

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
        ) : chartData.length === 0 ? (
          <div className="flex items-center justify-center h-72 text-muted-foreground">
            No readings for this time range
          </div>
        ) : (
          <div ref={chartWrapperRef} className="relative">
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData} onMouseLeave={() => setHoveredEvent(null)}>
              <XAxis
                dataKey="timestamp"
                type="number"
                scale="time"
                domain={["dataMin", "dataMax"]}
                tick={{ fontSize: 11 }}
                stroke="var(--muted-foreground)"
                interval="preserveStartEnd"
                minTickGap={60}
                tickFormatter={(ts: number) => format(new Date(ts), tickFormat)}
              />
              <YAxis
                yAxisId="gravity"
                domain={[(min: number) => Math.floor(min * 1000 - 1) / 1000, (max: number) => Math.ceil(max * 1000 + 1) / 1000]}
                allowDataOverflow={false}
                tick={{ fontSize: 11 }}
                stroke="#1971C2"
                tickFormatter={(v: number) => v.toFixed(3)}
              />
              <YAxis
                yAxisId="temp"
                orientation="right"
                domain={[(min: number) => Math.floor(min - 1), (max: number) => Math.ceil(max + 1)]}
                allowDataOverflow={false}
                tick={{ fontSize: 11 }}
                stroke="#E8590C"
                tickFormatter={(v: number) => `${v.toFixed(1)}°F`}
              />
              <Tooltip
                contentStyle={{ backgroundColor: "var(--card)", borderColor: "var(--border)", color: "var(--foreground)" }}
                formatter={(value: unknown, name?: string) => {
                  const v = typeof value === "number" ? value : 0;
                  if (name === "gravity") return [v.toFixed(3), "Gravity (SG)"];
                  return [`${v.toFixed(1)}°F`, "Temperature"];
                }}
                labelFormatter={(label) => `Time: ${format(new Date(Number(label)), "MMM d HH:mm")}`}
              />
              {targetFg != null && (
                <ReferenceLine
                  yAxisId="gravity"
                  y={targetFg}
                  stroke="#2F9E44"
                  strokeDasharray="6 4"
                  strokeWidth={2}
                  label={{ value: `Target FG: ${targetFg.toFixed(3)}`, position: "insideTopRight", fontSize: 11, fill: "#2F9E44" }}
                />
              )}
              {predictedFgDate != null && (
                <ReferenceLine
                  yAxisId="gravity"
                  x={new Date(predictedFgDate).getTime()}
                  stroke="#9c36b5"
                  strokeDasharray="5 3"
                  strokeWidth={2}
                  label={{ value: "Predicted FG", position: "insideTopRight", fontSize: 10, fill: "#9c36b5" }}
                />
              )}
              {visibleGaps.map((gap, i) => (
                <ReferenceArea
                  key={`gap-${i}`}
                  yAxisId="gravity"
                  x1={gap.x1}
                  x2={gap.x2}
                  fill="#ef4444"
                  fillOpacity={0.15}
                  label={{
                    value: `No data (${Math.floor(gap.durationMinutes / 60)}h ${Math.round(gap.durationMinutes % 60)}m)`,
                    position: "insideTop",
                    fontSize: 9,
                    fill: "#ef4444",
                  }}
                />
              ))}
              {visibleEvents.map((ev) => {
                const color = EVENT_COLORS[ev.eventType];
                const shortLabel = EVENT_LABELS[ev.eventType];
                return (
                  <ReferenceLine
                    key={ev.id}
                    yAxisId="gravity"
                    x={new Date(ev.eventTime).getTime()}
                    stroke={color}
                    strokeDasharray="4 3"
                    strokeWidth={1.5}
                    label={{ value: shortLabel, position: "insideTopLeft", fontSize: 9, fill: color, angle: -90 }}
                  />
                );
              })}
              <Customized component={(props: Record<string, unknown>) => {
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                const xAxisMap = props.xAxisMap as Record<string, any>;
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                const yAxisMap = props.yAxisMap as Record<string, any>;
                if (!xAxisMap || !yAxisMap) return null;
                const xAxis = Object.values(xAxisMap)[0];
                const yAxis = Object.values(yAxisMap).find((a: any) => a.yAxisId === "gravity" || a.axisId === "gravity") ?? Object.values(yAxisMap)[0];
                if (!xAxis?.scale || !yAxis?.scale) return null;
                const plotY = yAxis.scale(yAxis.domain[1]) + 4;
                return (
                  <g>
                    {visibleEvents.map((ev) => {
                      const color = EVENT_COLORS[ev.eventType];
                      const px = xAxis.scale(new Date(ev.eventTime).getTime());
                      if (px == null || isNaN(px)) return null;
                      return (
                        <polygon
                          key={ev.id}
                          points={`${px},${plotY} ${px + 6},${plotY + 10} ${px},${plotY + 20} ${px - 6},${plotY + 10}`}
                          fill={color}
                          stroke="white"
                          strokeWidth={1}
                          style={{ cursor: "pointer" }}
                          onMouseEnter={(e) => {
                            const wrapper = chartWrapperRef.current;
                            if (!wrapper) return;
                            const rect = wrapper.getBoundingClientRect();
                            setHoveredEvent({
                              event: ev,
                              x: e.clientX - rect.left,
                              y: e.clientY - rect.top,
                            });
                          }}
                          onMouseLeave={() => setHoveredEvent(null)}
                        />
                      );
                    })}
                  </g>
                );
              }} />
              <Legend />
              <Line
                yAxisId="gravity"
                type="monotone"
                dataKey="gravity"
                stroke="#1971C2"
                dot={false}
                strokeWidth={2}
                name="gravity"
              />
              <Line
                yAxisId="temp"
                type="monotone"
                dataKey="temperature"
                stroke="#E8590C"
                dot={false}
                strokeWidth={2}
                name="temperature"
              />
            </LineChart>
          </ResponsiveContainer>
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
