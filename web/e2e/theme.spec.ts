import { expect, test } from "./fixtures"

test("theme choice applies and survives a reload", async ({ page }) => {
  await page.goto("/")
  const html = page.locator("html")

  await page.getByRole("button", { name: "Toggle theme" }).first().click()
  await page.getByRole("menuitem", { name: "Dark" }).click()
  await expect(html).toHaveClass(/dark/)

  await page.reload()
  await expect(html).toHaveClass(/dark/)

  await page.getByRole("button", { name: "Toggle theme" }).first().click()
  await page.getByRole("menuitem", { name: "Light" }).click()
  await expect(html).not.toHaveClass(/dark/)
})
