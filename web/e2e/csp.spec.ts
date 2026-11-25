import { expect, test } from "./fixtures"

/**
 * The policy (server/src/fairings/security_headers.rs) must work with the production
 * build: every test that opens a page already fails on a CSP violation (fixtures.ts).
 * This spec covers what other specs don't reach: the markdown editor/preview, and
 * that the policy really refuses inline script.
 */

const NOTES = [
  "## Brew day",
  "",
  "| Step | Gravity |",
  "| ---- | ------- |",
  "| OG   | 1.065   |",
  "",
  "```js",
  "const og = 1.065",
  "```",
  "",
  "![pixel](data:image/gif;base64,R0lGODlhAQABAAAAACw=)",
].join("\n")

test.describe("content security policy", () => {
  test("the policy reaches the client on the SPA and the API", async ({ request }) => {
    for (const path of ["/", "/api/v1/health"]) {
      const res = await request.get(path)
      expect(res.headers()["content-security-policy"], path).toContain("default-src 'self'")
    }
  })

  test("brew detail with chart, markdown notes and the editor", async ({ page }) => {
    await page.goto("/brews")
    await page.getByRole("row", { name: /West Coast IPA/ }).click()
    await expect(page.getByRole("heading", { name: "West Coast IPA" })).toBeVisible()
    await expect(page.locator("canvas").first()).toBeVisible()

    await page.getByRole("button", { name: "Edit notes" }).click()
    await page.locator("textarea").first().fill(NOTES)
    await expect(page.locator(".wmde-markdown").getByRole("cell", { name: "1.065" })).toBeVisible()
    await page.getByRole("button", { name: "Save" }).click()
    await expect(page.locator("textarea")).toHaveCount(0)
    await expect(page.getByRole("heading", { name: "Brew day" })).toBeVisible()
  })

  test.describe("refusals", () => {
    test.use({ allowCspViolations: true })

    test("an injected inline script is refused", async ({ page, cspViolations }) => {
      await page.goto("/brews")
      await expect(page.getByRole("heading", { name: /^brews$/i })).toBeVisible()
      await page.evaluate(() => {
        const s = document.createElement("script")
        s.textContent = "window.__cspInline = 1"
        document.body.appendChild(s)
      })
      await expect.poll(() => cspViolations.some((v) => /script-src/.test(v))).toBe(true)
      expect(await page.evaluate(() => (window as { __cspInline?: number }).__cspInline)).toBeUndefined()
    })
  })
})
