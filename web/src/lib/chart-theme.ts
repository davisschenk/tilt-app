export function resolveColor(variable: string): string {
  if (typeof window === "undefined") return "#888";
  return getComputedStyle(document.documentElement)
    .getPropertyValue(variable)
    .trim();
}
