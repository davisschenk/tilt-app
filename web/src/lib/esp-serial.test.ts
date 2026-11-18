import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

const loaderState = vi.hoisted(() => ({ failWrite: false, calls: [] as string[] }))
vi.mock("esptool-js", () => ({
  Transport: class {
    async disconnect() { loaderState.calls.push("disconnect") }
  },
  ESPLoader: class {
    chip = { readMac: async () => "aa:bb" }
    async main() { return "ESP32" }
    async writeFlash(o: { reportProgress: (i: number, w: number, t: number) => void; eraseAll: boolean }) {
      if (loaderState.failWrite) throw new Error("write failed")
      o.reportProgress(0, 5, 10)
      loaderState.calls.push(`write erase=${o.eraseAll}`)
    }
    async after(m: string) { loaderState.calls.push(m) }
  },
}))

import { ProvisionClient, flashFirmware, requestPort, serialSupported } from "./esp-serial"

/** Minimal fake SerialPort whose readable we can push lines into and whose writes are recorded. */
function fakePort() {
  let push!: (s: string) => void
  let end!: () => void
  const readable = new ReadableStream<Uint8Array>({
    start(c) {
      push = (s) => c.enqueue(new TextEncoder().encode(s))
      end = () => c.close()
    },
  })
  const writes: string[] = []
  const writable = new WritableStream<Uint8Array>({
    write(chunk) { writes.push(new TextDecoder().decode(chunk)) },
  })
  const port = {
    readable, writable,
    open: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
    setSignals: vi.fn().mockResolvedValue(undefined),
  }
  return { port: port as unknown as SerialPort, raw: port, push, end, writes }
}

const tick = () => new Promise((r) => setTimeout(r, 0))

describe("serial support", () => {
  afterEach(() => { delete (navigator as unknown as Record<string, unknown>).serial })
  it("detects Web Serial", async () => {
    expect(serialSupported()).toBe(false)
    const requestPortMock = vi.fn().mockResolvedValue("port")
    Object.defineProperty(navigator, "serial", { value: { requestPort: requestPortMock }, configurable: true })
    expect(serialSupported()).toBe(true)
    expect(await requestPort()).toBe("port")
  })
})

describe("ProvisionClient", () => {
  beforeEach(() => { loaderState.calls = []; loaderState.failWrite = false })

  it("round-trips a request and notifies line listeners", async () => {
    const f = fakePort()
    const client = new ProvisionClient(f.port)
    const lines: string[] = []
    const off = client.onLine((l) => lines.push(l))
    await client.open()
    const p = client.request({ cmd: "info" })
    await tick()
    expect(f.writes.join("")).toBe('TILT>{"cmd":"info"}\n')
    f.push('boot log\r\nTILT<{"cmd":"info","ok":true,"version":"1.2"}\n')
    await expect(p).resolves.toMatchObject({ version: "1.2" })
    expect(lines).toContain("boot log")
    off()
    await client.close()
  })

  it("rejects when the device reports an error", async () => {
    const f = fakePort()
    const client = new ProvisionClient(f.port)
    await client.open()
    const p = client.request({ cmd: "set_config" })
    await tick()
    f.push('TILT<{"cmd":"set_config","ok":false,"error":"bad ssid"}\n')
    await expect(p).rejects.toThrow("bad ssid")
    await client.close()
  })

  it("treats a generic error reply as the answer and ignores garbled JSON and other commands", async () => {
    const f = fakePort()
    const client = new ProvisionClient(f.port)
    await client.open()
    const p = client.request({ cmd: "info" })
    await tick()
    f.push("TILT<{not json\n" + 'TILT<{"cmd":"other","ok":true}\n' + 'TILT<{"cmd":"error","ok":false}\n')
    await expect(p).rejects.toThrow("Device error")
    await client.close()
  })

  it("times out and rejects concurrent requests", async () => {
    const f = fakePort()
    const client = new ProvisionClient(f.port)
    await client.open()
    const p = client.request({ cmd: "info" }, 20)
    await expect(client.request({ cmd: "x" })).rejects.toThrow(/in flight/)
    await expect(p).rejects.toThrow(/No response to 'info'/)
    await client.close()
  })

  it("fails to open when the port has no readable stream", async () => {
    const f = fakePort()
    ;(f.raw as { readable: unknown }).readable = null
    await expect(new ProvisionClient(f.port).open()).rejects.toThrow(/not readable/)
  })

  it("fails to write when the port is not writable", async () => {
    const f = fakePort()
    const client = new ProvisionClient(f.port)
    await client.open()
    ;(f.raw as { writable: unknown }).writable = null
    await expect(client.request({ cmd: "info" })).rejects.toThrow(/not writable/)
    await client.close()
  })

  it("exposes typed helpers", async () => {
    const f = fakePort()
    const client = new ProvisionClient(f.port)
    await client.open()
    const reply = (cmd: string, extra: object) => { f.push(`TILT<${JSON.stringify({ cmd, ok: true, ...extra })}\n`) }

    const cfg = client.getConfig(); await tick(); reply("get_config", { config: { wifiSsid: "a" } })
    expect(await cfg).toEqual({ wifiSsid: "a" })

    const scan = client.scanWifi(); await tick(); reply("wifi_scan", { networks: [{ ssid: "n", rssi: -40, auth: "wpa2" }] })
    expect(await scan).toHaveLength(1)

    const scanNone = client.scanWifi(); await tick(); reply("wifi_scan", {})
    expect(await scanNone).toEqual([])

    const set = client.setConfig({ wifiSsid: "b" }); await tick(); reply("set_config", {})
    await set
    const reset = client.factoryReset(); await tick(); reply("factory_reset", {})
    await reset

    await client.reset()
    expect(f.raw.setSignals).toHaveBeenNthCalledWith(1, { dataTerminalReady: false, requestToSend: true })
    expect(f.raw.setSignals).toHaveBeenNthCalledWith(2, { dataTerminalReady: false, requestToSend: false })
    await client.close()
  })

  it("waitReady retries until the device answers, then gives up with the last error", async () => {
    const f = fakePort()
    const client = new ProvisionClient(f.port)
    await client.open()
    const spy = vi.spyOn(client, "request")
      .mockRejectedValueOnce(new Error("boot"))
      .mockResolvedValueOnce({ cmd: "info", version: "9" } as never)
    await expect(client.waitReady(5000)).resolves.toMatchObject({ version: "9" })
    spy.mockRejectedValue(new Error("still down"))
    await expect(client.waitReady(50)).rejects.toThrow("still down")
    await client.close()
  })

  it("close rejects any in-flight request and tolerates port errors", async () => {
    const f = fakePort()
    f.raw.close.mockRejectedValue(new Error("gone"))
    const client = new ProvisionClient(f.port)
    await client.open()
    const p = client.request({ cmd: "info" }, 5000)
    const assertion = expect(p).rejects.toThrow("Connection closed")
    await tick()
    await client.close()
    await assertion
  })

  it("stops quietly when the stream ends", async () => {
    const f = fakePort()
    const client = new ProvisionClient(f.port)
    await client.open()
    f.end()
    await tick()
    await client.close()
  })
})

describe("flashFirmware", () => {
  beforeEach(() => { loaderState.calls = []; loaderState.failWrite = false })

  it("writes the image, reports progress and hard-resets", async () => {
    const progress = vi.fn()
    const res = await flashFirmware({} as SerialPort, new Uint8Array([1]), { eraseAll: true, onProgress: progress, onLog: vi.fn() })
    expect(res).toEqual({ chip: "ESP32", mac: "aa:bb" })
    expect(progress).toHaveBeenCalledWith(5, 10)
    expect(loaderState.calls).toEqual(["write erase=true", "hard_reset", "disconnect"])
  })

  it("always disconnects, even when flashing fails", async () => {
    loaderState.failWrite = true
    await expect(
      flashFirmware({} as SerialPort, new Uint8Array(), { eraseAll: false, onProgress: vi.fn(), onLog: vi.fn() }),
    ).rejects.toThrow("write failed")
    expect(loaderState.calls).toContain("disconnect")
  })
})
