import { expect, test } from "./fixtures"

test.describe("brews", () => {
  test("seeded brews are listed by status", async ({ page }) => {
    await page.goto("/brews")
    await expect(page.getByRole("row", { name: /West Coast IPA/ })).toBeVisible()
    await page.getByRole("tab", { name: "Completed" }).click()
    await expect(page.getByRole("row", { name: /Imperial Stout/ })).toBeVisible()
    await expect(page.getByRole("row", { name: /West Coast IPA/ })).toHaveCount(0)
  })

  test("seeded brew detail shows readings and log events", async ({ page }) => {
    await page.goto("/brews")
    await page.getByRole("row", { name: /West Coast IPA/ }).click()
    await expect(page.getByRole("heading", { name: "West Coast IPA" })).toBeVisible()
    await expect(page.getByText("Current Gravity")).toBeVisible()
    await expect(page.getByText("Dry hop addition")).toBeVisible()
  })

  test("create → edit → delete a brew", async ({ page }) => {
    const name = `E2E Brew ${Date.now()}`

    await page.goto("/brews/new")
    await page.getByRole("button", { name: "Create Brew" }).click()
    await expect(page.getByText("Name is required")).toBeVisible()
    await expect(page.getByText("Hydrometer is required")).toBeVisible()

    await page.getByLabel("Name *").fill(name)
    await page.getByLabel("Style").fill("Test Style")
    await page.getByRole("combobox", { name: /Hydrometer/ }).click()
    await page.getByRole("option", { name: /Green/ }).click()
    await page.getByRole("button", { name: "Create Brew" }).click()

    await expect(page).toHaveURL(/\/brews\/[0-9a-f-]{36}$/)
    await expect(page.getByRole("heading", { name })).toBeVisible()

    // edit
    await page.getByRole("button", { name: "Edit", exact: true }).click()
    const dialog = page.getByRole("dialog", { name: "Edit Brew" })
    await dialog.getByLabel("Name").fill(`${name} (renamed)`)
    await dialog.getByRole("button", { name: /save|update/i }).click()
    await expect(page.getByRole("heading", { name: `${name} (renamed)` })).toBeVisible()

    // delete
    await page.getByRole("button", { name: "Delete", exact: true }).click()
    await page.getByRole("alertdialog").getByRole("button", { name: "Delete" }).click()
    await expect(page).toHaveURL(/\/brews$/)
    await expect(page.getByRole("row", { name: new RegExp(name) })).toHaveCount(0)
  })

  test("notes can be added to a brew", async ({ page }) => {
    await page.goto("/brews")
    await page.getByRole("tab", { name: "Completed" }).click()
    await page.getByRole("row", { name: /Imperial Stout/ }).click()
    await expect(page.getByRole("heading", { name: "Imperial Stout" })).toBeVisible()

    const text = `E2E note ${Date.now()}`
    await page.getByRole("button", { name: "Edit notes" }).or(page.getByText("Click to add notes...")).first().click()
    await page.locator("textarea").first().fill(text)
    await page.getByRole("button", { name: "Save" }).click()
    await expect(page.locator("textarea")).toHaveCount(0) // editor closed → saved
    await expect(page.getByRole("paragraph").filter({ hasText: text })).toBeVisible()

    await page.reload()
    await expect(page.getByRole("paragraph").filter({ hasText: text })).toBeVisible()
  })
})
