import { describe, expect, it } from "vitest"
import { defaultValues, validate, type ConfigFormValues } from "./device-config"
import type { DeviceConfig } from "./esp-serial"

const existing: DeviceConfig = {
  wifiSsid: "home", wifiPass: "***", serverUrl: "http://srv", apiKey: "***",
  deviceName: "mine", scanIntervalSecs: 20, minSamples: 5, ewaAlpha: 40, otaCheckIntervalCycles: 1,
}

describe("defaultValues", () => {
  it("builds defaults for a fresh device from the MAC suffix", () => {
    const v = defaultValues(null, "AA:BB:CC:DD:EE:FF")
    expect(v).toMatchObject({ deviceName: "tilt-esp32-EEFF", scanIntervalSecs: 15, minSamples: 3, ewaAlpha: 30, newApiKey: true, wifiPass: "" })
    expect(v.serverUrl).toBe(window.location.origin)
  })

  it("falls back to a bare name without a MAC", () => {
    expect(defaultValues(null, "").deviceName).toBe("tilt-esp32")
  })

  it("preserves an existing config and keeps the stored API key", () => {
    const v = defaultValues(existing, "AA:BB")
    expect(v).toMatchObject({ wifiSsid: "home", serverUrl: "http://srv", deviceName: "mine", scanIntervalSecs: 20, newApiKey: false })
  })
})

describe("validate", () => {
  const ok: ConfigFormValues = { ...defaultValues(null, "AA:BB"), wifiSsid: "net", serverUrl: "https://x" }

  it("accepts valid values", () => expect(validate(ok)).toBeNull())

  it.each([
    [{ wifiSsid: "  " }, /name is required/],
    [{ wifiSsid: "x".repeat(33) }, /too long/],
    [{ wifiPass: "x".repeat(65) }, /password is too long/],
    [{ serverUrl: "ftp://x" }, /http/],
    [{ scanIntervalSecs: 4 }, /Scan interval/],
    [{ scanIntervalSecs: 301 }, /Scan interval/],
    [{ minSamples: 0 }, /Minimum samples/],
    [{ minSamples: 1.5 }, /Minimum samples/],
    [{ ewaAlpha: 101 }, /Smoothing/],
  ])("rejects %j", (patch, msg) => {
    expect(validate({ ...ok, ...patch })).toMatch(msg)
  })
})
