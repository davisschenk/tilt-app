import { describe, expect, it } from "vitest"
import { screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { alertRule, alertTarget, brew, hydrometer } from "@/test/fixtures"
import AlertRules from "./alert-rules"

function mockLists(rules = [alertRule()]) {
  server.use(
    http.get("/api/v1/alert-rules", () => HttpResponse.json(rules)),
    http.get("/api/v1/alert-targets", () => HttpResponse.json([alertTarget()])),
    http.get("/api/v1/brews", () => HttpResponse.json([brew()])),
    http.get("/api/v1/hydrometers", () => HttpResponse.json([hydrometer()])),
  )
}

describe("AlertRules page", () => {
  it("shows the empty state", async () => {
    mockLists([])
    renderWithProviders(<AlertRules />)
    expect(await screen.findByText("No alert rules configured")).toBeInTheDocument()
  })

  it("formats conditions for each metric", async () => {
    mockLists([
      alertRule(),
      alertRule({ id: "r2", name: "Hot", metric: "temperature_f", operator: "gt", threshold: 78 }),
      alertRule({ id: "r3", name: "Stuck", metric: "gravity_plateau", operator: "plateau", threshold: 0.002, windowHours: 12, enabled: false, alertTargetId: "gone", lastTriggeredAt: "2026-01-02T00:00:00Z" }),
    ])
    renderWithProviders(<AlertRules />)
    expect(await screen.findByText("Gravity ≤ 1.010")).toBeInTheDocument()
    expect(screen.getByText("Temperature (°F) > 78.0")).toBeInTheDocument()
    expect(screen.getByText("Gravity flat ±0.0020 over 12h")).toBeInTheDocument()
    expect(screen.getByText("Unknown")).toBeInTheDocument()
    expect(screen.getByText("Disabled")).toBeInTheDocument()
    expect(screen.getAllByText("Never")).toHaveLength(2)
    expect(screen.getAllByText("Discord Alerts")).toHaveLength(2)
  })

  it("creates a rule", async () => {
    let body: Record<string, unknown> = {}
    mockLists([])
    server.use(
      http.post("/api/v1/alert-rules", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>
        return HttpResponse.json(alertRule())
      }),
    )
    renderWithProviders(<AlertRules />)
    await userEvent.click((await screen.findAllByRole("button", { name: /Add Rule/ }))[0])
    const dialog = await screen.findByRole("dialog")
    const create = () => userEvent.click(within(dialog).getByRole("button", { name: "Create" }))

    await create()
    expect(within(dialog).getByText("Name is required")).toBeInTheDocument()
    await userEvent.type(within(dialog).getByLabelText("Name *"), "Done")
    await create()
    expect(within(dialog).getByText("Threshold is required")).toBeInTheDocument()
    await userEvent.type(within(dialog).getByLabelText("Threshold *"), "1.010")
    await create()
    expect(within(dialog).getByText("Webhook target is required")).toBeInTheDocument()

    await userEvent.click(within(dialog).getByLabelText("Webhook Target *"))
    await userEvent.click(await screen.findByRole("option", { name: "Discord Alerts" }))
    await create()
    await waitFor(() =>
      expect(body).toMatchObject({
        name: "Done", metric: "gravity", operator: "lte", threshold: 1.01,
        alertTargetId: "t1", brewId: null, hydrometerId: null, cooldownMinutes: 60, enabled: true,
      }),
    )
  })

  it("switches to plateau mode with window field", async () => {
    mockLists([])
    renderWithProviders(<AlertRules />)
    await userEvent.click((await screen.findAllByRole("button", { name: /Add Rule/ }))[0])
    const dialog = await screen.findByRole("dialog")
    await userEvent.click(within(dialog).getByLabelText("Metric *"))
    await userEvent.click(await screen.findByRole("option", { name: "Gravity Plateau" }))
    expect(within(dialog).getByLabelText("Window (hours)")).toBeInTheDocument()
    expect(within(dialog).queryByLabelText("Operator *")).not.toBeInTheDocument()
  })

  it("edits and deletes a rule", async () => {
    let put: unknown
    let deleted = false
    mockLists()
    server.use(
      http.put("/api/v1/alert-rules/r1", async ({ request }) => ((put = await request.json()), HttpResponse.json(alertRule()))),
      http.delete("/api/v1/alert-rules/r1", () => ((deleted = true), new HttpResponse(null, { status: 204 }))),
    )
    renderWithProviders(<AlertRules />)
    await screen.findByText("FG Reached")
    const row = screen.getByRole("row", { name: /FG Reached/ })
    const [editBtn, deleteBtn] = within(row).getAllByRole("button")

    await userEvent.click(editBtn)
    const edit = await screen.findByRole("dialog")
    expect(within(edit).getByLabelText("Name *")).toHaveValue("FG Reached")
    await userEvent.click(within(edit).getByRole("button", { name: /Save|Update/ }))
    await waitFor(() => expect(put).toMatchObject({ name: "FG Reached" }))
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument())

    await userEvent.click(deleteBtn)
    const confirm = await screen.findByRole("alertdialog")
    await userEvent.click(within(confirm).getByRole("button", { name: "Delete" }))
    await waitFor(() => expect(deleted).toBe(true))
  })
})
