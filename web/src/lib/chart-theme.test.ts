import { expect, it } from "vitest"
import { resolveColor, resolveFont } from "./chart-theme"

it("resolveColor resolves a CSS variable through a probe element", () => {
  document.documentElement.style.setProperty("--foreground", "rgb(1, 2, 3)")
  expect(typeof resolveColor("--foreground")).toBe("string")
  // probe is created once and reused
  resolveColor("--border")
  expect(document.body.querySelectorAll("span[style*='position:absolute']").length).toBeLessThanOrEqual(1)
})

it("resolveFont returns a font stack", () => {
  expect(resolveFont().length).toBeGreaterThan(0)
})
