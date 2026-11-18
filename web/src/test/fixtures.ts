import type { BrewResponse, HydrometerResponse, TiltReading } from "@/types"

export const reading = (over: Partial<TiltReading> = {}): TiltReading => ({
  color: "Red",
  temperatureF: 68.5,
  gravity: 1.05,
  rssi: -60,
  recordedAt: new Date().toISOString(),
  ...over,
})

export const hydrometer = (over: Partial<HydrometerResponse> = {}): HydrometerResponse => ({
  id: "h1",
  color: "Red",
  name: "Kitchen Tilt",
  tempOffsetF: 0,
  gravityOffset: 0,
  isDisabled: false,
  createdAt: "2026-01-01T00:00:00Z",
  latestReading: reading(),
  ...over,
})

export const brew = (over: Partial<BrewResponse> = {}): BrewResponse => ({
  id: "b1",
  name: "Test IPA",
  style: "IPA",
  og: 1.06,
  fg: null,
  targetFg: 1.012,
  status: "Active",
  startDate: "2026-01-02T00:00:00Z",
  endDate: null,
  notes: null,
  hydrometerId: "h1",
  createdAt: "2026-01-02T00:00:00Z",
  updatedAt: "2026-01-02T00:00:00Z",
  latestReading: reading(),
  liveAbv: 1.3,
  apparentAttenuation: 20,
  finalAbv: null,
  batchSizeGallons: 5,
  yeastNitrogenRequirement: null,
  pitchTime: null,
  nutrientProtocol: null,
  yeastStrain: null,
  nutrientAlertTargetId: null,
  ...over,
})
