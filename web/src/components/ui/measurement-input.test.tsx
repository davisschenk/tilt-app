import { useState } from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { MeasurementInput } from "./measurement-input";
const state = vi.hoisted(() => ({ units: { temperature: "C", gravity: "P" } }));
vi.mock("@/lib/units", async () => ({
  ...(await vi.importActual("@/lib/measurement")),
  useUnits: () => state,
}));
describe("MeasurementInput", () => {
  it("converts temperature and preserves canonical values on unit switch", () => {
    const change = vi.fn();
    const view = render(
      <MeasurementInput
        measurement="temperature"
        value="68"
        onChange={change}
      />,
    );
    expect(screen.getByRole("textbox")).toHaveValue("20");
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "21" } });
    expect(Number(change.mock.lastCall?.[0].target.value)).toBeCloseTo(69.8);
    state.units.temperature = "F";
    view.rerender(
      <MeasurementInput
        measurement="temperature"
        value="68"
        onChange={change}
      />,
    );
    expect(screen.getByRole("textbox")).toHaveValue("68");
    state.units.temperature = "C";
  });
  it("converts negative temperature offsets without adding 32", () => {
    const change = vi.fn();
    render(
      <MeasurementInput
        measurement="tempOffset"
        value="-1.8"
        onChange={change}
      />,
    );
    expect(screen.getByRole("textbox")).toHaveValue("-1");
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "-2" } });
    expect(Number(change.mock.lastCall?.[0].target.value)).toBeCloseTo(-3.6);
  });
  it("retains partial input and converts Plato offsets relative to SG 1.000", () => {
    function Controlled() {
      const [value, setValue] = useState("0.002");
      return (
        <MeasurementInput
          measurement="gravityOffset"
          value={value}
          onChange={(e) => setValue(e.target.value)}
        />
      );
    }
    render(<Controlled />);
    expect(
      Number((screen.getByRole("textbox") as HTMLInputElement).value),
    ).toBeCloseTo(0.52, 2);
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "0." } });
    expect(screen.getByRole("textbox")).toHaveValue("0.");
  });
});
