import { useState, useMemo } from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  Legend,
  ReferenceLine,
  ResponsiveContainer,
} from "recharts";
import { format } from "date-fns";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useReadings } from "@/hooks/use-readings";

type TimeRange = "24h" | "7d" | "30d" | "all";

const RANGE_HOURS: Record<TimeRange, number | null> = {
  "24h": 24,
  "7d": 168,
  "30d": 720,
  all: null,
};

interface ReadingsChartProps {
  brewId: string;
  targetFg?: number | null;
}

export default function ReadingsChart({ brewId, targetFg }: ReadingsChartProps) {
  const [range, setRange] = useState<TimeRange>("7d");

  const since = useMemo(() => {
    const hours = RANGE_HOURS[range];
    if (!hours) return undefined;
    const d = new Date();
    d.setHours(d.getHours() - hours);
    return d.toISOString();
  }, [range]);

  const { data: readings, isLoading } = useReadings({ brewId, since });

  const chartData = useMemo(() => {
    if (!readings || readings.length === 0) return [];
    return readings
      .slice()
      .sort((a, b) => new Date(a.recordedAt).getTime() - new Date(b.recordedAt).getTime())
      .map((r) => ({
        time: format(new Date(r.recordedAt), range === "24h" ? "HH:mm" : "MMM d HH:mm"),
        timestamp: new Date(r.recordedAt).getTime(),
        gravity: r.gravity,
        temperature: r.temperatureF,
      }));
  }, [readings, range]);

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
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <XAxis
                dataKey="time"
                tick={{ fontSize: 11 }}
                stroke="hsl(var(--muted-foreground))"
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
                formatter={(value: unknown, name?: string) => {
                  const v = typeof value === "number" ? value : 0;
                  if (name === "gravity") return [v.toFixed(3), "Gravity (SG)"];
                  return [`${v.toFixed(1)}°F`, "Temperature"];
                }}
                labelFormatter={(label) => `Time: ${String(label)}`}
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
        )}
      </CardContent>
    </Card>
  );
}
