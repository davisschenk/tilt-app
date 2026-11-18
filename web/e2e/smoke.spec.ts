import { expect, test } from "./fixtures"

const ROUTES: { path: string; heading: RegExp }[] = [
  { path: "/", heading: /dashboard|brews|get started|west coast/i },
  { path: "/brews", heading: /^brews$/i },
  { path: "/brews/new", heading: /new brew/i },
  { path: "/hydrometers", heading: /^hydrometers$/i },
  { path: "/devices/setup", heading: /device setup|set up/i },
  { path: "/alert-targets", heading: /alert targets/i },
  { path: "/alert-rules", heading: /alert rules/i },
  { path: "/settings/api-keys", heading: /api keys/i },
  { path: "/settings/account", heading: /^account$/i },
]

test.describe("smoke", () => {
  test("health endpoint is up", async ({ request }) => {
    const res = await request.get("/api/v1/health")
    expect(res.ok()).toBe(true)
  })

  for (const { path, heading } of ROUTES) {
    test(`renders ${path} without console errors`, async ({ page, consoleErrors }) => {
      await page.goto(path)
      await expect(page.getByRole("heading", { name: heading }).first()).toBeVisible()
      await page.waitForLoadState("networkidle")
      expect(consoleErrors).toEqual([])
    })
  }

  test("unknown route shows the 404 page", async ({ page }) => {
    await page.goto("/nope/nothing")
    await expect(page.getByRole("heading", { name: "404" })).toBeVisible()
    await page.getByRole("link", { name: "Go to Dashboard" }).click()
    await expect(page).toHaveURL(/\/$/)
  })

  test("public legal pages render", async ({ page }) => {
    await page.goto("/privacy")
    await expect(page.getByRole("heading", { name: /privacy/i }).first()).toBeVisible()
    await page.goto("/terms")
    await expect(page.getByRole("heading", { name: /terms/i }).first()).toBeVisible()
  })

  test("sidebar navigation moves between pages", async ({ page }) => {
    await page.goto("/")
    await page.getByRole("link", { name: "Brews", exact: true }).click()
    await expect(page).toHaveURL(/\/brews$/)
    await page.getByRole("link", { name: "Hydrometers" }).click()
    await expect(page).toHaveURL(/\/hydrometers$/)
  })
})
