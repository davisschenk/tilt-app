import { describe, it, expect } from "vitest";
import { sgToPlato, platoToSg, unitTools } from "./measurement";
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
