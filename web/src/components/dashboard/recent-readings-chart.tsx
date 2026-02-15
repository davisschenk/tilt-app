import { useMemo } from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";
import { format } from "date-fns";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useReadings } from "@/hooks/use-readings";

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

  const { chartData, colors } = useMemo(() => {
    if (!readings || readings.length === 0) return { chartData: [], colors: [] };

    const byTime = new Map<string, Record<string, number>>();
    const colorSet = new Set<string>();

    for (const r of readings) {
      colorSet.add(r.color);
      const timeKey = r.recordedAt;
      const existing = byTime.get(timeKey) ?? {};
      existing[r.color] = r.gravity;
      byTime.set(timeKey, existing);
    }

    const sortedEntries = Array.from(byTime.entries()).sort(
      ([a], [b]) => new Date(a).getTime() - new Date(b).getTime(),
    );

    const data = sortedEntries.map(([time, values]) => ({
      time: format(new Date(time), "HH:mm"),
      timestamp: new Date(time).getTime(),
      ...values,
    }));

    return { chartData: data, colors: Array.from(colorSet) };
  }, [readings]);

  return (
    <Card className="mt-8">
      <CardHeader>
        <CardTitle className="text-lg">Recent Readings (24h)</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <Skeleton className="h-64 w-full" />
        ) : chartData.length === 0 ? (
          <div className="flex items-center justify-center h-64 text-muted-foreground">
            No readings in the last 24 hours
          </div>
        ) : (
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <XAxis
                dataKey="time"
                tick={{ fontSize: 12 }}
                stroke="hsl(var(--muted-foreground))"
              />
              <YAxis
                domain={["auto", "auto"]}
                tick={{ fontSize: 12 }}
                stroke="hsl(var(--muted-foreground))"
                tickFormatter={(v: number) => v.toFixed(3)}
              />
              <Tooltip
                formatter={(value: unknown) => [
                  typeof value === "number" ? value.toFixed(3) : String(value),
                  "SG",
                ]}
                labelFormatter={(label) => `Time: ${String(label)}`}
              />
              <Legend />
              {colors.map((color) => (
                <Line
                  key={color}
                  type="monotone"
                  dataKey={color}
                  stroke={BREW_COLORS[color] ?? "#868E96"}
                  dot={false}
                  strokeWidth={2}
                  connectNulls
                  name={color}
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        )}
      </CardContent>
    </Card>
  );
}
