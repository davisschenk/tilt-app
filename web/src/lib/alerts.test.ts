import { describe, expect, it } from "vitest";
import { formatCondition } from "./alerts";
import { unitTools } from "./measurement";

const imperial = unitTools({ temperature: "F", gravity: "SG" });
const metric = unitTools({ temperature: "C", gravity: "P" });

describe("formatCondition", () => {
  it("formats threshold rules in the default units", () => {
    expect(
      formatCondition({ metric: "gravity", operator: "lte", threshold: 1.01, windowHours: 24 }, imperial),
    ).toBe("Gravity ≤ 1.010 SG");
    expect(
      formatCondition({ metric: "temperature_f", operator: "gt", threshold: 78, windowHours: 24 }, imperial),
    ).toBe("Temperature (°F) > 78.0°F");
  });

  it("converts thresholds and labels to the viewer's units", () => {
    expect(
      formatCondition({ metric: "temperature_f", operator: "gte", threshold: 68, windowHours: 24 }, metric),
    ).toBe("Temperature (°C) ≥ 20.0°C");
    expect(
      formatCondition({ metric: "gravity", operator: "eq", threshold: 1.05, windowHours: 24 }, metric),
    ).toBe("Gravity = 12.39 °P");
  });

  it("treats plateau thresholds as gravity deltas", () => {
    expect(
      formatCondition({ metric: "gravity_plateau", operator: "plateau", threshold: 0.002, windowHours: 12 }, imperial),
    ).toBe("Gravity flat ±0.0020 over 12h");
    expect(
      formatCondition({ metric: "gravity_plateau", operator: "plateau", threshold: 0.002, windowHours: 12 }, metric),
    ).toBe("Gravity flat ±0.52 over 12h");
  });
});
