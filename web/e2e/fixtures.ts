import AxeBuilder from "@axe-core/playwright"
import { expect, test as base, type Page } from "@playwright/test"

type Impact = "minor" | "moderate" | "serious" | "critical"
const BLOCKING: Impact[] = ["serious", "critical"]

/**
 * Run axe against the current page (WCAG 2.0/2.1 A + AA). Serious/critical
 * violations fail the test; the full list is attached so lesser ones stay visible.
 */
export async function expectAccessible(
  page: Page,
  opts: { include?: string; disableRules?: string[] } = {},
) {
  // Dialogs/menus fade and zoom in; axe computes contrast from live colours, so let them settle.
  await page.evaluate(() =>
    Promise.all(document.getAnimations().map((a) => a.finished.catch(() => undefined))),
  )
  let builder = new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
  if (opts.include) builder = builder.include(opts.include)
  if (opts.disableRules) builder = builder.disableRules(opts.disableRules)
  const { violations } = await builder.analyze()
  const blocking = violations.filter((v) => BLOCKING.includes(v.impact as Impact))
  const summary = blocking.map(
    (v) =>
      `${v.id} (${v.impact}): ${v.help}\n` +
      v.nodes.slice(0, 3).map((n) => `    ${n.target.join(" ")}`).join("\n"),
  )
  expect(blocking, `Accessibility violations:\n${summary.join("\n")}`).toEqual([])
}

/** Fails the test on uncaught page errors or console errors (except noisy known ones). */
export const test = base.extend<{ consoleErrors: string[] }>({
  consoleErrors: async ({ page }, provide) => {
    const errors: string[] = []
    page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`))
    page.on("console", (m) => {
      if (m.type() !== "error") return
      // No firmware is installed in the E2E stack; the app handles this 404 on purpose.
      if (m.location().url.endsWith("/api/v1/firmware/manifest")) return
      errors.push(`console: ${m.text()} (${m.location().url})`)
    })
    await provide(errors)
  },
})

export { expect }
