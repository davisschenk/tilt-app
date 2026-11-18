import { describe, expect, it, vi } from "vitest"
import { screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { mockNavigation } from "@/test/navigation"
import AccountPage from "./account"

describe("AccountPage", () => {
  it("shows the profile from the session", async () => {
    renderWithProviders(<AccountPage />)
    expect(await screen.findByText("dev@example.com")).toBeInTheDocument()
    expect(screen.getByText("Dev")).toBeInTheDocument()
  })

  it("downloads an export", async () => {
    let hit = false
    server.use(http.get("/api/v1/account/export", () => ((hit = true), HttpResponse.json({}))))
    Object.assign(URL, { createObjectURL: vi.fn(() => "blob:x"), revokeObjectURL: vi.fn() })
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {})
    renderWithProviders(<AccountPage />)
    await userEvent.click(screen.getByRole("button", { name: /Download export/ }))
    await waitFor(() => expect(hit).toBe(true))
    await screen.findByRole("button", { name: /Download export/ })
  })

  it("reports export failure and re-enables the button", async () => {
    server.use(http.get("/api/v1/account/export", () => new HttpResponse(null, { status: 500 })))
    renderWithProviders(<AccountPage />)
    await userEvent.click(screen.getByRole("button", { name: /Download export/ }))
    await waitFor(() => expect(screen.getByRole("button", { name: /Download export/ })).toBeEnabled())
  })

  it("requires typing the email before deleting the account", async () => {
    let body: unknown
    server.use(http.post("/api/v1/account/delete", async ({ request }) => ((body = await request.json()), HttpResponse.json({}))))
    renderWithProviders(<AccountPage />)
    await screen.findByText("dev@example.com")
    const nav = mockNavigation()
    await userEvent.click(screen.getByRole("button", { name: /Delete my account/ }))
    const confirm = await screen.findByRole("button", { name: "Delete everything" })
    expect(confirm).toBeDisabled()
    await userEvent.type(screen.getByLabelText(/to confirm/), "wrong@example.com")
    expect(confirm).toBeDisabled()
    await userEvent.clear(screen.getByLabelText(/to confirm/))
    await userEvent.type(screen.getByLabelText(/to confirm/), " DEV@example.com ")
    expect(confirm).toBeEnabled()
    await userEvent.click(confirm)
    await waitFor(() => expect(nav.assigned).toBe("/"))
    expect(body).toEqual({ confirmEmail: " DEV@example.com " })
  })

  it("keeps the account if deletion fails", async () => {
    server.use(http.post("/api/v1/account/delete", () => new HttpResponse(null, { status: 500 })))
    renderWithProviders(<AccountPage />)
    await screen.findByText("dev@example.com")
    await userEvent.click(screen.getByRole("button", { name: /Delete my account/ }))
    await userEvent.type(await screen.findByLabelText(/to confirm/), "dev@example.com")
    await userEvent.click(screen.getByRole("button", { name: "Delete everything" }))
    await waitFor(() => expect(screen.getByRole("button", { name: "Delete everything" })).toBeEnabled())
  })
})
