import type { APIRequestContext, Page } from "@playwright/test"
import { expect, test } from "./fixtures"

/**
 * The security headers (server/src/fairings/security_headers.rs) are only worth
 * anything if the production build actually runs under them. Every E2E test already
 * fails on a CSP violation (see fixtures.ts); this spec additionally walks every main
 * page, exercises the parts most likely to need extra sources (charts, the markdown
 * editor/preview, device setup), and pins the policy's important directives.
 */

const NOTES = [
  "## Brew day",
  "",
  "- **Mash** at 152 °F, _60 min_",
  "- [link](https://example.com)",
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

async function brewId(request: APIRequestContext, name: string): Promise<string> {
  const res = await request.get("/api/v1/brews")
  expect(res.ok()).toBe(true)
  const brew = ((await res.json()) as { id: string; name: string }[]).find((b) => b.name === name)
  expect(brew, `seeded brew ${name}`).toBeTruthy()
  return brew!.id
}

async function settle(page: Page) {
  await page.waitForLoadState("networkidle")
}

function directives(csp: string): Map<string, string> {
  return new Map(
    csp
      .split(";")
      .map((d) => d.trim())
      .filter(Boolean)
      .map((d) => {
        const [name, ...values] = d.split(/\s+/)
        return [name, values.join(" ")] as [string, string]
      }),
  )
}

test.describe("content security policy", () => {
  test("SPA and API responses carry the policy and security headers", async ({ request }) => {
    for (const path of ["/", "/brews", "/api/v1/health"]) {
      const res = await request.get(path)
      const h = res.headers()
      const csp = directives(h["content-security-policy"] ?? "")
      expect(csp.get("default-src"), path).toBe("'self'")
      expect(csp.get("script-src"), path).not.toContain("'unsafe-inline'")
      expect(csp.get("script-src"), path).not.toContain("'unsafe-eval'")
      expect(csp.get("object-src"), path).toBe("'none'")
      expect(csp.get("base-uri"), path).toBe("'self'")
      expect(csp.get("frame-ancestors"), path).toBe("'none'")
      expect(csp.get("form-action"), path).toBe("'self'")
      expect(h["x-content-type-options"], path).toBe("nosniff")
      expect(h["x-frame-options"], path).toBe("DENY")
      expect(h["referrer-policy"], path).toBe("strict-origin-when-cross-origin")
      expect(h["permissions-policy"], path).toContain("serial=(self)")
      // The E2E stack is plain HTTP with no proxy in front: HSTS must not be sent.
      expect(h["strict-transport-security"], path).toBeUndefined()
    }
  })

  test("HSTS is sent when a TLS-terminating proxy says the request was HTTPS", async ({
    request,
  }) => {
    const res = await request.get("/api/v1/health", {
      headers: { "X-Forwarded-Proto": "https" },
    })
    expect(res.headers()["strict-transport-security"]).toContain("max-age=")
  })

  const PAGES: { path: string; heading: RegExp }[] = [
    { path: "/", heading: /dashboard|brews|get started|west coast/i },
    { path: "/brews", heading: /^brews$/i },
    { path: "/brews/new", heading: /new brew/i },
    { path: "/hydrometers", heading: /^hydrometers$/i },
    { path: "/alert-rules", heading: /alert rules/i },
    { path: "/alert-targets", heading: /alert targets/i },
    { path: "/settings/api-keys", heading: /api keys/i },
    { path: "/settings/account", heading: /^account$/i },
    { path: "/privacy", heading: /privacy/i },
    { path: "/terms", heading: /terms/i },
    { path: "/nope/nothing", heading: /404/ },
  ]

  for (const { path, heading } of PAGES) {
    test(`no violations on ${path}`, async ({ page }) => {
      await page.goto(path)
      await expect(page.getByRole("heading", { name: heading }).first()).toBeVisible()
      await settle(page)
    })
  }

  test("no violations on brew detail with chart, markdown notes and the editor", async ({
    page,
    request,
  }) => {
    const id = await brewId(request, "West Coast IPA")
    const put = await request.put(`/api/v1/brews/${id}`, { data: { notes: NOTES } })
    expect(put.ok()).toBe(true)

    await page.goto(`/brews/${id}`)
    await expect(page.getByRole("heading", { name: "West Coast IPA" })).toBeVisible()
    await expect(page.locator("canvas").first()).toBeVisible()
    await expect(page.getByRole("heading", { name: "Brew day" })).toBeVisible()
    await expect(page.getByRole("cell", { name: "1.065" })).toBeVisible()
    await settle(page)

    // The live editor + preview (react-md-editor injects its own styles).
    await page.getByRole("button", { name: "Edit notes" }).click()
    await page.locator("textarea").first().fill(`${NOTES}\n\n> quoted`)
    await expect(page.locator(".wmde-markdown blockquote").first()).toBeVisible()
    await page.getByRole("button", { name: "Cancel" }).or(page.getByLabel("Cancel")).first().click()
    await settle(page)
  })

  test("device setup renders and Web Serial stays allowed", async ({ page }) => {
    await page.goto("/devices/setup")
    await expect(page.getByRole("heading", { name: /device setup|set up/i }).first()).toBeVisible()
    await settle(page)
    const serialAllowed = await page.evaluate(() => {
      const doc = document as Document & {
        featurePolicy?: { allowsFeature(f: string): boolean }
      }
      return doc.featurePolicy ? doc.featurePolicy.allowsFeature("serial") : null
    })
    // null = the browser exposes no policy API; Chromium does, and must allow serial.
    expect(serialAllowed).not.toBe(false)
    expect(await page.evaluate(() => "serial" in navigator)).toBe(true)
  })

  test.describe("blocks injected inline script", () => {
    test.use({ allowCspViolations: true })

    test("inline event handlers in notes do not run", async ({ page, request, cspViolations }) => {
      const id = await brewId(request, "West Coast IPA")
      const payload = `xss probe\n\n<img src="/no-such-image.png" onerror="window.__cspXss = 1">`
      const put = await request.put(`/api/v1/brews/${id}`, { data: { notes: payload } })
      expect(put.ok()).toBe(true)

      await page.goto(`/brews/${id}`)
      await expect(page.getByRole("paragraph").filter({ hasText: "xss probe" }).first()).toBeVisible()
      await settle(page)
      expect(await page.evaluate(() => (window as { __cspXss?: number }).__cspXss)).toBeUndefined()
      // Whatever the markdown renderer lets through, nothing but the probe may be blocked.
      for (const v of cspViolations) expect(v).toMatch(/script-src|inline/i)
    })

    test("an injected inline script is refused", async ({ page, cspViolations }) => {
      await page.goto("/brews")
      await expect(page.getByRole("heading", { name: /^brews$/i })).toBeVisible()
      const ran = await page.evaluate(async () => {
        const s = document.createElement("script")
        s.textContent = "window.__cspInline = 1"
        document.body.appendChild(s)
        await new Promise((r) => setTimeout(r, 100))
        return (window as { __cspInline?: number }).__cspInline
      })
      expect(ran).toBeUndefined()
      await expect.poll(() => cspViolations.some((v) => /script-src/.test(v))).toBe(true)
    })
  })
})
