import { expect, test } from "./fixtures"

test.describe("alerts", () => {
  test("seeded target and rule are listed", async ({ page }) => {
    await page.goto("/alert-targets")
    await expect(page.getByText("Local webhook stub")).toBeVisible()
    await page.goto("/alert-rules")
    await expect(page.getByRole("row", { name: /FG threshold reached/ })).toBeVisible()
  })

  test("webhook targets pointing at private addresses are rejected", async ({ page }) => {
    await page.goto("/alert-targets")
    await page.getByRole("button", { name: /add target/i }).first().click()
    const dialog = page.getByRole("dialog", { name: "Add Alert Target" })
    await dialog.getByLabel("Name *").fill("E2E private")
    await dialog.getByLabel("Webhook URL *").fill("http://127.0.0.1:9/hook")
    await dialog.getByRole("button", { name: "Create" }).click()
    await expect(page.getByText("Failed to create alert target")).toBeVisible()
    await expect(dialog).toBeVisible()
  })

  test("create target → create rule using it → delete both", async ({ page }) => {
    const stamp = Date.now()
    const targetName = `E2E Hook ${stamp}`
    const ruleName = `E2E Rule ${stamp}`

    await page.goto("/alert-targets")
    await page.getByRole("button", { name: /add target/i }).first().click()
    const tDialog = page.getByRole("dialog", { name: "Add Alert Target" })
    await tDialog.getByRole("button", { name: "Create" }).click()
    await expect(tDialog.getByText("Name is required")).toBeVisible()
    await tDialog.getByLabel("Name *").fill(targetName)
    // Public literal IP: the SSRF guard rejects loopback/private targets (and nothing is fired here).
    await tDialog.getByLabel("Webhook URL *").fill("https://8.8.8.8/e2e-hook")
    await tDialog.getByRole("button", { name: "Create" }).click()
    const targetCard = page.locator('[data-slot="card"]', { hasText: targetName })
    await expect(targetCard).toBeVisible()

    await page.goto("/alert-rules")
    await page.getByRole("button", { name: /add rule/i }).first().click()
    const rDialog = page.getByRole("dialog", { name: "Add Alert Rule" })
    await rDialog.getByLabel("Name *").fill(ruleName)
    await rDialog.getByLabel("Threshold *").fill("1.010")
    await rDialog.getByRole("combobox", { name: /Webhook Target/ }).click()
    await page.getByRole("option", { name: targetName }).click()
    await rDialog.getByRole("button", { name: "Create" }).click()
    const row = page.getByRole("row", { name: new RegExp(ruleName) })
    await expect(row).toBeVisible()
    await expect(row).toContainText("Gravity ≤ 1.010")
    await expect(row).toContainText(targetName)

    await row.getByRole("button", { name: `Delete rule ${ruleName}` }).click()
    await page.getByRole("alertdialog").getByRole("button", { name: "Delete" }).click()
    await expect(page.getByRole("row", { name: new RegExp(ruleName) })).toHaveCount(0)

    await page.goto("/alert-targets")
    await page.locator('[data-slot="card"]', { hasText: targetName }).getByRole("button", { name: "Delete" }).click()
    await page.getByRole("alertdialog").getByRole("button", { name: "Delete" }).click()
    await expect(page.getByText(targetName)).toHaveCount(0)
  })
})
