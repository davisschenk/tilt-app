import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { AlertTriangle, CheckCircle2, Circle, Cpu, Loader2, Usb } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import PageHeader from "@/components/layout/page-header";
import ConfigForm from "@/components/device/config-form";
import { defaultValues, validate, type ConfigFormValues } from "@/lib/device-config";
import SerialConsole from "@/components/device/serial-console";
import { downloadFirmware, useFirmwareManifest } from "@/hooks/use-firmware";
import { apiPost } from "@/lib/api";
import {
  ProvisionClient,
  flashFirmware,
  requestPort,
  serialSupported,
  type ConfigPatch,
  type DeviceConfig,
  type DeviceInfo,
  type WifiNetwork,
} from "@/lib/esp-serial";
import * as toast from "@/lib/toast";

type Step = "connect" | "flash" | "configure" | "verify";
const STEPS: { id: Step; label: string }[] = [
  { id: "connect", label: "Connect" },
  { id: "flash", label: "Flash" },
  { id: "configure", label: "Configure" },
  { id: "verify", label: "Verify" },
];

const MAX_CONSOLE_LINES = 2000;

function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

function Unsupported() {
  return (
    <Card>
      <CardContent className="flex flex-col items-center py-12 text-center">
        <AlertTriangle className="h-10 w-10 text-muted-foreground mb-3" />
        <p className="font-medium">Web Serial isn't available</p>
        <p className="text-sm text-muted-foreground mt-1 max-w-md">
          {window.isSecureContext
            ? "Use a Chromium-based browser (Chrome, Edge, Brave or Opera) to flash and configure a device from this page."
            : "Web Serial needs a secure context. Open this site over HTTPS (or from localhost)."}
        </p>
      </CardContent>
    </Card>
  );
}

function Stepper({ current }: { current: Step }) {
  const idx = STEPS.findIndex((s) => s.id === current);
  return (
    <ol className="flex flex-wrap items-center gap-2 text-sm">
      {STEPS.map((s, i) => (
        <li key={s.id} className="flex items-center gap-2">
          <Badge variant={i === idx ? "default" : i < idx ? "secondary" : "outline"}>
            {i + 1}. {s.label}
          </Badge>
        </li>
      ))}
    </ol>
  );
}

function Check({ done, failed, label }: { done: boolean; failed?: boolean; label: string }) {
  return (
    <li className="flex items-center gap-2 text-sm">
      {done ? (
        <CheckCircle2 className="h-4 w-4 text-green-700 dark:text-green-500" />
      ) : failed ? (
        <AlertTriangle className="h-4 w-4 text-destructive" />
      ) : (
        <Circle className="h-4 w-4 text-muted-foreground" />
      )}
      <span className={done ? "" : "text-muted-foreground"}>{label}</span>
    </li>
  );
}

export default function DeviceSetup() {
  const manifest = useFirmwareManifest();
  const supported = serialSupported();

  const portRef = useRef<SerialPort | null>(null);
  const clientRef = useRef<ProvisionClient | null>(null);
  const unsubRef = useRef<(() => void) | null>(null);

  const [step, setStep] = useState<Step>("connect");
  const [lines, setLines] = useState<string[]>([]);
  const [info, setInfo] = useState<DeviceInfo | null>(null);
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [eraseAll, setEraseAll] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);

  const [reconfigure, setReconfigure] = useState(false);
  const [networks, setNetworks] = useState<WifiNetwork[]>([]);
  const [scanning, setScanning] = useState(false);
  const [values, setValues] = useState<ConfigFormValues | null>(null);

  const pushLine = useCallback((line: string) => {
    setLines((prev) => [...prev, line].slice(-MAX_CONSOLE_LINES));
  }, []);

  const attach = useCallback(
    async (port: SerialPort) => {
      const client = new ProvisionClient(port);
      unsubRef.current?.();
      unsubRef.current = client.onLine(pushLine);
      await client.open();
      clientRef.current = client;
      return client;
    },
    [pushLine],
  );

  const disconnect = useCallback(async () => {
    unsubRef.current?.();
    unsubRef.current = null;
    await clientRef.current?.close();
    clientRef.current = null;
    portRef.current = null;
    setConnected(false);
    setInfo(null);
    setStep("connect");
  }, []);

  useEffect(
    () => () => {
      unsubRef.current?.();
      void clientRef.current?.close();
    },
    [],
  );

  const run = useCallback(async (label: string, fn: () => Promise<void>) => {
    setBusy(label);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setBusy(null);
    }
  }, []);

  const scan = useCallback(async () => {
    const client = clientRef.current;
    if (!client) return;
    setScanning(true);
    try {
      setNetworks(await client.scanWifi());
    } catch (e) {
      toast.error(`WiFi scan failed: ${errMsg(e)}`);
    } finally {
      setScanning(false);
    }
  }, []);

  const enterConfigure = useCallback(
    async (dev: DeviceInfo) => {
      const client = clientRef.current;
      if (!client) return;
      let existing: DeviceConfig | null = null;
      if (dev.provisioned) {
        existing = await client.getConfig().catch(() => null);
      }
      setReconfigure(dev.provisioned);
      setValues(defaultValues(existing, dev.mac));
      setStep("configure");
      if (!dev.provisioned) void scan();
    },
    [scan],
  );

  const handleConnect = () =>
    run("Connecting…", async () => {
      const port = await requestPort().catch((e: unknown) => {
        // The user dismissing the picker is not an error.
        if (e instanceof DOMException && e.name === "NotFoundError") return null;
        throw e;
      });
      if (!port) return;
      portRef.current = port;
      setLines([]);
      await attach(port);
      setConnected(true);
      try {
        // Opening the port usually resets the board; give the firmware time to boot.
        setInfo(await clientRef.current!.waitReady(8000));
      } catch {
        setInfo(null);
      }
    });

  const handleFlash = () =>
    run("Flashing…", async () => {
      const port = portRef.current;
      const m = manifest.data;
      if (!port || !m) return;
      setProgress(0);
      const image = await downloadFirmware(m.files.merged);
      unsubRef.current?.();
      await clientRef.current?.close();
      clientRef.current = null;
      await flashFirmware(port, image, {
        eraseAll,
        onProgress: (w, t) => setProgress(Math.round((w / t) * 100)),
        onLog: (l) => pushLine(l.trim()),
      });
      setProgress(100);
      await attach(port);
      const dev = await clientRef.current!.waitReady(20000);
      setInfo(dev);
      toast.success("Firmware flashed");
      await enterConfigure(dev);
    });

  const handleSubmit = () => {
    if (!values || !info) return;
    const problem = validate(values);
    if (problem) {
      setError(problem);
      return;
    }
    void run("Saving…", async () => {
      const client = clientRef.current;
      if (!client) throw new Error("Not connected");
      const patch: ConfigPatch = {
        wifiSsid: values.wifiSsid.trim(),
        serverUrl: values.serverUrl.trim().replace(/\/+$/, ""),
        deviceName: values.deviceName.trim(),
        scanIntervalSecs: values.scanIntervalSecs,
        minSamples: values.minSamples,
        ewaAlpha: values.ewaAlpha,
      };
      // When reconfiguring, a blank password means "keep the stored one".
      if (!reconfigure || values.wifiPass) patch.wifiPass = values.wifiPass;
      if (values.newApiKey) {
        const created = await apiPost<{ key: string }>("/api-keys", {
          name: values.deviceName.trim() || "esp32",
        });
        patch.apiKey = created.key;
      }
      await client.setConfig(patch);
      setStep("verify");
    });
  };

  const checks = useMemo(() => {
    const text = lines.join("\n");
    return {
      wifi: /WiFi connected: IP=/.test(text),
      wifiFailed: /Initial WiFi connection failed|Failed to connect to WiFi/.test(text),
      time: /Time synced/.test(text),
      upload: /Uploaded \d+ readings/.test(text),
    };
  }, [lines]);

  const m = manifest.data;
  const updateAvailable = !!(info && m && info.version !== m.version);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Device Setup"
        description="Flash and configure an ESP32 Tilt bridge from your browser"
        actions={
          connected && (
            <Button variant="outline" onClick={() => void disconnect()}>
              Disconnect
            </Button>
          )
        }
      />

      {!supported ? (
        <Unsupported />
      ) : (
        <>
          <Stepper current={step} />

          {step === "connect" && (
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Usb className="h-5 w-5" /> Connect your ESP32
                </CardTitle>
                <CardDescription>
                  Plug the board in over USB, then pick its serial port.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                {!connected ? (
                  <Button onClick={() => void handleConnect()} disabled={!!busy}>
                    {busy && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                    {busy ?? "Select serial port"}
                  </Button>
                ) : info ? (
                  <>
                    <div className="rounded-md border p-3 text-sm space-y-1">
                      <div className="flex items-center gap-2">
                        <Cpu className="h-4 w-4" />
                        <span className="font-medium">
                          {info.deviceName || "Tilt ESP32"} · firmware {info.version}
                        </span>
                        {updateAvailable && <Badge>Update {m?.version} available</Badge>}
                      </div>
                      <div className="text-muted-foreground">
                        {info.chip} · {info.mac} ·{" "}
                        {info.provisioned ? "configured" : "not configured yet"}
                      </div>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <Button
                        onClick={() => void run("Loading…", () => enterConfigure(info))}
                        disabled={!!busy}
                      >
                        {info.provisioned ? "Reconfigure" : "Configure"}
                      </Button>
                      <Button variant="outline" onClick={() => setStep("flash")} disabled={!!busy}>
                        {updateAvailable ? "Update firmware" : "Reflash firmware"}
                      </Button>
                    </div>
                  </>
                ) : (
                  <>
                    <p className="text-sm text-muted-foreground">
                      Connected, but no Tilt firmware answered. The board is probably blank or
                      running something else.
                    </p>
                    <Button onClick={() => setStep("flash")}>Continue to flash</Button>
                  </>
                )}
              </CardContent>
            </Card>
          )}

          {step === "flash" && (
            <Card>
              <CardHeader>
                <CardTitle>Flash firmware</CardTitle>
                <CardDescription>
                  Writes the firmware bundled with this server. Your WiFi and server settings are
                  entered in the next step.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                {manifest.isLoading ? (
                  <p className="text-sm text-muted-foreground">Checking available firmware…</p>
                ) : !m ? (
                  <p className="text-sm text-destructive">
                    This server has no firmware installed. Build it with{" "}
                    <code className="font-mono">just esp32-release</code> or use the official
                    Docker image.
                  </p>
                ) : (
                  <>
                    <div className="text-sm">
                      Firmware <span className="font-medium">{m.version}</span> ({m.chip}, built{" "}
                      {new Date(m.builtAt).toLocaleString()})
                      {info && (
                        <span className="text-muted-foreground"> — device has {info.version}</span>
                      )}
                    </div>
                    <div className="flex items-center gap-2">
                      <Switch id="erase" checked={eraseAll} onCheckedChange={setEraseAll} />
                      <Label htmlFor="erase">Erase everything first (also clears saved settings)</Label>
                    </div>
                    {progress !== null && (
                      <div className="space-y-1">
                        <div className="h-2 w-full overflow-hidden rounded bg-muted">
                          <div
                            className="h-full bg-primary transition-all"
                            style={{ width: `${progress}%` }}
                          />
                        </div>
                        <p className="text-xs text-muted-foreground">{progress}%</p>
                      </div>
                    )}
                    <div className="flex gap-2">
                      <Button onClick={() => void handleFlash()} disabled={!!busy}>
                        {busy && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                        {busy ?? "Flash"}
                      </Button>
                      <Button variant="ghost" onClick={() => setStep("connect")} disabled={!!busy}>
                        Back
                      </Button>
                    </div>
                  </>
                )}
              </CardContent>
            </Card>
          )}

          {step === "configure" && values && (
            <Card>
              <CardHeader>
                <CardTitle>{reconfigure ? "Reconfigure device" : "Configure device"}</CardTitle>
                <CardDescription>
                  Settings are saved on the device, which then restarts and connects.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <ConfigForm
                  values={values}
                  onChange={setValues}
                  networks={networks}
                  scanning={scanning}
                  onScan={reconfigure ? undefined : () => void scan()}
                  reconfigure={reconfigure}
                  error={error}
                  submitting={busy === "Saving…"}
                  onSubmit={handleSubmit}
                />
              </CardContent>
            </Card>
          )}

          {step === "verify" && (
            <Card>
              <CardHeader>
                <CardTitle>Verifying</CardTitle>
                <CardDescription>
                  Watching the device boot. A reading is only uploaded once a Tilt is in range.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <ul className="space-y-2">
                  <Check done={checks.wifi} failed={checks.wifiFailed} label="Connected to WiFi" />
                  <Check done={checks.time} label="Clock synced" />
                  <Check done={checks.upload} label="First reading uploaded" />
                </ul>
                {checks.wifiFailed && !checks.wifi && (
                  <p className="text-sm text-destructive">
                    The device couldn't join WiFi. Check the network name and password.
                  </p>
                )}
                <div className="flex flex-wrap gap-2">
                  {checks.wifi && checks.time && (
                    <Button asChild>
                      <Link to="/hydrometers">Done</Link>
                    </Button>
                  )}
                  <Button
                    variant="outline"
                    onClick={() =>
                      info && void run("Loading…", () => enterConfigure({ ...info, provisioned: true }))
                    }
                  >
                    Change settings
                  </Button>
                </div>
              </CardContent>
            </Card>
          )}

          {error && step !== "configure" && <p className="text-sm text-destructive">{error}</p>}

          {connected && <SerialConsole lines={lines} />}
        </>
      )}
    </div>
  );
}
