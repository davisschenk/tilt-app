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

import type { AlertRuleResponse, AlertTargetResponse } from "@/types"

export const alertTarget = (over: Partial<AlertTargetResponse> = {}): AlertTargetResponse => ({
  id: "t1",
  name: "Discord Alerts",
  url: "https://discord.com/api/webhooks/123/abc",
  format: "discord",
  secretHeader: null,
  enabled: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  ...over,
})

export const alertRule = (over: Partial<AlertRuleResponse> = {}): AlertRuleResponse => ({
  id: "r1",
  name: "FG Reached",
  brewId: null,
  hydrometerId: null,
  metric: "gravity",
  operator: "lte",
  threshold: 1.01,
  alertTargetId: "t1",
  enabled: true,
  cooldownMinutes: 60,
  windowHours: 24,
  lastTriggeredAt: null,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  ...over,
})

import type { ReadingResponse } from "@/types"

export const readingRow = (i: number, over: Partial<ReadingResponse> = {}): ReadingResponse => ({
  id: `rd${i}`,
  brewId: "b1",
  hydrometerId: "h1",
  color: "Red",
  temperatureF: 68 + i * 0.1,
  gravity: 1.05 - i * 0.001,
  rssi: i % 2 ? -60 : null,
  recordedAt: new Date(Date.UTC(2026, 0, 1, 0, i)).toISOString(),
  createdAt: new Date(Date.UTC(2026, 0, 1, 0, i)).toISOString(),
  ...over,
})
