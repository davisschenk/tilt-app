import { useState, useMemo } from "react";
import ReactECharts from "echarts-for-react";
import type { EChartsOption } from "echarts";
import { format } from "date-fns";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useReadings } from "@/hooks/use-readings";
import { useBrewEvents } from "@/hooks/use-brew-events";
import { useBrewAnalytics } from "@/hooks/use-brew-analytics";
import type { BrewEventType } from "@/types";
import { useEChartsTheme } from "@/lib/echarts-theme";

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

export default function ReadingsChart({ brewId, targetFg, predictedFgDate }: ReadingsChartProps) {
  const [range, setRange] = useState<TimeRange>("7d");
  const theme = useEChartsTheme();

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

    // Build event markLine items
    const eventMarkLines = visibleEvents.map((ev) => ({
      name: EVENT_LABELS[ev.eventType],
      xAxis: new Date(ev.eventTime).getTime(),
      lineStyle: {
        color: EVENT_COLORS[ev.eventType],
        type: "dashed" as const,
        width: 1.5,
      },
      label: {
        show: true,
        position: "insideStartTop" as const,
        formatter: EVENT_LABELS[ev.eventType],
        color: EVENT_COLORS[ev.eventType],
        fontSize: 10,
      },
    }));

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

    const allMarkLineData = [...eventMarkLines, ...staticMarkLines];

    const gapMarkAreaData = visibleGaps.map((g) => [
      { xAxis: new Date(g.startAt).getTime() },
      { xAxis: new Date(g.endAt).getTime() },
    ]);

    return {
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
          const title = format(new Date(ts), "MMM d, yyyy · HH:mm");
          let html = `<div style="font-size:12px;color:${theme.mutedColor};margin-bottom:6px">${title}</div>`;
          for (const item of params as Array<{ seriesName: string; value: [number, number]; color: string }>) {
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
      axisPointer: {
        link: [{ xAxisIndex: "all" }],
      },
      legend: {
        data: ["Gravity", "Temperature"],
        bottom: 0,
        textStyle: { color: theme.textColor, fontFamily: theme.fontFamily, fontSize: 12 },
      },
      grid: [
        { left: 72, right: 64, top: 12, bottom: "54%" },
        { left: 72, right: 64, top: "50%", bottom: 32 },
      ],
      xAxis: [
        {
          gridIndex: 0,
          type: "time",
          min: xMin,
          max: xMax,
          axisLabel: { color: theme.mutedColor, fontFamily: theme.fontFamily, fontSize: 11, formatter: axisLabelFormatter },
          splitLine: { lineStyle: { color: theme.gridColor } },
          axisLine: { lineStyle: { color: theme.borderColor } },
          axisTick: { lineStyle: { color: theme.borderColor } },
        },
        {
          gridIndex: 1,
          type: "time",
          min: xMin,
          max: xMax,
          axisLabel: { color: theme.mutedColor, fontFamily: theme.fontFamily, fontSize: 11, formatter: axisLabelFormatter },
          splitLine: { lineStyle: { color: theme.gridColor } },
          axisLine: { lineStyle: { color: theme.borderColor } },
          axisTick: { lineStyle: { color: theme.borderColor } },
        },
      ],
      yAxis: [
        {
          gridIndex: 0,
          name: "SG",
          nameTextStyle: { color: "#1971C2", fontFamily: theme.fontFamily, fontSize: 11 },
          min: Math.floor(gMin * 1000 - 1) / 1000,
          max: Math.ceil(gMax * 1000 + 1) / 1000,
          axisLabel: {
            color: "#1971C2",
            fontFamily: theme.fontFamily,
            fontSize: 11,
            formatter: (val: number) => val.toFixed(3),
          },
          splitLine: { lineStyle: { color: theme.gridColor } },
          axisLine: { lineStyle: { color: theme.borderColor } },
        },
        {
          gridIndex: 1,
          name: "°F",
          nameTextStyle: { color: "#E8590C", fontFamily: theme.fontFamily, fontSize: 11 },
          min: Math.floor(tMin - 2),
          max: Math.ceil(tMax + 2),
          axisLabel: {
            color: "#E8590C",
            fontFamily: theme.fontFamily,
            fontSize: 11,
            formatter: (val: number) => `${val.toFixed(1)}°F`,
          },
          splitLine: { show: false },
          axisLine: { lineStyle: { color: theme.borderColor } },
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
            silent: false,
            symbol: ["none", "none"],
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            data: allMarkLineData as any,
            tooltip: {
              show: true,
              backgroundColor: theme.popoverBg,
              borderColor: theme.borderColor,
              borderWidth: 1,
              padding: 10,
              textStyle: { color: theme.popoverFg, fontFamily: theme.fontFamily, fontSize: 12 },
              // eslint-disable-next-line @typescript-eslint/no-explicit-any
              formatter: (params: any) => {
                const xVal: number | undefined = params?.data?.xAxis;
                if (xVal == null) return "";
                const ev = visibleEvents.find(
                  (e) => new Date(e.eventTime).getTime() === xVal,
                );
                if (!ev) return "";
                const color = EVENT_COLORS[ev.eventType];
                const label = EVENT_LABELS[ev.eventType];
                const time = format(new Date(ev.eventTime), "MMM d, yyyy · HH:mm");
                let html = `<div style="font-weight:600;color:${color};margin-bottom:4px">${label}</div>`;
                html += `<div style="font-size:11px;color:${theme.mutedColor}">${time}</div>`;
                if (ev.notes) {
                  html += `<div style="font-size:12px;margin-top:4px;max-width:200px">${ev.notes}</div>`;
                }
                return html;
              },
            },
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
          xAxisIndex: 1,
          yAxisIndex: 1,
          data: tempData,
          lineStyle: { color: "#E8590C", width: 2 },
          itemStyle: { color: "#E8590C" },
          showSymbol: false,
          smooth: true,
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
  ]);

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
          <Skeleton className="h-[420px] w-full" />
        ) : gravityData.length === 0 ? (
          <div className="flex items-center justify-center h-[420px] text-muted-foreground">
            No readings for this time range
          </div>
        ) : (
          <ReactECharts option={option} style={{ height: 420 }} notMerge />
        )}
      </CardContent>
    </Card>
  );
}
