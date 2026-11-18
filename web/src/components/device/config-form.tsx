import { useState } from "react";
import { Loader2, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import type { WifiNetwork } from "@/lib/esp-serial";
import type { ConfigFormValues } from "@/lib/device-config";

const isLoopback = (url: string) => {
  try {
    const h = new URL(url).hostname;
    return h === "localhost" || h === "127.0.0.1" || h === "::1" || h === "[::1]";
  } catch {
    return false;
  }
};

interface Props {
  values: ConfigFormValues;
  onChange: (v: ConfigFormValues) => void;
  networks: WifiNetwork[];
  scanning: boolean;
  onScan?: () => void;
  reconfigure: boolean;
  error: string | null;
  submitting: boolean;
  onSubmit: () => void;
}

export default function ConfigForm({
  values,
  onChange,
  networks,
  scanning,
  onScan,
  reconfigure,
  error,
  submitting,
  onSubmit,
}: Props) {
  const [showAdvanced, setShowAdvanced] = useState(false);
  const set = <K extends keyof ConfigFormValues>(k: K, val: ConfigFormValues[K]) =>
    onChange({ ...values, [k]: val });

  return (
    <form
      className="space-y-4"
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
    >
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label htmlFor="ssid">WiFi network</Label>
          {onScan && (
            <Button type="button" variant="ghost" size="sm" onClick={onScan} disabled={scanning}>
              {scanning ? (
                <Loader2 className="h-3 w-3 mr-1 animate-spin" />
              ) : (
                <RefreshCw className="h-3 w-3 mr-1" />
              )}
              Scan
            </Button>
          )}
        </div>
        <Input
          id="ssid"
          list="ssid-options"
          value={values.wifiSsid}
          onChange={(e) => set("wifiSsid", e.target.value)}
          placeholder="Network name (2.4 GHz)"
          autoComplete="off"
        />
        <datalist id="ssid-options">
          {networks.map((n) => (
            <option key={n.ssid} value={n.ssid}>
              {`${n.rssi} dBm · ${n.auth}`}
            </option>
          ))}
        </datalist>
        <p className="text-xs text-muted-foreground">
          The ESP32 only supports 2.4 GHz WiFi.
        </p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="wifi-pass">WiFi password</Label>
        <Input
          id="wifi-pass"
          type="password"
          value={values.wifiPass}
          onChange={(e) => set("wifiPass", e.target.value)}
          placeholder={reconfigure ? "Leave blank to keep current" : "Leave blank for open networks"}
          autoComplete="new-password"
        />
      </div>

      <div className="space-y-2">
        <Label htmlFor="server-url">Server URL</Label>
        <Input
          id="server-url"
          value={values.serverUrl}
          onChange={(e) => set("serverUrl", e.target.value)}
          placeholder="http://192.168.1.100:8000"
        />
        {isLoopback(values.serverUrl) && (
          <p className="text-xs text-amber-600 dark:text-amber-400">
            The device can't reach "localhost". Use this server's LAN address instead.
          </p>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="device-name">Device name</Label>
        <Input
          id="device-name"
          value={values.deviceName}
          onChange={(e) => set("deviceName", e.target.value)}
        />
      </div>

      <div className="flex items-center justify-between rounded-md border p-3">
        <div>
          <Label htmlFor="new-key">Create a new API key for this device</Label>
          <p className="text-xs text-muted-foreground">
            Named after the device; manage it under API Keys.
          </p>
        </div>
        <Switch
          id="new-key"
          checked={values.newApiKey}
          onCheckedChange={(c) => set("newApiKey", c)}
        />
      </div>

      <button
        type="button"
        className="text-sm text-muted-foreground underline-offset-4 hover:underline"
        onClick={() => setShowAdvanced((s) => !s)}
      >
        {showAdvanced ? "Hide" : "Show"} advanced settings
      </button>

      {showAdvanced && (
        <div className="grid gap-4 sm:grid-cols-3">
          <div className="space-y-2">
            <Label htmlFor="scan">Scan interval (s)</Label>
            <Input
              id="scan"
              type="number"
              min={5}
              max={300}
              value={values.scanIntervalSecs}
              onChange={(e) => set("scanIntervalSecs", e.target.valueAsNumber)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="samples">Min samples</Label>
            <Input
              id="samples"
              type="number"
              min={1}
              max={50}
              value={values.minSamples}
              onChange={(e) => set("minSamples", e.target.valueAsNumber)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="ewa">Smoothing (%)</Label>
            <Input
              id="ewa"
              type="number"
              min={1}
              max={100}
              value={values.ewaAlpha}
              onChange={(e) => set("ewaAlpha", e.target.valueAsNumber)}
            />
          </div>
        </div>
      )}

      {error && <p className="text-sm text-destructive">{error}</p>}

      <Button type="submit" disabled={submitting}>
        {submitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
        Save &amp; restart device
      </Button>
    </form>
  );
}
