import { useMemo } from "react";
import ReactECharts from "echarts-for-react";
import type { EChartsOption } from "echarts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useReadings } from "@/hooks/use-readings";
import { useEChartsTheme } from "@/lib/echarts-theme";

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
  const theme = useEChartsTheme();

  const { since, xMin, xMax } = useMemo(() => {
    const now = Date.now();
    const start = now - 24 * 60 * 60 * 1000;
    return {
      since: new Date(start).toISOString(),
      xMin: start,
      xMax: now,
    };
  }, []);

  const { data: readings, isLoading } = useReadings({ since });

  const seriesData = useMemo(() => {
    if (!readings || readings.length === 0) return [];
    const byColor = new Map<string, [number, number][]>();
    for (const r of readings) {
      const pts = byColor.get(r.color) ?? [];
      pts.push([new Date(r.recordedAt).getTime(), r.gravity]);
      byColor.set(r.color, pts);
    }
    return Array.from(byColor.entries()).map(([color, data]) => ({
      color,
      data: data.slice().sort((a, b) => a[0] - b[0]),
    }));
  }, [readings]);

  const option = useMemo((): EChartsOption => ({
    backgroundColor: "transparent",
    animation: false,
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "cross" },
      backgroundColor: theme.popoverBg,
      borderColor: theme.borderColor,
      borderWidth: 1,
      padding: 10,
      textStyle: { color: theme.popoverFg, fontFamily: theme.fontFamily, fontSize: 13 },
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      formatter: (params: any) => {
        if (!params || !Array.isArray(params) || params.length === 0) return "";
        const ts: number = params[0].axisValue;
        const d = new Date(ts);
        const title = `${d.toLocaleDateString([], { month: "short", day: "numeric" })} · ${d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
        let html = `<div style="font-size:12px;color:${theme.mutedColor};margin-bottom:6px">${title}</div>`;
        for (const item of params as Array<{ seriesName: string; value: [number, number]; color: string }>) {
          if (item.value == null || item.value[1] == null) continue;
          const val = item.value[1];
          html += `<div style="display:flex;align-items:center;gap:8px;margin-bottom:2px">
            <span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:${item.color}"></span>
            <span>${item.seriesName}</span>
            <span style="margin-left:auto;font-weight:600">${val.toFixed(3)} SG</span>
          </div>`;
        }
        return html;
      },
    },
    legend: {
      bottom: 5,
      icon: "circle",
      textStyle: { color: theme.textColor, fontFamily: theme.fontFamily, fontSize: 12 },
    },
    grid: { left: 72, right: 20, top: 10, bottom: 80 },
    xAxis: {
      type: "time",
      min: xMin,
      max: xMax,
      axisLabel: {
        color: theme.mutedColor,
        fontFamily: theme.fontFamily,
        fontSize: 11,
        formatter: (val: number) => {
          const d = new Date(val);
          return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
        },
      },
      splitLine: { lineStyle: { color: theme.gridColor } },
      axisLine: { lineStyle: { color: theme.borderColor } },
      axisTick: { lineStyle: { color: theme.borderColor } },
    },
    yAxis: {
      axisLabel: {
        color: theme.mutedColor,
        fontFamily: theme.fontFamily,
        fontSize: 11,
        formatter: (val: number) => val.toFixed(3),
      },
      splitLine: { lineStyle: { color: theme.gridColor } },
      axisLine: { lineStyle: { color: theme.borderColor } },
    },
    dataZoom: [
      { type: "inside" },
      {
        type: "slider",
        bottom: 32,
        height: 18,
        borderColor: theme.borderColor,
        textStyle: { color: theme.mutedColor, fontSize: 10 },
      },
    ],
    series: seriesData.map(({ color, data }) => ({
      name: color,
      type: "line",
      data,
      lineStyle: { color: BREW_COLORS[color] ?? "#868E96", width: 2 },
      itemStyle: { color: BREW_COLORS[color] ?? "#868E96" },
      showSymbol: false,
      smooth: true,
    })),
  }), [theme, seriesData, xMin, xMax]);

  return (
    <Card className="mt-8">
      <CardHeader>
        <CardTitle className="text-lg">Recent Readings (24h)</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <Skeleton className="h-[300px] w-full" />
        ) : seriesData.length === 0 ? (
          <div className="flex items-center justify-center h-[300px] text-muted-foreground">
            No readings in the last 24 hours
          </div>
        ) : (
          <ReactECharts option={option} style={{ height: 300 }} notMerge />
        )}
      </CardContent>
    </Card>
  );
}
