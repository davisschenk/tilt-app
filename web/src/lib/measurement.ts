export interface Preferences {
  temperature: "F" | "C";
  gravity: "SG" | "P";
}
export const defaultUnits: Preferences = { temperature: "F", gravity: "SG" };
export function sgToPlato(sg: number) {
  return -616.868 + 1111.14 * sg - 630.272 * sg ** 2 + 135.997 * sg ** 3;
}
export function platoToSg(p: number) {
  let lo = 0.8,
    hi = 1.4;
  for (let i = 0; i < 60; i++) {
    const mid = (lo + hi) / 2;
    if (sgToPlato(mid) < p) lo = mid;
    else hi = mid;
  }
  return (lo + hi) / 2;
}
export function unitTools(units: Preferences) {
  const gravity = (v: number) => (units.gravity === "P" ? sgToPlato(v) : v);
  const temperature = (v: number) =>
    units.temperature === "C" ? ((v - 32) * 5) / 9 : v;
  const gravityLabel = units.gravity === "P" ? "°P" : "SG";
  const temperatureLabel = `°${units.temperature}`;
  const formatGravity = (v: number | null | undefined) =>
    v == null
      ? "—"
      : `${gravity(v).toFixed(units.gravity === "P" ? 2 : 3)} ${gravityLabel}`;
  const formatTemperature = (v: number | null | undefined) =>
    v == null ? "—" : `${temperature(v).toFixed(1)}${temperatureLabel}`;
  return {
    units,
    gravity,
    temperature,
    gravityLabel,
    temperatureLabel,
    formatGravity,
    formatTemperature,
  };
}
