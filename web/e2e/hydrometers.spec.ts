import { expect, test } from "./fixtures"

test.describe("hydrometers", () => {
  test("lists seeded hydrometers with status", async ({ page }) => {
    await page.goto("/hydrometers")
    await expect(page.getByText("Mash Tun")).toBeVisible()
    await expect(page.getByText("Conical")).toBeVisible()
  })

  test("register → edit → disable → delete", async ({ page }) => {
    const alias = `E2E Pink ${Date.now()}`
    await page.goto("/hydrometers")

    await page.getByRole("button", { name: /register hydrometer/i }).first().click()
    const dialog = page.getByRole("dialog", { name: "Register Hydrometer" })
    await dialog.getByRole("combobox", { name: /Color/ }).click()
    await page.getByRole("option", { name: /Pink/ }).click()
    await dialog.getByLabel("Name / Alias").fill(alias)
    await dialog.getByRole("button", { name: /register|create|save/i }).last().click()

    const card = page.locator('[data-slot="card"]', { hasText: alias })
    await expect(card).toBeVisible()

    await card.getByRole("button", { name: "Disable" }).click()
    await expect(card.getByText("Disabled")).toBeVisible()
    await card.getByRole("button", { name: "Enable" }).click()
    await expect(card.getByRole("button", { name: "Disable" })).toBeVisible()

    await card.getByRole("button", { name: "Edit" }).click()
    const edit = page.getByRole("dialog")
    await edit.getByLabel(/Name/).first().fill(`${alias} v2`)
    await edit.getByRole("button", { name: /save|update/i }).click()
    const renamed = page.locator('[data-slot="card"]', { hasText: `${alias} v2` })
    await expect(renamed).toBeVisible()

    await renamed.getByRole("button", { name: "Delete" }).click()
    await page.getByRole("alertdialog").getByRole("button", { name: "Delete" }).click()
    await expect(page.getByText(`${alias} v2`)).toHaveCount(0)
  })
})
