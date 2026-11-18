// Web Serial helpers for flashing (esptool-js) and provisioning the Tilt ESP32.
// Protocol: requests are `TILT>{json}\n`, replies are `TILT<{json}\n`; all other
// serial output is plain log text. Mirrors esp32-client/src/protocol.rs.

import { ESPLoader, Transport } from "esptool-js";

export function serialSupported(): boolean {
  return typeof navigator !== "undefined" && "serial" in navigator;
}

export async function requestPort(): Promise<SerialPort> {
  return navigator.serial.requestPort();
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export interface DeviceInfo {
  version: string;
  chip: string;
  mac: string;
  provisioned: boolean;
  deviceName: string;
}

export interface DeviceConfig {
  wifiSsid: string;
  wifiPass: string; // "***" when set
  serverUrl: string;
  apiKey: string; // "***" when set
  deviceName: string;
  scanIntervalSecs: number;
  minSamples: number;
  ewaAlpha: number;
  otaCheckIntervalCycles: number;
}

/** Fields accepted by `set_config`; omitted fields keep their stored value. */
export type ConfigPatch = Partial<Omit<DeviceConfig, "otaCheckIntervalCycles">> & {
  otaCheckIntervalCycles?: number;
};

export interface WifiNetwork {
  ssid: string;
  rssi: number;
  auth: string;
}

interface Reply {
  cmd: string;
  ok?: boolean;
  error?: string;
  [k: string]: unknown;
}

interface Pending {
  cmd: string;
  resolve: (r: Reply) => void;
  reject: (e: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

/** Talks to the firmware's provisioning listener and streams its log output. */
export class ProvisionClient {
  private port: SerialPort;
  private reader: ReadableStreamDefaultReader<Uint8Array> | null = null;
  private loop: Promise<void> | null = null;
  private pending: Pending | null = null;
  private listeners = new Set<(line: string) => void>();

  constructor(port: SerialPort) {
    this.port = port;
  }

  onLine(cb: (line: string) => void): () => void {
    this.listeners.add(cb);
    return () => this.listeners.delete(cb);
  }

  async open(): Promise<void> {
    await this.port.open({ baudRate: 115200 });
    if (!this.port.readable) throw new Error("Serial port is not readable");
    this.reader = this.port.readable.getReader();
    this.loop = this.readLoop(this.reader);
  }

  private async readLoop(reader: ReadableStreamDefaultReader<Uint8Array>) {
    const decoder = new TextDecoder();
    let buf = "";
    try {
      for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });
        let nl: number;
        while ((nl = buf.indexOf("\n")) >= 0) {
          const line = buf.slice(0, nl).replace(/\r$/, "");
          buf = buf.slice(nl + 1);
          this.handleLine(line);
        }
        if (buf.length > 4096) buf = buf.slice(-1024);
      }
    } catch {
      // Port unplugged or closed while reading — end the loop quietly.
    } finally {
      reader.releaseLock();
    }
  }

  private handleLine(line: string) {
    if (!line) return;
    for (const cb of this.listeners) cb(line);
    const idx = line.indexOf("TILT<");
    if (idx < 0 || !this.pending) return;
    let reply: Reply;
    try {
      reply = JSON.parse(line.slice(idx + 5));
    } catch {
      return; // Log text interleaved into the reply — let the request time out/retry.
    }
    const p = this.pending;
    if (reply.cmd === p.cmd || reply.cmd === "error") {
      clearTimeout(p.timer);
      this.pending = null;
      if (reply.ok === false) p.reject(new Error(reply.error ?? "Device error"));
      else p.resolve(reply);
    }
  }

  private async write(text: string) {
    const writable = this.port.writable;
    if (!writable) throw new Error("Serial port is not writable");
    const writer = writable.getWriter();
    try {
      await writer.write(new TextEncoder().encode(text));
    } finally {
      writer.releaseLock();
    }
  }

  async request(
    payload: { cmd: string; [k: string]: unknown },
    timeoutMs = 5000,
  ): Promise<Reply> {
    if (this.pending) throw new Error("A request is already in flight");
    let timerRef: ReturnType<typeof setTimeout> | undefined;
    const reply = new Promise<Reply>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending = null;
        reject(new Error(`No response to '${payload.cmd}' from device`));
      }, timeoutMs);
      timerRef = timer;
      this.pending = { cmd: payload.cmd, resolve, reject, timer };
    });
    try {
      await this.write(`TILT>${JSON.stringify(payload)}\n`);
    } catch (e) {
      clearTimeout(timerRef);
      this.pending = null;
      throw e;
    }
    return reply;
  }

  /** Poll `info` until the firmware answers (it may be mid-reboot). */
  async waitReady(timeoutMs: number): Promise<DeviceInfo> {
    const deadline = Date.now() + timeoutMs;
    let lastErr: Error = new Error("Device did not respond");
    while (Date.now() < deadline) {
      try {
        return (await this.request({ cmd: "info" }, 1200)) as unknown as DeviceInfo;
      } catch (e) {
        lastErr = e as Error;
        await sleep(400);
      }
    }
    throw lastErr;
  }

  async getConfig(): Promise<DeviceConfig> {
    const r = await this.request({ cmd: "get_config" });
    return r.config as DeviceConfig;
  }

  async scanWifi(): Promise<WifiNetwork[]> {
    const r = await this.request({ cmd: "wifi_scan" }, 20000);
    return (r.networks as WifiNetwork[]) ?? [];
  }

  /** Save config; the device reboots right after replying. */
  async setConfig(config: ConfigPatch): Promise<void> {
    await this.request({ cmd: "set_config", config }, 8000);
  }

  async factoryReset(): Promise<void> {
    await this.request({ cmd: "factory_reset" }, 8000);
  }

  /** Pulse EN via RTS (DTR low keeps GPIO0 high so we boot normally). */
  async reset(): Promise<void> {
    await this.port.setSignals({ dataTerminalReady: false, requestToSend: true });
    await sleep(120);
    await this.port.setSignals({ dataTerminalReady: false, requestToSend: false });
  }

  async close(): Promise<void> {
    if (this.pending) {
      clearTimeout(this.pending.timer);
      this.pending.reject(new Error("Connection closed"));
      this.pending = null;
    }
    try {
      await this.reader?.cancel();
    } catch {
      /* already closed */
    }
    await this.loop;
    this.reader = null;
    this.loop = null;
    try {
      await this.port.close();
    } catch {
      /* already closed */
    }
  }
}

export interface FlashOptions {
  eraseAll: boolean;
  onProgress: (written: number, total: number) => void;
  onLog: (line: string) => void;
}

/**
 * Flash a merged image (bootloader + partition table + app) at 0x0, then hard
 * reset. The port must be closed; it is left closed on return.
 */
export async function flashFirmware(
  port: SerialPort,
  image: Uint8Array,
  opts: FlashOptions,
): Promise<{ chip: string; mac: string }> {
  const transport = new Transport(port, false);
  try {
    const loader = new ESPLoader({
      transport,
      baudrate: 460800,
      romBaudrate: 115200,
      terminal: {
        clean: () => {},
        writeLine: (d) => opts.onLog(d),
        write: (d) => opts.onLog(d),
      },
    });
    const chip = await loader.main();
    const mac = await loader.chip.readMac(loader);
    await loader.writeFlash({
      fileArray: [{ data: image, address: 0 }],
      flashSize: "keep",
      flashMode: "keep",
      flashFreq: "keep",
      eraseAll: opts.eraseAll,
      compress: true,
      reportProgress: (_i, written, total) => opts.onProgress(written, total),
    });
    await loader.after("hard_reset");
    return { chip, mac };
  } finally {
    await transport.disconnect().catch(() => {});
  }
}
