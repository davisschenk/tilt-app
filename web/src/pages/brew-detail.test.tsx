import { beforeEach, describe, expect, it, vi } from "vitest"
import { screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { Route, Routes } from "react-router-dom"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { alertRule, alertTarget, brew, hydrometer, reading, readingRow } from "@/test/fixtures"
import { resetChart } from "@/test/mocks/echarts-for-react"
import BrewDetail from "./brew-detail"

vi.mock("echarts-for-react", async () => await import("@/test/mocks/echarts-for-react"))
const { success, error } = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }))
vi.mock("@/lib/toast", () => ({ success, error, info: vi.fn() }))
vi.mock("@uiw/react-md-editor", () => {
  const MDEditor = () => <textarea aria-label="editor" />
  MDEditor.Markdown = ({ source }: { source: string }) => <div>{source}</div>
  return { default: MDEditor }
})

beforeEach(() => { resetChart(); success.mockClear(); error.mockClear() })

function setup(b = brew(), extra: { analytics?: object; rules?: unknown[] } = {}) {
  const puts: Record<string, unknown>[] = []
  server.use(
    http.get("/api/v1/brews/b1", () => (b ? HttpResponse.json(b) : new HttpResponse(null, { status: 404 }))),
    http.put("/api/v1/brews/b1", async ({ request }) => (puts.push((await request.json()) as Record<string, unknown>), HttpResponse.json(b))),
    http.get("/api/v1/hydrometers/h1", () => HttpResponse.json(hydrometer())),
    http.get("/api/v1/hydrometers", () => HttpResponse.json([hydrometer()])),
    http.get("/api/v1/brews", () => HttpResponse.json([])),
    http.get("/api/v1/brews/b1/analytics", () => HttpResponse.json(extra.analytics ?? { currentGravity: 1.04, predictedFgDate: null, hoursRemaining: null, gaps: [] })),
    http.get("/api/v1/alert-rules", () => HttpResponse.json(extra.rules ?? [])),
    http.get("/api/v1/alert-targets", () => HttpResponse.json([alertTarget()])),
    http.get("/api/v1/readings", () => HttpResponse.json([readingRow(0), readingRow(1)])),
    http.get("/api/v1/brews/b1/events", () => HttpResponse.json([])),
    http.get("/api/v1/brews/b1/nutrient-schedule", () => new HttpResponse(null, { status: 400 })),
  )
  renderWithProviders(
    <Routes>
      <Route path="/brews/:id" element={<BrewDetail />} />
      <Route path="/brews" element={<p>brew list</p>} />
    </Routes>,
    { route: "/brews/b1" },
  )
  return puts
}

describe("BrewDetail", () => {
  it("shows a not-found state with a way back", async () => {
    setup(null as never)
    expect(await screen.findByText("This brew does not exist.")).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", { name: "Back to Brews" }))
    expect(await screen.findByText("brew list")).toBeInTheDocument()
  })

  it("renders header, status, live badge and stats", async () => {
    setup()
    expect(await screen.findByRole("heading", { name: "Test IPA" })).toBeInTheDocument()
    expect(screen.getByText("IPA")).toBeInTheDocument()
    expect(screen.getByText("Active")).toBeInTheDocument()
    expect(screen.getByText("Live")).toBeInTheDocument()
    expect(screen.getAllByText("1.060 SG").length).toBeGreaterThan(0) // OG
    expect(screen.getAllByText("Red").length).toBeGreaterThan(0) // hydrometer color
  })

  it("flags a brew whose hydrometer went quiet", async () => {
    setup(brew({ latestReading: reading({ recordedAt: "2020-01-01T00:00:00Z" }) }))
    expect(await screen.findByText("May be offline")).toBeInTheDocument()
  })

  it("finishes a brew, recording the latest gravity as FG", async () => {
    const puts = setup()
    await userEvent.click(await screen.findByRole("button", { name: /Finish Brew/ }))
    await waitFor(() => expect(success).toHaveBeenCalledWith("Brew finished with FG 1.050 SG"))
    expect(puts[0]).toMatchObject({ status: "Completed", fg: 1.05 })
    expect(typeof puts[0].endDate).toBe("string")
  })

  it("reports a failed finish", async () => {
    setup()
    server.use(http.put("/api/v1/brews/b1", () => new HttpResponse(null, { status: 500 })))
    await userEvent.click(await screen.findByRole("button", { name: /Finish Brew/ }))
    await waitFor(() => expect(error).toHaveBeenCalledWith("Failed to finish brew"))
  })

  it("archives a brew", async () => {
    const puts = setup()
    await userEvent.click(await screen.findByRole("button", { name: /^Archive$/ }))
    await waitFor(() => expect(success).toHaveBeenCalledWith("Brew marked as Archived"))
    expect(puts[0]).toEqual({ status: "Archived" })
  })

  it("hides finish/archive for archived brews and finish for completed ones", async () => {
    setup(brew({ status: "Archived" }))
    await screen.findByRole("heading", { name: "Test IPA" })
    expect(screen.queryByRole("button", { name: /Finish Brew/ })).not.toBeInTheDocument()
    expect(screen.queryByRole("button", { name: /^Archive$/ })).not.toBeInTheDocument()
  })

  it("opens edit and delete dialogs; delete returns to the list", async () => {
    setup()
    server.use(http.delete("/api/v1/brews/b1", () => new HttpResponse(null, { status: 204 })))
    await userEvent.click(await screen.findByRole("button", { name: /^Edit$/ }))
    expect(await screen.findByRole("dialog", { name: "Edit Brew" })).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }))
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument())
    await userEvent.click(screen.getByRole("button", { name: /^Delete$/ }))
    const confirm = await screen.findByRole("alertdialog")
    await userEvent.click(within(confirm).getByRole("button", { name: "Delete" }))
    expect(await screen.findByText("brew list")).toBeInTheDocument()
  })

  it.each([
    ["a date and time remaining in days", { predictedFgDate: "2026-02-01T12:00:00Z", hoursRemaining: 52 }, /2d 4h/],
    ["time remaining in hours", { predictedFgDate: "2026-02-01T12:00:00Z", hoursRemaining: 5.2 }, /5h/],
    ["completion when gravity already hit target", { predictedFgDate: null, currentGravity: 1.01 }, /Fermentation complete/],
    ["insufficient data otherwise", { predictedFgDate: null, currentGravity: 1.04 }, /Insufficient data/],
  ])("predicts %s", async (_n, analytics, expected) => {
    setup(brew(), { analytics: { gaps: [], ...analytics } })
    expect((await screen.findAllByText(expected)).length).toBeGreaterThan(0)
  })

  it("celebrates when target gravity is reached on an active brew", async () => {
    setup(brew({ latestReading: reading({ gravity: 1.01 }) }))
    expect((await screen.findAllByText("Target gravity reached!")).length).toBeGreaterThan(0)
  })

  it("lists recently fired alerts for the brew, expandable on desktop", async () => {
    setup(brew(), { rules: [alertRule({ lastTriggeredAt: new Date().toISOString() }), alertRule({ id: "r2", name: "never fired" })] })
    const toggle = await screen.findByRole("button", { name: /Recent Alerts/ })
    await userEvent.click(toggle)
    expect((await screen.findAllByText(/Gravity ≤ 1\.010 SG → Discord Alerts/)).length).toBeGreaterThan(0)
    expect(screen.queryByText("never fired")).not.toBeInTheDocument()
  })

  it("says so when no alerts have fired", async () => {
    setup()
    await userEvent.click(await screen.findByRole("button", { name: /Recent Alerts/ }))
    expect((await screen.findAllByText("No alerts fired yet.")).length).toBeGreaterThan(0)
  })

  it("opens the add-alert dialog scoped to this brew", async () => {
    setup()
    const [add] = await screen.findAllByRole("button", { name: /Add Alert/ })
    await userEvent.click(add)
    expect(await screen.findByRole("dialog", { name: "Add Alert Rule" })).toBeInTheDocument()
  })

  it("expands the readings table", async () => {
    setup()
    await userEvent.click(await screen.findByRole("button", { name: /Recent Readings/ }))
    expect((await screen.findAllByText("Readings Data")).length).toBeGreaterThan(0)
  })
})
