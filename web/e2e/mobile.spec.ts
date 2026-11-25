import { expect, test } from "./fixtures"
import type { Locator, Page } from "@playwright/test"

// iPhone 12/13/14-sized viewport: below the md breakpoint, so the brew page uses its tabbed layout.
test.use({ viewport: { width: 390, height: 844 } })

const height = async (canvas: Locator) => (await canvas.boundingBox())?.height ?? 0

/** The chart fits the phone width, and no canvas on the page is 0×0 (an invisible, display:none-mounted chart). */
async function expectChartFits(page: Page, canvas: Locator) {
  const box = (await canvas.boundingBox())!
  expect(box.width).toBeGreaterThan(200)
  expect(box.width).toBeLessThanOrEqual(page.viewportSize()!.width)
  const sizes = await page.locator("canvas").evaluateAll((cs) =>
    cs.map((c) => [c.getBoundingClientRect().width, c.getBoundingClientRect().height]),
  )
  expect(sizes.filter(([w, h]) => w === 0 || h === 0), "zero-size chart canvases").toEqual([])
}

test.describe("mobile charts", () => {
  test("brew detail readings chart renders at phone width", async ({ page }) => {
    await page.goto("/brews")
    await page.getByText("West Coast IPA").first().click()
    await expect(page.getByRole("heading", { name: "West Coast IPA" })).toBeVisible()

    await page.getByRole("tab", { name: "Chart" }).click()
    const canvas = page.getByRole("tabpanel").filter({ hasText: "Readings Chart" }).locator("canvas").first()
    // Phone layout settles on the compact 280px chart height once the container is measured.
    await expect.poll(() => height(canvas)).toBe(280)
    await expectChartFits(page, canvas)
  })

  test("dashboard recent readings chart renders at phone width", async ({ page }) => {
    await page.goto("/")
    const canvas = page.locator("[data-slot=card]").filter({ hasText: "Recent Readings (24h)" }).locator("canvas").first()
    await expect.poll(() => height(canvas)).toBeGreaterThan(200)
    await expectChartFits(page, canvas)
  })
})
