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

const collected = new WeakMap<Page, { csp: string[]; errors: string[] }>()

/**
 * `page` is wrapped so every test that opens one fails on a Content-Security-Policy
 * violation (Chrome logs each one to the console), so a policy that breaks the
 * production build fails CI. A test that provokes a violation on purpose sets
 * `test.use({ allowCspViolations: true })` and asserts on `cspViolations` itself.
 * `consoleErrors` collects uncaught page errors and console errors (except noisy
 * known ones); tests assert on it explicitly.
 */
export const test = base.extend<{
  consoleErrors: string[]
  cspViolations: string[]
  allowCspViolations: boolean
}>({
  allowCspViolations: [false, { option: true }],
  page: async ({ page, allowCspViolations }, provide) => {
    const c = { csp: [] as string[], errors: [] as string[] }
    collected.set(page, c)
    page.on("pageerror", (e) => c.errors.push(`pageerror: ${e.message}`))
    page.on("console", (m) => {
      if (/Content Security Policy/i.test(m.text())) c.csp.push(m.text())
      if (m.type() !== "error") return
      // No firmware is installed in the E2E stack; the app handles this 404 on purpose.
      if (m.location().url.endsWith("/api/v1/firmware/manifest")) return
      c.errors.push(`console: ${m.text()} (${m.location().url})`)
    })
    await provide(page)
    if (!allowCspViolations) {
      expect(c.csp, `CSP violations:\n${c.csp.join("\n")}`).toEqual([])
    }
  },
  consoleErrors: async ({ page }, provide) => provide(collected.get(page)!.errors),
  cspViolations: async ({ page }, provide) => provide(collected.get(page)!.csp),
})

export { expect }
