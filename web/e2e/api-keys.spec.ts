import { expect, test } from "./fixtures"

test("create an API key, see the secret once, then delete it", async ({ page }) => {
  const name = `E2E key ${Date.now()}`
  await page.goto("/settings/api-keys")

  await page.getByRole("button", { name: /new api key/i }).click()
  const dialog = page.getByRole("dialog", { name: "Create API Key" })
  await expect(dialog.getByRole("button", { name: "Create" })).toBeDisabled()
  await dialog.getByLabel("Name").fill(name)
  await dialog.getByRole("button", { name: "Create" }).click()

  const created = page.getByRole("dialog", { name: "API Key Created" })
  const secret = created.getByRole("textbox")
  await expect(secret).toHaveValue(/.{20,}/)
  await created.getByRole("button", { name: "Done" }).click()

  const card = page.locator('[data-slot="card"]', { hasText: name })
  await expect(card).toBeVisible()

  await card.getByRole("button", { name: `Delete API key ${name}` }).click()
  await page.getByRole("alertdialog").getByRole("button", { name: "Delete" }).click()
  await expect(page.getByText(name)).toHaveCount(0)
})
