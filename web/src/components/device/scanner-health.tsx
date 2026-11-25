import { useState, useEffect } from "react";
import { useBrewAnalytics } from "@/hooks/use-brew-analytics";
import { useQuery } from "@tanstack/react-query";
import { apiGet } from "@/lib/api";
interface Scanner {
  id: string;
  deviceId: string;
  firmwareVersion: string;
  lastSeenAt: string;
  uptimeSeconds: number;
  wifiRssi: number | null;
  scanIntervalSeconds: number;
  bufferedReadings: number;
  scanOk: boolean;
  uploadOk: boolean;
}
export default function ScannerHealth() {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(timer);
  }, []);
  const q = useQuery({
    queryKey: ["scanners"],
    queryFn: () => apiGet<Scanner[]>("/scanners"),
    refetchInterval: 30_000,
  });
  return (
    <section className="mb-6 space-y-3">
      <h2 className="text-xl font-semibold">Scanners</h2>
      {q.isPending ? (
        <p>Loading scanner status…</p>
      ) : q.isError ? (
        <p>Could not load scanner status.</p>
      ) : !q.data?.length ? (
        <p className="text-muted-foreground">
          Scanner status unavailable. Update ESP32 firmware to enable
          heartbeats. Hydrometer readings still work with older firmware.
        </p>
      ) : (
        q.data.map((s) => {
          const stale =
            now - Date.parse(s.lastSeenAt) >
            Math.max(s.scanIntervalSeconds * 3, 300) * 1000;
          return (
            <div key={s.id} className="border rounded-lg p-4">
              <p className="font-semibold">
                {s.deviceId} · {stale ? "Stale" : "Online"}
              </p>
              <p>
                Last contact: {new Date(s.lastSeenAt).toLocaleString()} ·
                Firmware {s.firmwareVersion}
              </p>
              <p>
                Wi-Fi:{" "}
                {s.wifiRssi == null ? "Unavailable" : `${s.wifiRssi} dBm`} ·
                Buffered readings: {s.bufferedReadings} · Uptime:{" "}
                {Math.floor(s.uptimeSeconds / 60)} min
              </p>
              <p>
                Last scan: {s.scanOk ? "OK" : "Failed"} · Upload:{" "}
                {s.uploadOk ? "OK" : "Failed or pending"}
              </p>
            </div>
          );
        })
      )}
    </section>
  );
}

export function ReadingGaps({ brewId }: { brewId: string }) {
  const q = useBrewAnalytics(brewId);
  if (!q.data) return null;
  const minutes = q.data.gaps.reduce((sum, g) => sum + g.durationMinutes, 0);
  return (
    <p className="text-sm">
      Reading gaps: {q.data.gaps.length} ({Math.round(minutes)} min total). Gaps
      exceed 45 minutes.
    </p>
  );
}
