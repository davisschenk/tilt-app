import { useNow } from "@/hooks/use-now";
import { useScanners } from "@/hooks/use-scanners";

export default function ScannerHealth() {
  const now = useNow(30_000);
  const q = useScanners();
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
