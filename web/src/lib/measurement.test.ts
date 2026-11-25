import { describe, it, expect } from "vitest";
import {
  sgToPlato,
  platoToSg,
  unitTools,
  measurementConverters,
  type MeasurementKind,
} from "./measurement";

const KINDS: MeasurementKind[] = [
  "gravity",
  "gravityOffset",
  "temperature",
  "tempOffset",
];

describe("measurement units", () => {
  it("round trips gravity across brewing range", () => {
    for (const sg of [0.99, 1, 1.01, 1.05, 1.12, 1.2])
      expect(platoToSg(sgToPlato(sg))).toBeCloseTo(sg, 10);
  });
  it("formats Celsius and Plato without changing canonical values", () => {
    const u = unitTools({ temperature: "C", gravity: "P" });
    expect(u.formatTemperature(68)).toBe("20.0°C");
    expect(u.formatGravity(1.05)).toBe("12.39 °P");
    expect(u.formatGravity(null)).toBe("—");
  });
  it("keeps legacy defaults exact", () => {
    const u = unitTools({ temperature: "F", gravity: "SG" });
    expect(u.formatGravity(1.01)).toBe("1.010 SG");
    expect(u.formatTemperature(32)).toBe("32.0°F");
  });
});

describe("measurement converters", () => {
  it("round trips every kind in every unit", () => {
    const samples: Record<MeasurementKind, number[]> = {
      gravity: [0.995, 1.01, 1.06, 1.12],
      gravityOffset: [-0.003, 0, 0.002, 0.01],
      temperature: [32, 50, 68.5, 212],
      tempOffset: [-1.8, 0, 3.6],
    };
    for (const units of [
      { temperature: "F", gravity: "SG" },
      { temperature: "C", gravity: "P" },
    ] as const) {
      const table = measurementConverters(units);
      for (const kind of KINDS)
        for (const v of samples[kind])
          expect(table[kind].toCanonical(table[kind].toDisplay(v))).toBeCloseTo(v, 8);
    }
  });
  it("converts offsets without the absolute shift", () => {
    const table = measurementConverters({ temperature: "C", gravity: "P" });
    expect(table.tempOffset.toDisplay(1.8)).toBeCloseTo(1);
    expect(table.temperature.toDisplay(32)).toBeCloseTo(0);
    expect(table.gravityOffset.toDisplay(0)).toBeCloseTo(0);
    expect(table.gravityOffset.toDisplay(0.002)).toBeCloseTo(0.52, 2);
  });
  it("uses consistent precision per kind", () => {
    const sg = measurementConverters({ temperature: "F", gravity: "SG" });
    const p = measurementConverters({ temperature: "C", gravity: "P" });
    expect([sg.gravity.decimals, sg.gravityOffset.decimals]).toEqual([3, 4]);
    expect([p.gravity.decimals, p.gravityOffset.decimals]).toEqual([2, 2]);
    expect(sg.temperature.label).toBe("°F");
    expect(p.gravity.label).toBe("°P");
  });
});

describe("delta and axis formatting", () => {
  it("formats gravity and temperature deltas", () => {
    const sg = unitTools({ temperature: "F", gravity: "SG" });
    const p = unitTools({ temperature: "C", gravity: "P" });
    expect(sg.formatGravityDelta(0.002)).toBe("0.0020 SG");
    expect(p.formatGravityDelta(0.002)).toBe("0.52 °P");
    expect(sg.formatTemperatureDelta(-1.8)).toBe("-1.8°F");
    expect(p.formatTemperatureDelta(-1.8)).toBe("-1.0°C");
    expect(p.formatGravityDelta(undefined)).toBe("—");
  });
  it("formats numbers without a unit suffix for chart axes", () => {
    const p = unitTools({ temperature: "C", gravity: "P" });
    expect(p.formatNumber("gravity", 1.05)).toBe("12.39");
    expect(p.formatDisplayNumber("gravity", 12.3912)).toBe("12.39");
    expect(p.formatDisplayNumber("temperature", 20.04)).toBe("20.0");
  });
});
