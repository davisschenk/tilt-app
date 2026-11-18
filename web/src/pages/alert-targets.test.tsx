import { beforeEach, describe, expect, it, vi } from "vitest"
import { screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { alertTarget } from "@/test/fixtures"
import AlertTargets from "./alert-targets"

const { toastSuccess, toastError } = vi.hoisted(() => ({ toastSuccess: vi.fn(), toastError: vi.fn() }))
vi.mock("sonner", async (orig) => ({
  ...(await orig<typeof import("sonner")>()),
  toast: { success: toastSuccess, error: toastError },
}))

beforeEach(() => {
  toastSuccess.mockClear()
  toastError.mockClear()
})

describe("AlertTargets page", () => {
  it("shows the empty state", async () => {
    server.use(http.get("/api/v1/alert-targets", () => HttpResponse.json([])))
    renderWithProviders(<AlertTargets />)
    expect(await screen.findByText("No alert targets configured")).toBeInTheDocument()
  })

  it("renders targets with format badge, truncated URL, and secret marker", async () => {
    server.use(
      http.get("/api/v1/alert-targets", () =>
        HttpResponse.json([
          alertTarget({ secretHeader: "Bearer x", url: "https://example.com/" + "a".repeat(80) }),
          alertTarget({ id: "t2", name: "Off", enabled: false, format: "slack" }),
        ]),
      ),
    )
    renderWithProviders(<AlertTargets />)
    expect(await screen.findByText("Discord Alerts")).toBeInTheDocument()
    expect(screen.getByText("Discord")).toBeInTheDocument()
    expect(screen.getByText("Slack")).toBeInTheDocument()
    expect(screen.getByText("Enabled")).toBeInTheDocument()
    expect(screen.getByText("Disabled")).toBeInTheDocument()
    expect(screen.getByText("••••••••")).toBeInTheDocument()
    expect(screen.getByText(/…/)).toBeInTheDocument()
  })

  it("test-fires a target and reports success or failure", async () => {
    server.use(http.get("/api/v1/alert-targets", () => HttpResponse.json([alertTarget()])))
    let result: { ok: boolean; statusCode?: number; error?: string } = { ok: true, statusCode: 204 }
    server.use(http.post("/api/v1/alert-targets/t1/test", () => HttpResponse.json(result)))
    renderWithProviders(<AlertTargets />)
    await userEvent.click(await screen.findByRole("button", { name: /Test/ }))
    await waitFor(() => expect(toastSuccess).toHaveBeenCalledWith(expect.stringContaining("status 204")))

    result = { ok: false, error: "refused" }
    await userEvent.click(screen.getByRole("button", { name: /Test/ }))
    await waitFor(() => expect(toastError).toHaveBeenCalledWith(expect.stringContaining("refused")))

    server.use(http.post("/api/v1/alert-targets/t1/test", () => new HttpResponse(null, { status: 500 })))
    await userEvent.click(screen.getByRole("button", { name: /Test/ }))
    await waitFor(() => expect(toastError).toHaveBeenCalledWith(expect.stringContaining("Failed to send test")))
  })

  it("creates a target with validation", async () => {
    let body: unknown
    server.use(
      http.get("/api/v1/alert-targets", () => HttpResponse.json([])),
      http.post("/api/v1/alert-targets", async ({ request }) => {
        body = await request.json()
        return HttpResponse.json(alertTarget())
      }),
    )
    renderWithProviders(<AlertTargets />)
    await userEvent.click((await screen.findAllByRole("button", { name: /Add Target/ }))[0])
    const dialog = await screen.findByRole("dialog")
    await userEvent.click(within(dialog).getByRole("button", { name: "Create" }))
    expect(within(dialog).getByText("Name is required")).toBeInTheDocument()
    await userEvent.type(within(dialog).getByLabelText("Name *"), "Hook")
    await userEvent.click(within(dialog).getByRole("button", { name: "Create" }))
    expect(within(dialog).getByText("URL is required")).toBeInTheDocument()
    await userEvent.type(within(dialog).getByLabelText("Webhook URL *"), "https://x.test/h")
    await userEvent.click(within(dialog).getByRole("button", { name: "Create" }))
    await waitFor(() => expect(body).toEqual({ name: "Hook", url: "https://x.test/h", format: "generic_json", secretHeader: null, enabled: true }))
    await waitFor(() => expect(toastSuccess).toHaveBeenCalledWith("Alert target created", expect.anything()))
  })

  it("edits a target", async () => {
    let body: unknown
    server.use(
      http.get("/api/v1/alert-targets", () => HttpResponse.json([alertTarget()])),
      http.put("/api/v1/alert-targets/t1", async ({ request }) => {
        body = await request.json()
        return HttpResponse.json(alertTarget({ name: "Renamed" }))
      }),
    )
    renderWithProviders(<AlertTargets />)
    await userEvent.click(await screen.findByRole("button", { name: /Edit/ }))
    const dialog = await screen.findByRole("dialog")
    const name = within(dialog).getByLabelText("Name *")
    expect(name).toHaveValue("Discord Alerts")
    await userEvent.clear(name)
    await userEvent.type(name, "Renamed")
    await userEvent.click(within(dialog).getByRole("button", { name: /Save|Update/ }))
    await waitFor(() => expect(body).toMatchObject({ name: "Renamed", format: "discord", enabled: true }))
  })

  it("deletes a target after confirmation", async () => {
    let deleted = false
    server.use(
      http.get("/api/v1/alert-targets", () => HttpResponse.json([alertTarget()])),
      http.delete("/api/v1/alert-targets/t1", () => ((deleted = true), new HttpResponse(null, { status: 204 }))),
    )
    renderWithProviders(<AlertTargets />)
    await userEvent.click(await screen.findByRole("button", { name: /Delete/ }))
    const dialog = await screen.findByRole("alertdialog")
    expect(within(dialog).getByText(/Delete "Discord Alerts"/)).toBeInTheDocument()
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }))
    await waitFor(() => expect(deleted).toBe(true))
  })
})
