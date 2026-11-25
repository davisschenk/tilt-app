import { beforeEach, describe, expect, it, vi } from "vitest"
import { screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { alertTarget, brew } from "@/test/fixtures"
import type { BrewEventResponse, NutrientScheduleResponse } from "@/types"
import NutrientScheduleTable from "./nutrient-schedule-table"
import NutrientSetupPanel from "./nutrient-setup-panel"

const { success, error } = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }))
vi.mock("@/lib/toast", () => ({ success, error, info: vi.fn() }))

const configured = brew({
  batchSizeGallons: 1,
  pitchTime: "2026-01-01T00:00:00Z",
  og: 1.1,
  targetFg: 1.0,
  nutrientProtocol: "tosna_2",
  latestReading: { color: "Red", temperatureF: 68, gravity: 1.08, rssi: null, recordedAt: "2026-01-02T00:00:00Z" },
})

const schedule: NutrientScheduleResponse = {
  protocol: "tosna_2",
  additions: [
    { additionNumber: 1, product: "fermaid_o", amountGrams: 2.6, primaryTrigger: "at_pitch", gravityThreshold: null, fallbackHours: null, dueAt: null },
    { additionNumber: 2, product: "fermaid_o", amountGrams: 2.6, primaryTrigger: "time_elapsed", gravityThreshold: null, fallbackHours: 24, dueAt: "2026-01-02T00:00:00Z" },
    { additionNumber: 3, product: "fermaid_k", amountGrams: 2.8, primaryTrigger: "gravity_threshold", gravityThreshold: 1.09, fallbackHours: 48, dueAt: "2999-01-01T00:00:00Z" },
    { additionNumber: 4, product: "dap", amountGrams: 3.1, primaryTrigger: "gravity_threshold", gravityThreshold: 1.05, fallbackHours: null, dueAt: "2999-01-01T00:00:00Z" },
  ],
  totalYanRequiredPpm: 300,
  nutrientTotals: { fermaid_o: 5.2, "go-ferm": 2 },
  batchSizeGallons: 1,
  batchSizeLiters: 3.785,
  og: 1.1,
  targetFg: 1,
  nitrogenRequirement: "medium",
  pitchTime: "2026-01-01T00:00:00Z",
  resolvedFromStrain: true,
}

const doneEvent = (n: number): BrewEventResponse => ({
  id: `e${n}`, brewId: "b1", eventType: "nutrient_addition", label: `Nutrient Addition #${n}`,
  notes: `Addition #${n}: 2.6g Fermaid-O logged manually`, gravityAtEvent: null, tempAtEvent: null,
  eventTime: "2026-01-01T01:00:00Z", createdAt: "2026-01-01T01:00:00Z", attachments: [],
})

function serve(opts: { schedule?: NutrientScheduleResponse | "error"; events?: BrewEventResponse[] } = {}) {
  server.use(
    http.get("/api/v1/brews/b1/nutrient-schedule", () =>
      opts.schedule === "error" ? new HttpResponse(null, { status: 400 }) : HttpResponse.json(opts.schedule ?? schedule),
    ),
    http.get("/api/v1/brews/b1/events", () => HttpResponse.json(opts.events ?? [])),
    http.get("/api/v1/alert-targets", () => HttpResponse.json([alertTarget()])),
  )
}

beforeEach(() => { success.mockClear(); error.mockClear() })

describe("NutrientScheduleTable", () => {
  it("renders nothing until the brew is fully configured", () => {
    serve()
    const { container } = renderWithProviders(<NutrientScheduleTable brew={brew({ batchSizeGallons: null })} />)
    expect(container).toBeEmptyDOMElement()
  })

  it("shows an error when the schedule cannot be loaded", async () => {
    serve({ schedule: "error" })
    renderWithProviders(<NutrientScheduleTable brew={configured} />)
    expect(await screen.findByText(/Could not load nutrient schedule/)).toBeInTheDocument()
  })

  it("renders additions with triggers, totals and due state", async () => {
    serve({ events: [doneEvent(1)] })
    renderWithProviders(<NutrientScheduleTable brew={configured} />)
    expect(await screen.findByText(/Schedule — TOSNA 2.0/)).toBeInTheDocument()
    expect(screen.getByText("Addition #1")).toBeInTheDocument()
    expect(screen.getByText("At pitch")).toBeInTheDocument()
    expect(screen.getByText("24h elapsed")).toBeInTheDocument()
    expect(screen.getByText("Gravity ≤ 1.090 SG (fallback: 48h)")).toBeInTheDocument()
    expect(screen.getByText("Gravity ≤ 1.050 SG")).toBeInTheDocument()
    expect(screen.getByText("Auto-detected from strain")).toBeInTheDocument()
    expect(screen.getByText("1/4 done")).toBeInTheDocument()
    expect(screen.getByText("Done")).toBeInTheDocument()
    // #2 is past its fallback time, #3's gravity threshold is met → both due now; #4 isn't
    expect(screen.getAllByText("Due now")).toHaveLength(2)
    expect(screen.getAllByText("Fermaid-O").length).toBeGreaterThan(1)
    expect(screen.getAllByText(/tsp total/)).toHaveLength(2)
    expect(screen.getAllByRole("button", { name: "Log" })).toHaveLength(3)
  })

  it("logs an addition as a nutrient event", async () => {
    serve()
    let body: Record<string, unknown> = {}
    server.use(http.post("/api/v1/brews/b1/events", async ({ request }) => {
      body = (await request.json()) as Record<string, unknown>
      return HttpResponse.json(doneEvent(1))
    }))
    renderWithProviders(<NutrientScheduleTable brew={configured} />)
    await userEvent.click((await screen.findAllByRole("button", { name: "Log" }))[0])
    await waitFor(() => expect(body).toMatchObject({
      eventType: "nutrient_addition", label: "Nutrient Addition #1", gravityAtEvent: 1.08,
    }))
    expect(String(body.notes)).toMatch(/^Addition #1: 2\.6g Fermaid-O/)
  })
})

describe("NutrientSetupPanel", () => {
  it("is collapsed by default and shows the configured protocol", () => {
    serve()
    renderWithProviders(<NutrientSetupPanel brew={configured} />)
    expect(screen.getByText(/Configured \(TOSNA 2.0/)).toBeInTheDocument()
    expect(screen.queryByText("Save Settings")).not.toBeInTheDocument()
  })

  it("warns about missing OG/target FG", async () => {
    serve()
    renderWithProviders(<NutrientSetupPanel brew={brew({ og: null, targetFg: null })} />)
    await userEvent.click(screen.getByRole("button", { name: /Nutrient Schedule/ }))
    expect(screen.getByText(/Set OG and Target FG on the brew/)).toBeInTheDocument()
  })

  it("validates batch size before saving", async () => {
    serve()
    renderWithProviders(<NutrientSetupPanel brew={brew({ batchSizeGallons: null })} />)
    await userEvent.click(screen.getByRole("button", { name: /Nutrient Schedule/ }))
    await userEvent.click(screen.getByRole("button", { name: "Save Settings" }))
    expect(error).toHaveBeenCalledWith("Batch size must be a positive number")
  })

  it("saves settings including protocol and alert target", async () => {
    serve()
    let body: Record<string, unknown> = {}
    server.use(http.put("/api/v1/brews/b1", async ({ request }) => ((body = (await request.json()) as Record<string, unknown>), HttpResponse.json(brew()))))
    renderWithProviders(<NutrientSetupPanel brew={configured} />)
    await userEvent.click(screen.getByRole("button", { name: /Nutrient Schedule/ }))
    const batch = screen.getByLabelText("Batch Size (gallons)")
    await userEvent.clear(batch)
    await userEvent.type(batch, "2.5")
    await userEvent.type(screen.getByLabelText(/Yeast Strain/), " D47 ")
    await userEvent.click(screen.getByLabelText("Protocol"))
    await userEvent.click(await screen.findByRole("option", { name: /TOSNA 3.0/ }))
    await userEvent.click(screen.getByLabelText("Webhook Alert Target"))
    await userEvent.click(await screen.findByRole("option", { name: "Discord Alerts" }))
    await userEvent.click(screen.getByRole("button", { name: "Save Settings" }))
    await waitFor(() => expect(success).toHaveBeenCalledWith("Nutrient schedule settings saved"))
    expect(body).toEqual({
      batchSizeGallons: 2.5, yeastNitrogenRequirement: "medium", nutrientProtocol: "tosna_3",
      yeastStrain: "D47", nutrientAlertTargetId: "t1",
    })
  })

  it("reports save failures", async () => {
    serve()
    server.use(http.put("/api/v1/brews/b1", () => new HttpResponse(null, { status: 500 })))
    renderWithProviders(<NutrientSetupPanel brew={configured} />)
    await userEvent.click(screen.getByRole("button", { name: /Nutrient Schedule/ }))
    await userEvent.click(screen.getByRole("button", { name: "Save Settings" }))
    await waitFor(() => expect(error).toHaveBeenCalledWith("Failed to save nutrient settings"))
  })
})
