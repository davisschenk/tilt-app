import type { MeasurementPreferences } from "@/types";

export type Preferences = MeasurementPreferences;

export const defaultUnits: Preferences = { temperature: "F", gravity: "SG" };

/** What a number measures. Offsets are differences, so they skip the 32° shift and convert relative to SG 1.000. */
export type MeasurementKind =
  | "gravity"
  | "gravityOffset"
  | "temperature"
  | "tempOffset";

export interface Converter {
  /** Canonical (°F / SG) value to the preferred unit. */
  toDisplay: (v: number) => number;
  /** Preferred-unit value back to canonical (°F / SG). */
  toCanonical: (v: number) => number;
  /** Decimal places shown for this kind in the preferred unit. */
  decimals: number;
  /** Unit label, e.g. "SG", "°P", "°F". */
  label: string;
}

export function sgToPlato(sg: number) {
  return -616.868 + 1111.14 * sg - 630.272 * sg ** 2 + 135.997 * sg ** 3;
}

export function platoToSg(p: number) {
  let lo = 0.8;
  let hi = 1.4;
  for (let i = 0; i < 60; i++) {
    const mid = (lo + hi) / 2;
    if (sgToPlato(mid) < p) lo = mid;
    else hi = mid;
  }
  return (lo + hi) / 2;
}

const identity = (v: number) => v;
const PLATO_AT_WATER = sgToPlato(1);

export function measurementConverters(
  units: Preferences,
): Record<MeasurementKind, Converter> {
  const plato = units.gravity === "P";
  const celsius = units.temperature === "C";
  const gravityLabel = plato ? "°P" : "SG";
  const temperatureLabel = `°${units.temperature}`;
  return {
    gravity: {
      toDisplay: plato ? sgToPlato : identity,
      toCanonical: plato ? platoToSg : identity,
      decimals: plato ? 2 : 3,
      label: gravityLabel,
    },
    gravityOffset: {
      toDisplay: plato ? (v) => sgToPlato(1 + v) - PLATO_AT_WATER : identity,
      toCanonical: plato ? (v) => platoToSg(v + PLATO_AT_WATER) - 1 : identity,
      decimals: plato ? 2 : 4,
      label: gravityLabel,
    },
    temperature: {
      toDisplay: celsius ? (v) => ((v - 32) * 5) / 9 : identity,
      toCanonical: celsius ? (v) => (v * 9) / 5 + 32 : identity,
      decimals: 1,
      label: temperatureLabel,
    },
    tempOffset: {
      toDisplay: celsius ? (v) => (v * 5) / 9 : identity,
      toCanonical: celsius ? (v) => (v * 9) / 5 : identity,
      decimals: 1,
      label: temperatureLabel,
    },
  };
}

export function unitTools(units: Preferences) {
  const converters = measurementConverters(units);
  /** A canonical value in the preferred unit, rounded, without the unit suffix. */
  const formatNumber = (kind: MeasurementKind, v: number) => {
    const c = converters[kind];
    return c.toDisplay(v).toFixed(c.decimals);
  };
  /** An already-converted value (e.g. a chart axis tick), rounded for its kind. */
  const formatDisplayNumber = (kind: MeasurementKind, v: number) =>
    v.toFixed(converters[kind].decimals);
  const withUnit =
    (kind: MeasurementKind, separator: string) =>
    (v: number | null | undefined) =>
      v == null
        ? "—"
        : `${formatNumber(kind, v)}${separator}${converters[kind].label}`;
  return {
    units,
    converters,
    gravity: converters.gravity.toDisplay,
    temperature: converters.temperature.toDisplay,
    gravityLabel: converters.gravity.label,
    temperatureLabel: converters.temperature.label,
    formatNumber,
    formatDisplayNumber,
    formatGravity: withUnit("gravity", " "),
    formatGravityDelta: withUnit("gravityOffset", " "),
    formatTemperature: withUnit("temperature", ""),
    formatTemperatureDelta: withUnit("tempOffset", ""),
  };
}

export type UnitTools = ReturnType<typeof unitTools>;
