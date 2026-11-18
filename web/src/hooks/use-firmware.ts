import { useQuery } from "@tanstack/react-query";
import { API_BASE_URL, ApiError, apiGet } from "@/lib/api";
import type { FirmwareFile, FirmwareManifest } from "@/types";

/** Manifest of the firmware bundled with the server (404 when none is installed). */
export function useFirmwareManifest() {
  return useQuery<FirmwareManifest>({
    queryKey: ["firmware", "manifest"],
    queryFn: () => apiGet<FirmwareManifest>("/firmware/manifest"),
    retry: (count, err) =>
      !(err instanceof ApiError && err.status === 404) && count < 2,
  });
}

function toHex(buf: ArrayBuffer): string {
  return Array.from(new Uint8Array(buf))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/** Download a firmware image and verify it against the manifest's SHA-256. */
export async function downloadFirmware(file: FirmwareFile): Promise<Uint8Array> {
  const res = await fetch(
    `${API_BASE_URL}/firmware/files/${encodeURIComponent(file.name)}`,
    { credentials: "include" },
  );
  if (!res.ok) {
    throw new Error(`Firmware download failed (HTTP ${res.status})`);
  }
  const buf = await res.arrayBuffer();
  const digest = toHex(await crypto.subtle.digest("SHA-256", buf));
  if (digest !== file.sha256.toLowerCase()) {
    throw new Error("Firmware checksum mismatch — refusing to flash");
  }
  return new Uint8Array(buf);
}
