let _probe: HTMLSpanElement | null = null;

function probe(): HTMLSpanElement {
  if (!_probe) {
    _probe = document.createElement("span");
    _probe.style.cssText = "position:absolute;width:0;height:0;overflow:hidden;pointer-events:none";
    document.body.appendChild(_probe);
  }
  return _probe;
}

export function resolveColor(variable: string): string {
  if (typeof window === "undefined") return "#888";
  const el = probe();
  el.style.color = `var(${variable})`;
  return getComputedStyle(el).color;
}

export function resolveFont(): string {
  if (typeof window === "undefined") return "system-ui, sans-serif";
  return getComputedStyle(document.body).fontFamily || "system-ui, sans-serif";
}
