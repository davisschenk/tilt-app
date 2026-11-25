import { useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { API_BASE_URL } from "@/lib/api";
import { useBrew } from "@/hooks/use-brews";
import { useBrewEvents } from "@/hooks/use-brew-events";
import { useReadings } from "@/hooks/use-readings";
import {
  useBrewReports,
  usePublicBrewReport,
  usePublishBrewReport,
  useRevokeBrewReport,
} from "@/hooks/use-brew-reports";
import { useUnits, unitTools } from "@/lib/units";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import type {
  BrewEventResponse,
  BrewReportEvent,
  BrewReportLink,
  BrewReportSnapshot,
} from "@/types";
import * as toast from "@/lib/toast";

// Chart geometry (SVG user units): the plot spans X_LEFT..X_RIGHT and
// Y_BOTTOM up to Y_BOTTOM - PLOT_HEIGHT.
const X_LEFT = 75;
const X_RIGHT = 795;
const Y_BOTTOM = 240;
const PLOT_HEIGHT = 190;
const GRID_FRACTIONS = [0, 0.25, 0.5, 0.75, 1];

/** Journal-style entries stay private unless explicitly selected. */
const PRIVATE_EVENT_TYPES = ["note", "tasting_note"];

type Extent = [lo: number, hi: number];

function extent(values: number[]): Extent {
  return [Math.min(...values), Math.max(...values)];
}

function ReportView({ report: r }: { report: BrewReportSnapshot }) {
  const u = useMemo(() => unitTools(r.units), [r.units]);
  const rows = r.readings;
  // Extents are computed once per snapshot; the conversions are monotonic, so
  // converting the canonical extent gives the displayed one.
  const chart = useMemo(() => {
    const times = rows.map((p) => Date.parse(p.recordedAt));
    const gravity = extent(rows.map((p) => p.gravity));
    const temperatureF = extent(rows.map((p) => p.temperatureF));
    return {
      times,
      start: times[0] ?? 0,
      end: times.at(-1) ?? 1,
      gravity,
      temperatureF,
      gravityDisplay: gravity.map(u.gravity) as Extent,
      temperatureDisplay: temperatureF.map(u.temperature) as Extent,
    };
  }, [rows, u]);
  const { times, start, end } = chart;

  const xAt = (t: number) =>
    X_LEFT + ((t - start) / Math.max(end - start, 1)) * (X_RIGHT - X_LEFT);
  const yAt = (v: number, [lo, hi]: Extent) =>
    Y_BOTTOM - ((v - lo) / Math.max(hi - lo, 0.001)) * PLOT_HEIGHT;
  const path = (value: (i: number) => number, range: Extent) =>
    rows
      .map((_, i) => `${i ? "L" : "M"}${xAt(times[i])},${yAt(value(i), range)}`)
      .join(" ");
  const lerp = ([lo, hi]: Extent, f: number) => lo + f * (hi - lo);
  const markers = r.events.filter((e) => {
    const t = Date.parse(e.eventTime);
    return t >= start && t <= end;
  });

  return (
    <article className="report-body space-y-6">
      <header>
        <h1 className="text-3xl font-bold">{r.name}</h1>
        <p>
          {r.style} · {r.status}
        </p>
        <p>
          {r.startDate ? new Date(r.startDate).toLocaleDateString() : ""}
          {r.endDate ? ` – ${new Date(r.endDate).toLocaleDateString()}` : ""}
        </p>
      </header>
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
        <p>OG: {u.formatGravity(r.og)}</p>
        <p>FG: {u.formatGravity(r.fg)}</p>
        <p>Target: {u.formatGravity(r.targetFg)}</p>
        <p>ABV: {r.abv == null ? "—" : `${r.abv.toFixed(1)}%`}</p>
      </div>
      <p>Yeast: {r.yeastStrain ?? "—"}</p>
      <section>
        <h2 className="text-xl font-semibold">Fermentation</h2>
        {rows.length ? (
          <>
            <svg
              viewBox="0 0 900 285"
              role="img"
              aria-label="Gravity and temperature over time"
              className="w-full"
            >
              {GRID_FRACTIONS.map((fraction) => {
                const y = Y_BOTTOM - fraction * PLOT_HEIGHT;
                return (
                  <g key={fraction}>
                    <line x1={X_LEFT} x2={X_RIGHT} y1={y} y2={y} stroke="#ddd" />
                    <text
                      x={X_LEFT - 7}
                      y={y + 4}
                      textAnchor="end"
                      fontSize="11"
                      fill="#1971c2"
                    >
                      {u.formatDisplayNumber(
                        "gravity",
                        lerp(chart.gravityDisplay, fraction),
                      )}
                    </text>
                    <text x={X_RIGHT + 10} y={y + 4} fontSize="11" fill="#d9480f">
                      {u.formatDisplayNumber(
                        "temperature",
                        lerp(chart.temperatureDisplay, fraction),
                      )}
                    </text>
                  </g>
                );
              })}
              <path
                d={path((i) => u.gravity(rows[i].gravity), chart.gravityDisplay)}
                fill="none"
                stroke="#1971c2"
                strokeWidth="2"
              />
              <path
                d={path(
                  (i) => u.temperature(rows[i].temperatureF),
                  chart.temperatureDisplay,
                )}
                fill="none"
                stroke="#d9480f"
                strokeWidth="2"
              />
              {markers.map((e, i) => {
                const x = xAt(Date.parse(e.eventTime));
                return (
                  <line
                    key={i}
                    x1={x}
                    x2={x}
                    y1="35"
                    y2="245"
                    stroke="#888"
                    strokeDasharray="3 5"
                  >
                    <title>{e.label}</title>
                  </line>
                );
              })}
              <text x={X_LEFT} y="275" fill="currentColor" fontSize="12">
                {new Date(start).toLocaleString()}
              </text>
              <text
                x={X_RIGHT}
                y="275"
                textAnchor="end"
                fill="currentColor"
                fontSize="12"
              >
                {new Date(end).toLocaleString()}
              </text>
            </svg>
            <p className="text-sm">
              <span className="text-blue-700">Gravity ({u.gravityLabel})</span>:{" "}
              {u.formatGravity(chart.gravity[0])} –{" "}
              {u.formatGravity(chart.gravity[1])};{" "}
              <span className="text-orange-700">
                temperature ({u.temperatureLabel})
              </span>
              : {u.formatTemperature(chart.temperatureF[0])} –{" "}
              {u.formatTemperature(chart.temperatureF[1])}. Each line uses its own vertical scale.
            </p>
          </>
        ) : (
          <p>No readings recorded.</p>
        )}
      </section>
      {r.notes && (
        <section>
          <h2 className="text-xl font-semibold">Batch notes</h2>
          <p className="whitespace-pre-wrap">{r.notes}</p>
        </section>
      )}
      <section>
        <h2 className="text-xl font-semibold">Selected log entries</h2>
        {r.events.length === 0 && <p>No entries selected.</p>}
        {r.events.map((e, i) => (
          <div key={i} className="report-entry border-b py-4">
            <h3 className="font-semibold">{e.label}</h3>
            <p>
              {new Date(e.eventTime).toLocaleString()} ·{" "}
              {u.formatGravity(e.gravityAtEvent)} ·{" "}
              {u.formatTemperature(e.tempAtEvent)}
            </p>
            {e.notes && <p className="whitespace-pre-wrap">{e.notes}</p>}
            <div className="flex flex-wrap gap-3">
              {e.photos.map((p) => (
                <img
                  key={p.id}
                  src={
                    p.url.startsWith("/api/v1/")
                      ? `${API_BASE_URL}${p.url.slice(7)}`
                      : p.url
                  }
                  alt={p.filename}
                  className="max-h-64 max-w-full"
                />
              ))}
            </div>
          </div>
        ))}
      </section>
      <p className="text-sm">
        Report snapshot: {new Date(r.createdAt).toLocaleString()}
      </p>
    </article>
  );
}
export function PublicBrewReport() {
  const { token } = useParams();
  const q = usePublicBrewReport(token);
  return (
    <main className="max-w-5xl mx-auto p-6">
      <div className="no-print mb-6">
        <Button onClick={() => window.print()}>Print / Save PDF</Button>
      </div>
      {q.isPending ? (
        <p>Loading report…</p>
      ) : q.isError ? (
        <p>This report is unavailable or its link has been revoked.</p>
      ) : (
        <ReportView report={q.data} />
      )}
    </main>
  );
}

export default function BrewReport() {
  const { id = "" } = useParams();
  const brew = useBrew(id);
  const events = useBrewEvents(id);
  const readings = useReadings({ brewId: id });
  const { units } = useUnits();
  const shares = useBrewReports(id);
  const publishReport = usePublishBrewReport(id);
  const revokeReport = useRevokeBrewReport(id);
  const [selection, setSelection] = useState<Record<string, boolean>>({});
  const [notes, setNotes] = useState(false);
  const [photos, setPhotos] = useState(false);
  const busy = publishReport.isPending || revokeReport.isPending;

  const isSelected = (e: BrewEventResponse) =>
    selection[e.id] ?? !PRIVATE_EVENT_TYPES.includes(e.eventType);
  const selected = (events.data ?? []).filter(isSelected);

  async function publish(old?: BrewReportLink) {
    const options = {
      eventIds: selected.map((e) => e.id),
      includeNotes: notes,
      includePhotos: photos,
      units,
    };
    try {
      const result = await publishReport.mutateAsync({
        options,
        reportId: old?.id,
      });
      toast.success(old ? "Snapshot refreshed" : "Report link created");
      if (!old) {
        await navigator.clipboard
          ?.writeText(`${location.origin}/reports/${result.token}`)
          .catch(() => {});
      }
    } catch {
      toast.error("Could not publish report");
    }
  }

  function revoke(link: BrewReportLink) {
    revokeReport.mutate(link.id, {
      onError: () => toast.error("Could not revoke link"),
    });
  }

  if (brew.isPending || events.isPending || readings.isPending)
    return <p>Loading report preview…</p>;
  if (!brew.data || events.isError || readings.isError)
    return <p>Could not load report preview.</p>;
  const b = brew.data;
  const mapEvent = (e: BrewEventResponse): BrewReportEvent => ({
    ...e,
    notes: notes ? e.notes : null,
    photos: photos
      ? e.attachments.map((a) => ({
          id: a.id,
          filename: a.filename,
          url: a.url,
        }))
      : [],
  });
  const report: BrewReportSnapshot = {
    ...b,
    abv: b.finalAbv,
    units,
    notes: notes ? b.notes : null,
    events: selected.map(mapEvent),
    readings: readings.data ?? [],
    createdAt: new Date().toISOString(),
  };
  return (
    <div className="max-w-5xl mx-auto p-4">
      <div className="no-print space-y-4 mb-8">
        <Link to={`/brews/${id}`}>← Back to batch</Link>
        <h1 className="text-2xl font-bold">Report preview</h1>
        <p>
          Only the selected content below will be shared. Published reports stay
          fixed until refreshed.
        </p>
        <div className="flex items-center gap-2">
          <Switch id="report-notes" checked={notes} onCheckedChange={setNotes} />
          <Label htmlFor="report-notes">Include batch and entry notes</Label>
        </div>
        <div className="flex items-center gap-2">
          <Switch id="report-photos" checked={photos} onCheckedChange={setPhotos} />
          <Label htmlFor="report-photos">Include selected entries’ photos</Label>
        </div>
        <fieldset className="space-y-2">
          <legend>Log entries</legend>
          {events.data?.map((e) => (
            <div key={e.id} className="flex items-center gap-2">
              <Switch
                id={`report-event-${e.id}`}
                checked={isSelected(e)}
                onCheckedChange={(checked) =>
                  setSelection({ ...selection, [e.id]: checked })
                }
              />
              <Label htmlFor={`report-event-${e.id}`}>
                {e.label} ({e.eventType.replaceAll("_", " ")})
              </Label>
            </div>
          ))}
        </fieldset>
        <div className="flex gap-3">
          <Button disabled={busy} onClick={() => publish()}>
            Create share link
          </Button>
          <Button variant="outline" onClick={() => window.print()}>
            Print / Save PDF
          </Button>
        </div>
        {shares.isError && <p>Could not load existing share links.</p>}
        {shares.data?.map((s) => (
          <div key={s.id} className="border p-3 space-y-2">
            <a className="break-all underline" href={`/reports/${s.token}`}>
              {location.origin}/reports/{s.token}
            </a>
            <div className="flex gap-2">
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => publish(s)}
              >
                Refresh snapshot
              </Button>
              <Button
                variant="destructive"
                disabled={busy}
                onClick={() => revoke(s)}
              >
                Revoke link
              </Button>
            </div>
          </div>
        ))}
      </div>
      <ReportView report={report} />
    </div>
  );
}
