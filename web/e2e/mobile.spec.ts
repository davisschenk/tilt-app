import { expect, test } from "./fixtures"
import type { Locator, Page } from "@playwright/test"

// iPhone 12/13/14-sized viewport: below the md breakpoint, so the brew page uses its tabbed layout.
test.use({ viewport: { width: 390, height: 844 } })

/** Every chart canvas on the page must be laid out with a real size — a 0×0 canvas is an invisible chart. */
async function expectCanvasesSized(page: Page, scope: Locator) {
  const canvas = scope.locator("canvas").first()
  await expect(canvas).toBeVisible()
  // Wait for the ResizeObserver-driven re-layout to settle before measuring.
  await expect.poll(async () => (await canvas.boundingBox())?.height ?? 0).toBeGreaterThan(0)
  const box = (await canvas.boundingBox())!
  expect(box.width).toBeGreaterThan(200)
  expect(box.width).toBeLessThanOrEqual(page.viewportSize()!.width)
  expect(box.height).toBeGreaterThan(200)

  const sizes = await page.locator("canvas").evaluateAll((cs) =>
    cs.map((c) => [c.getBoundingClientRect().width, c.getBoundingClientRect().height]),
  )
  expect(sizes.filter(([w, h]) => w === 0 || h === 0), "zero-size (hidden) chart canvases").toEqual([])
}

test.describe("mobile charts", () => {
  test("brew detail readings chart renders at phone width", async ({ page }) => {
    await page.goto("/brews")
    await page.getByText("West Coast IPA").first().click()
    await expect(page.getByRole("heading", { name: "West Coast IPA" })).toBeVisible()

    await page.getByRole("tab", { name: "Chart" }).click()
    const chartCard = page.getByRole("tabpanel").filter({ hasText: "Readings Chart" })
    await expectCanvasesSized(page, chartCard)
    // Phone layout uses the compact chart height.
    await expect.poll(async () => (await chartCard.locator("canvas").first().boundingBox())?.height).toBe(280)
  })

  test("dashboard recent readings chart renders at phone width", async ({ page }) => {
    await page.goto("/")
    const card = page.locator("[data-slot=card]").filter({ hasText: "Recent Readings (24h)" })
    await expectCanvasesSized(page, card)
  })
})
