import { beforeEach, describe, expect, it, vi } from "vitest"
import { act, fireEvent, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { readingRow } from "@/test/fixtures"
import { chart, resetChart, series } from "@/test/mocks/echarts-for-react"
import type { BrewEventResponse } from "@/types"
import RecentReadingsChart from "./recent-readings-chart"
import ReadingsChart from "../readings/readings-chart"

vi.mock("echarts-for-react", async () => await import("@/test/mocks/echarts-for-react"))

beforeEach(() => resetChart())

describe("RecentReadingsChart", () => {
  it("shows an empty state", async () => {
    server.use(http.get("/api/v1/readings", () => HttpResponse.json([])))
    renderWithProviders(<RecentReadingsChart />)
    expect(await screen.findByText("No readings in the last 24 hours")).toBeInTheDocument()
  })

  it("draws one series per tilt color, sorted by time", async () => {
    server.use(http.get("/api/v1/readings", () =>
      HttpResponse.json([
        readingRow(5, { color: "Red", gravity: 1.04 }),
        readingRow(1, { color: "Red", gravity: 1.05 }),
        readingRow(2, { color: "Blue", gravity: 1.03 }),
      ])))
    renderWithProviders(<RecentReadingsChart />)
    await screen.findByTestId("echart")
    const all = chart.props!.option.series as { name: string; data: [number, number][] }[]
    expect(all.map((s) => s.name).sort()).toEqual(["Blue", "Red"])
    const red = all.find((s) => s.name === "Red")!
    expect(red.data.map((d) => d[1])).toEqual([1.05, 1.04])
  })
})

const events: BrewEventResponse[] = [
  { id: "e1", brewId: "b1", eventType: "dry_hop", label: "Dry hop", notes: "citra", gravityAtEvent: null, tempAtEvent: null,
    eventTime: readingRow(2).recordedAt, createdAt: "", attachments: [] },
  { id: "e2", brewId: "b1", eventType: "note", label: "Outside", notes: null, gravityAtEvent: null, tempAtEvent: null,
    eventTime: "2020-01-01T00:00:00Z", createdAt: "", attachments: [] },
]

function serveBrew(opts: { readings?: unknown[]; analytics?: object } = {}) {
  const urls: string[] = []
  server.use(
    http.get("/api/v1/readings", ({ request }) => (urls.push(request.url), HttpResponse.json(opts.readings ?? [readingRow(0), readingRow(1), readingRow(2), readingRow(3)]))),
    http.get("/api/v1/brews/b1/events", () => HttpResponse.json(events)),
    http.get("/api/v1/brews/b1/analytics", () => HttpResponse.json(opts.analytics ?? { currentGravity: 1.04, gaps: [{ startAt: readingRow(1).recordedAt, endAt: readingRow(2).recordedAt, durationMinutes: 60 }] })),
  )
  return urls
}

describe("ReadingsChart", () => {
  it("shows an empty state when there are no readings", async () => {
    serveBrew({ readings: [] })
    renderWithProviders(<ReadingsChart brewId="b1" />)
    expect(await screen.findByText("No readings for this time range")).toBeInTheDocument()
  })

  it("builds gravity, temperature and in-range event series with target and predicted lines", async () => {
    serveBrew()
    renderWithProviders(<ReadingsChart brewId="b1" targetFg={1.01} predictedFgDate="2026-01-03T00:00:00Z" />)
    await screen.findByTestId("echart")
    await waitFor(() => expect(series("Events")?.data).toHaveLength(1)) // out-of-range event is dropped
    expect(series("Gravity")?.data).toHaveLength(4)
    expect(series("Temperature")?.data).toHaveLength(4)
    const markNames = series("Gravity")!.markLine!.data.map((d) => d.name)
    expect(markNames).toEqual(expect.arrayContaining(["targetFg", "predictedFg"]))
  })

  it("omits target/predicted lines when not provided", async () => {
    serveBrew()
    renderWithProviders(<ReadingsChart brewId="b1" />)
    await screen.findByTestId("echart")
    const markNames = (series("Gravity")!.markLine?.data ?? []).map((d) => d.name)
    expect(markNames).not.toContain("targetFg")
    expect(markNames).not.toContain("predictedFg")
  })

  it("toggles event markers", async () => {
    serveBrew()
    renderWithProviders(<ReadingsChart brewId="b1" />)
    await waitFor(() => expect(series("Events")?.data).toHaveLength(1))
    await userEvent.click(screen.getByRole("button", { name: /Events/ }))
    await waitFor(() => expect(series("Events")?.data).toHaveLength(0))
  })

  it("refetches with a `since` bound when the range changes", async () => {
    const urls = serveBrew()
    renderWithProviders(<ReadingsChart brewId="b1" />)
    await screen.findByTestId("echart")
    expect(urls.every((u) => !u.includes("since="))).toBe(true)
    await userEvent.click(screen.getByRole("button", { name: "24h" }))
    await waitFor(() => expect(urls.some((u) => u.includes("since="))).toBe(true))
  })

  it("shows an event tooltip on hover and hides it on mouseout", async () => {
    serveBrew()
    renderWithProviders(<ReadingsChart brewId="b1" />)
    await waitFor(() => expect(series("Events")?.data).toHaveLength(1))
    act(() => chart.props!.onEvents!.mouseover({ seriesName: "Events", dataIndex: 0, event: { offsetX: 50, offsetY: 40 } }))
    expect(await screen.findByText("Dry Hop")).toBeInTheDocument()
    expect(screen.getByText("citra")).toBeInTheDocument()
    expect(chart.instance.setOption).toHaveBeenCalledWith({ tooltip: { show: false } }, false)
    act(() => chart.props!.onEvents!.mouseout({ seriesName: "Events" }))
    await waitFor(() => expect(screen.queryByText("citra")).not.toBeInTheDocument())
    // other series are ignored
    act(() => chart.props!.onEvents!.mouseover({ seriesName: "Gravity", event: {} }))
    expect(screen.queryByText("citra")).not.toBeInTheDocument()
  })

  it("opens 'add event at…' from the right-click menu and closes it on click elsewhere", async () => {
    serveBrew()
    renderWithProviders(<ReadingsChart brewId="b1" />)
    const el = await screen.findByTestId("echart")
    const ts = new Date(readingRow(2).recordedAt).getTime()
    chart.instance.convertFromPixel.mockReturnValue([ts, 1])
    fireEvent.contextMenu(el.parentElement!, { clientX: 20, clientY: 10 })
    const item = await screen.findByRole("button", { name: /Add event at/ })
    await userEvent.click(item)
    expect(await screen.findByRole("dialog", { name: "Add Fermentation Event" })).toBeInTheDocument()

    // right-click again, then click anywhere to dismiss
    await userEvent.keyboard("{Escape}")
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument())
    fireEvent.contextMenu(el.parentElement!)
    await screen.findByRole("button", { name: /Add event at/ })
    fireEvent.click(window)
    await waitFor(() => expect(screen.queryByRole("button", { name: /Add event at/ })).not.toBeInTheDocument())
  })

  it("ignores right-clicks that don't map to a time", async () => {
    serveBrew()
    renderWithProviders(<ReadingsChart brewId="b1" />)
    const el = await screen.findByTestId("echart")
    chart.instance.convertFromPixel.mockReturnValue(null)
    fireEvent.contextMenu(el.parentElement!)
    expect(screen.queryByRole("button", { name: /Add event at/ })).not.toBeInTheDocument()
  })
})
