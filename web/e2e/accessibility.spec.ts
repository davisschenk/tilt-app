import { expect, expectAccessible, test } from "./fixtures"

const PAGES = [
  ["dashboard", "/"],
  ["brew list", "/brews"],
  ["new brew", "/brews/new"],
  ["hydrometers", "/hydrometers"],
  ["device setup", "/devices/setup"],
  ["alert targets", "/alert-targets"],
  ["alert rules", "/alert-rules"],
  ["api keys", "/settings/api-keys"],
  ["account", "/settings/account"],
  ["privacy", "/privacy"],
  ["terms", "/terms"],
  ["not found", "/nope"],
] as const

for (const theme of ["light", "dark"] as const) {
  test.describe(`axe — ${theme} theme`, () => {
    test.beforeEach(async ({ page }) => {
      await page.addInitScript((t) => localStorage.setItem("tilt-theme", t), theme)
    })

    for (const [name, path] of PAGES) {
      test(`${name} has no serious violations`, async ({ page }) => {
        await page.goto(path)
        await page.waitForLoadState("networkidle")
        await expectAccessible(page)
      })
    }

    test("brew detail has no serious violations", async ({ page }) => {
      await page.goto("/brews")
      await page.getByRole("row", { name: /West Coast IPA/ }).click()
      await expect(page.getByRole("heading", { name: "West Coast IPA" })).toBeVisible()
      await page.waitForLoadState("networkidle")
      await expectAccessible(page)
    })
  })
}

test.describe("axe — dialogs and menus", () => {
  test("register hydrometer dialog", async ({ page }) => {
    await page.goto("/hydrometers")
    await page.getByRole("button", { name: /register hydrometer/i }).click()
    await expect(page.getByRole("dialog")).toBeVisible()
    await expectAccessible(page)
  })

  test("add alert target dialog", async ({ page }) => {
    await page.goto("/alert-targets")
    await page.getByRole("button", { name: /add target/i }).first().click()
    await expect(page.getByRole("dialog")).toBeVisible()
    await expectAccessible(page)
  })

  test("add alert rule dialog", async ({ page }) => {
    await page.goto("/alert-rules")
    await page.getByRole("button", { name: /add rule/i }).first().click()
    await expect(page.getByRole("dialog")).toBeVisible()
    await expectAccessible(page)
  })

  test("create api key dialog", async ({ page }) => {
    await page.goto("/settings/api-keys")
    await page.getByRole("button", { name: /new api key/i }).click()
    await expect(page.getByRole("dialog")).toBeVisible()
    await expectAccessible(page)
  })

  test("delete account confirmation", async ({ page }) => {
    await page.goto("/settings/account")
    await page.getByRole("button", { name: /delete my account/i }).click()
    await expect(page.getByRole("alertdialog")).toBeVisible()
    await expectAccessible(page)
  })

  test("theme menu", async ({ page }) => {
    await page.goto("/")
    await page.getByRole("button", { name: "Toggle theme" }).first().click()
    await expect(page.getByRole("menu")).toBeVisible()
    // Radix hides the rest of the page with aria-hidden while a modal menu is open; axe flags the
    // sidebar's focusable children even though focus is trapped in the menu. Known false positive.
    await expectAccessible(page, { disableRules: ["aria-hidden-focus"] })
  })
})

test.describe("keyboard and focus", () => {
  test("skip to first interactive element with Tab and open a dialog by keyboard", async ({ page }) => {
    await page.goto("/hydrometers")
    const trigger = page.getByRole("button", { name: /register hydrometer/i })
    await trigger.focus()
    await page.keyboard.press("Enter")
    const dialog = page.getByRole("dialog")
    await expect(dialog).toBeVisible()
    // focus is trapped inside the dialog
    for (let i = 0; i < 8; i++) {
      await page.keyboard.press("Tab")
      expect(await dialog.evaluate((el) => el.contains(document.activeElement))).toBe(true)
    }
    // Escape closes and returns focus to the trigger
    await page.keyboard.press("Escape")
    await expect(dialog).toBeHidden()
    await expect(trigger).toBeFocused()
  })

  test("every form control on the new-brew page has an accessible name", async ({ page }) => {
    await page.goto("/brews/new")
    const unnamed = await page.evaluate(() =>
      [...document.querySelectorAll("input, select, textarea, [role=combobox]")]
        .filter((el) => {
          const e = el as HTMLElement
          const labelled =
            e.getAttribute("aria-label") ||
            e.getAttribute("aria-labelledby") ||
            (e as HTMLInputElement).labels?.length
          return !labelled
        })
        .map((el) => el.outerHTML.slice(0, 80)),
    )
    expect(unnamed).toEqual([])
  })
})
