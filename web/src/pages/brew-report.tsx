import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPost, apiPut, apiDelete, API_BASE_URL } from "@/lib/api";
import { useBrew } from "@/hooks/use-brews";
import { useBrewEvents } from "@/hooks/use-brew-events";
import { useReadings } from "@/hooks/use-readings";
import { useUnits, unitTools, type Preferences } from "@/lib/units";
import { Button } from "@/components/ui/button";
import type { BrewEventResponse } from "@/types";
import * as toast from "@/lib/toast";
interface ReportEvent {
  label: string;
  eventType: string;
  eventTime: string;
  gravityAtEvent: number | null;
  tempAtEvent: number | null;
  notes: string | null;
  photos: { id: string; filename: string; url: string }[];
}
interface Report {
  name: string;
  style: string | null;
  status: string;
  og: number | null;
  fg: number | null;
  targetFg: number | null;
  abv: number | null;
  yeastStrain: string | null;
  startDate: string | null;
  endDate: string | null;
  notes: string | null;
  units: Preferences;
  events: ReportEvent[];
  readings: { recordedAt: string; gravity: number; temperatureF: number }[];
  createdAt: string;
}
interface Share {
  id: string;
  token: string;
  createdAt: string;
}
function ReportView({ report: r }: { report: Report }) {
  const u = unitTools(r.units);
  const rows = r.readings;
  const dates = rows.map((p) => Date.parse(p.recordedAt));
  const min = dates[0] ?? 0,
    max = dates.at(-1) ?? 1;
  function path(kind: "gravity" | "temperatureF") {
    const values = rows.map((p) =>
      kind === "gravity" ? u.gravity(p.gravity) : u.temperature(p.temperatureF),
    );
    const lo = Math.min(...values),
      hi = Math.max(...values);
    return values
      .map(
        (v, i) =>
          `${i ? "L" : "M"}${75 + ((dates[i] - min) / Math.max(max - min, 1)) * 720},${240 - ((v - lo) / Math.max(hi - lo, 0.001)) * 190}`,
      )
      .join(" ");
  }
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
              {[0, 0.25, 0.5, 0.75, 1].map((fraction) => {
                const gLo = Math.min(...rows.map((p) => p.gravity)),
                  gHi = Math.max(...rows.map((p) => p.gravity));
                const tLo = Math.min(...rows.map((p) => p.temperatureF)),
                  tHi = Math.max(...rows.map((p) => p.temperatureF));
                const y = 240 - fraction * 190;
                return (
                  <g key={fraction}>
                    <line x1="75" x2="795" y1={y} y2={y} stroke="#ddd" />
                    <text
                      x="68"
                      y={y + 4}
                      textAnchor="end"
                      fontSize="11"
                      fill="#1971c2"
                    >
                      {u
                        .gravity(gLo + fraction * (gHi - gLo))
                        .toFixed(r.units.gravity === "P" ? 2 : 3)}
                    </text>
                    <text x="805" y={y + 4} fontSize="11" fill="#d9480f">
                      {u.temperature(tLo + fraction * (tHi - tLo)).toFixed(1)}
                    </text>
                  </g>
                );
              })}
              <path
                d={path("gravity")}
                fill="none"
                stroke="#1971c2"
                strokeWidth="2"
              />
              <path
                d={path("temperatureF")}
                fill="none"
                stroke="#d9480f"
                strokeWidth="2"
              />
              {r.events
                .filter(
                  (e) =>
                    Date.parse(e.eventTime) >= min &&
                    Date.parse(e.eventTime) <= max,
                )
                .map((e, i) => (
                  <line
                    key={i}
                    x1={
                      50 +
                      ((Date.parse(e.eventTime) - min) /
                        Math.max(max - min, 1)) *
                        800
                    }
                    x2={
                      50 +
                      ((Date.parse(e.eventTime) - min) /
                        Math.max(max - min, 1)) *
                        800
                    }
                    y1="35"
                    y2="245"
                    stroke="#888"
                    strokeDasharray="3 5"
                  >
                    <title>{e.label}</title>
                  </line>
                ))}
              <text x="50" y="275" fill="currentColor" fontSize="12">
                {new Date(min).toLocaleString()}
              </text>
              <text
                x="850"
                y="275"
                textAnchor="end"
                fill="currentColor"
                fontSize="12"
              >
                {new Date(max).toLocaleString()}
              </text>
            </svg>
            <p className="text-sm">
              <span className="text-blue-700">Gravity ({u.gravityLabel})</span>:{" "}
              {u.formatGravity(Math.min(...rows.map((p) => p.gravity)))} –{" "}
              {u.formatGravity(Math.max(...rows.map((p) => p.gravity)))};{" "}
              <span className="text-orange-700">
                temperature ({u.temperatureLabel})
              </span>
              :{" "}
              {u.formatTemperature(
                Math.min(...rows.map((p) => p.temperatureF)),
              )}{" "}
              –{" "}
              {u.formatTemperature(
                Math.max(...rows.map((p) => p.temperatureF)),
              )}
              . Each line uses its own vertical scale.
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
  const q = useQuery({
    queryKey: ["public-report", token],
    queryFn: () => apiGet<Report>(`/reports/${token}`),
    retry: false,
    staleTime: 0,
    refetchInterval: 30_000,
  });
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
  const client = useQueryClient();
  const [selection, setSelection] = useState<Record<string, boolean>>({});
  const [notes, setNotes] = useState(false);
  const [photos, setPhotos] = useState(false);
  const [busy, setBusy] = useState(false);
  const shares = useQuery({
    queryKey: ["reports", id],
    queryFn: () => apiGet<Share[]>(`/brews/${id}/reports`),
  });
  const selected = (events.data ?? []).filter(
    (e) => selection[e.id] ?? !["note", "tasting_note"].includes(e.eventType),
  );
  async function publish(old?: Share) {
    setBusy(true);
    try {
      const options = {
        eventIds: selected.map((e) => e.id),
        includeNotes: notes,
        includePhotos: photos,
        units,
      };
      const result = old
        ? await apiPut<Share>(`/brews/${id}/reports/${old.id}`, options)
        : await apiPost<Share>(`/brews/${id}/reports`, options);
      await client.invalidateQueries({ queryKey: ["reports", id] });
      toast.success(old ? "Snapshot refreshed" : "Report link created");
      if (!old)
        await navigator.clipboard
          ?.writeText(`${location.origin}/reports/${result.token}`)
          .catch(() => {});
    } catch {
      toast.error("Could not publish report");
    } finally {
      setBusy(false);
    }
  }
  if (brew.isPending || events.isPending || readings.isPending)
    return <p>Loading report preview…</p>;
  if (!brew.data || events.isError || readings.isError)
    return <p>Could not load report preview.</p>;
  const b = brew.data;
  const mapEvent = (e: BrewEventResponse): ReportEvent => ({
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
  const report: Report = {
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
        <label className="block">
          <input
            type="checkbox"
            checked={notes}
            onChange={(e) => setNotes(e.target.checked)}
          />{" "}
          Include batch and entry notes
        </label>
        <label className="block">
          <input
            type="checkbox"
            checked={photos}
            onChange={(e) => setPhotos(e.target.checked)}
          />{" "}
          Include selected entries’ photos
        </label>
        <fieldset>
          <legend>Log entries</legend>
          {events.data?.map((e) => (
            <label key={e.id} className="block">
              <input
                type="checkbox"
                checked={
                  selection[e.id] ??
                  !["note", "tasting_note"].includes(e.eventType)
                }
                onChange={(v) =>
                  setSelection({ ...selection, [e.id]: v.target.checked })
                }
              />{" "}
              {e.label} ({e.eventType.replaceAll("_", " ")})
            </label>
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
                onClick={async () => {
                  setBusy(true);
                  try {
                    await apiDelete(`/brews/${id}/reports/${s.id}`);
                    await shares.refetch();
                  } catch {
                    toast.error("Could not revoke link");
                  } finally {
                    setBusy(false);
                  }
                }}
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
