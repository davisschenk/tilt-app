import type { DeviceConfig } from "@/lib/esp-serial";

export interface ConfigFormValues {
  wifiSsid: string;
  wifiPass: string;
  serverUrl: string;
  deviceName: string;
  scanIntervalSecs: number;
  minSamples: number;
  ewaAlpha: number;
  newApiKey: boolean;
}

export function defaultValues(
  existing: DeviceConfig | null,
  mac: string,
): ConfigFormValues {
  const suffix = mac.replace(/:/g, "").slice(-4);
  return {
    wifiSsid: existing?.wifiSsid ?? "",
    wifiPass: "",
    serverUrl: existing?.serverUrl || window.location.origin,
    deviceName: existing?.deviceName || (suffix ? `tilt-esp32-${suffix}` : "tilt-esp32"),
    scanIntervalSecs: existing?.scanIntervalSecs ?? 15,
    minSamples: existing?.minSamples ?? 3,
    ewaAlpha: existing?.ewaAlpha ?? 30,
    // A fresh device needs a key; a reconfigure keeps the stored one by default.
    newApiKey: !existing,
  };
}

function range(label: string, v: number, lo: number, hi: number): string | null {
  return Number.isInteger(v) && v >= lo && v <= hi
    ? null
    : `${label} must be a whole number between ${lo} and ${hi}`;
}

/** Mirrors the firmware's validate_config ranges. */
export function validate(v: ConfigFormValues): string | null {
  if (!v.wifiSsid.trim()) return "WiFi network name is required";
  if (v.wifiSsid.length > 32) return "WiFi network name is too long (max 32)";
  if (v.wifiPass.length > 64) return "WiFi password is too long (max 64)";
  if (!/^https?:\/\//.test(v.serverUrl)) return "Server URL must start with http:// or https://";
  return (
    range("Scan interval", v.scanIntervalSecs, 5, 300) ??
    range("Minimum samples", v.minSamples, 1, 50) ??
    range("Smoothing", v.ewaAlpha, 1, 100)
  );
}

