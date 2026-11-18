import { describe, expect, it, vi } from "vitest"
import { screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import ApiKeysPage from "./api-keys"

const key = {
  id: "k1",
  name: "Pi Zero",
  prefix: "tilt_ab",
  createdBy: "u1",
  lastUsedAt: "2026-02-01T00:00:00Z",
  expiresAt: "2027-01-01T00:00:00Z",
  createdAt: "2026-01-01T00:00:00Z",
}

describe("ApiKeysPage", () => {
  it("shows empty state", async () => {
    server.use(http.get("/api/v1/api-keys", () => HttpResponse.json([])))
    renderWithProviders(<ApiKeysPage />)
    expect(await screen.findByText("No API keys yet.")).toBeInTheDocument()
  })

  it("lists keys with usage and expiry", async () => {
    server.use(http.get("/api/v1/api-keys", () => HttpResponse.json([key])))
    renderWithProviders(<ApiKeysPage />)
    expect(await screen.findByText("Pi Zero")).toBeInTheDocument()
    expect(screen.getByText(/Last used/)).toBeInTheDocument()
    expect(screen.getByText(/Expires/)).toBeInTheDocument()
  })

  it("creates a key and reveals the secret once", async () => {
    let created = false
    server.use(
      http.get("/api/v1/api-keys", () => HttpResponse.json(created ? [key] : [])),
      http.post("/api/v1/api-keys", async ({ request }) => {
        expect(await request.json()).toEqual({ name: "Pi Zero" })
        created = true
        return HttpResponse.json({ ...key, key: "tilt_secret_value", message: "Copy it now" })
      }),
    )
    renderWithProviders(<ApiKeysPage />)
    await userEvent.click(await screen.findByRole("button", { name: /New API Key/ }))
    const create = screen.getByRole("button", { name: "Create" })
    expect(create).toBeDisabled()
    await userEvent.type(screen.getByLabelText("Name"), "Pi Zero")
    await userEvent.click(create)
    expect(await screen.findByDisplayValue("tilt_secret_value")).toBeInTheDocument()
    expect(screen.getByText("Copy it now")).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", { name: "Done" }))
    await waitFor(() => expect(screen.queryByDisplayValue("tilt_secret_value")).not.toBeInTheDocument())
    expect(await screen.findByText("Pi Zero")).toBeInTheDocument()
  })

  it("copies the key to the clipboard", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    server.use(
      http.get("/api/v1/api-keys", () => HttpResponse.json([])),
      http.post("/api/v1/api-keys", () => HttpResponse.json({ ...key, key: "abc", message: "m" })),
    )
    renderWithProviders(<ApiKeysPage />)
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true })
    await userEvent.click(await screen.findByRole("button", { name: /New API Key/ }))
    const input = screen.getByLabelText("Name")
    await userEvent.type(input, "x{Enter}")
    await screen.findByDisplayValue("abc")
    const buttons = screen.getAllByRole("button")
    const copy = buttons.find((b) => b.className.includes("size-9") || b.querySelector("svg.lucide-copy"))
    await userEvent.click(copy!)
    expect(writeText).toHaveBeenCalledWith("abc")
  })

  it("deletes after confirmation", async () => {
    let deleted = false
    server.use(
      http.get("/api/v1/api-keys", () => HttpResponse.json([key])),
      http.delete("/api/v1/api-keys/k1", () => ((deleted = true), new HttpResponse(null, { status: 204 }))),
    )
    renderWithProviders(<ApiKeysPage />)
    await screen.findByText("Pi Zero")
    const trash = screen.getAllByRole("button").find((b) => b.querySelector("svg.lucide-trash-2"))
    await userEvent.click(trash!)
    await userEvent.click(await screen.findByRole("button", { name: "Delete" }))
    await waitFor(() => expect(deleted).toBe(true))
  })
})
