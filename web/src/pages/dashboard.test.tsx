import { beforeEach, describe, expect, it, vi } from "vitest"
import { screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { Route, Routes, useLocation } from "react-router-dom"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { alertRule, brew, hydrometer, reading, readingRow } from "@/test/fixtures"
import { resetChart } from "@/test/mocks/echarts-for-react"
import Dashboard from "./dashboard"

vi.mock("echarts-for-react", async () => await import("@/test/mocks/echarts-for-react"))

beforeEach(() => resetChart())

function Where() {
  const l = useLocation()
  return <p>at {l.pathname}{l.search}</p>
}

function setup(opts: { brews?: unknown[]; hydrometers?: unknown[]; rules?: unknown[]; readings?: unknown[] | ((brewScoped: boolean) => unknown[]) } = {}) {
  server.use(
    http.get("/api/v1/brews", () => HttpResponse.json(opts.brews ?? [])),
    http.get("/api/v1/hydrometers", () => HttpResponse.json(opts.hydrometers ?? [hydrometer()])),
    http.get("/api/v1/alert-rules", () => HttpResponse.json(opts.rules ?? [])),
    http.get("/api/v1/api-keys", () => HttpResponse.json([{ id: "k" }])),
    http.get("/api/v1/readings", ({ request }) => {
      const brewScoped = !!new URL(request.url).searchParams.get("brew_id")
      const r = typeof opts.readings === "function" ? opts.readings(brewScoped) : opts.readings
      return HttpResponse.json(r ?? [])
    }),
    http.get("/api/v1/brews/:id/analytics", () => HttpResponse.json({ currentGravity: 1.04, currentTempF: 67.2, liveAbv: 2.6, apparentAttenuation: 33, gaps: [] })),
  )
  return renderWithProviders(
    <Routes>
      <Route path="/" element={<Dashboard />} />
      <Route path="*" element={<Where />} />
    </Routes>,
  )
}

describe("Dashboard", () => {
  it("shows empty states", async () => {
    setup({ hydrometers: [] })
    expect(await screen.findByText("No active brews")).toBeInTheDocument()
    expect(screen.getByText("No readings yet")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /0 active brews/ })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /0 hydrometers/ })).toBeInTheDocument()
  })

  it("summarises live data and links to active brews", async () => {
    setup({
      brews: [brew(), brew({ id: "b2", name: "Older", latestReading: reading({ recordedAt: "2020-01-01T00:00:00Z" }) })],
      readings: [readingRow(0, { gravity: 1.045, temperatureF: 66.4, color: "Blue" })],
    })
    expect(await screen.findByText("1.045 SG / 66.4°F")).toBeInTheDocument()
    expect(screen.getByText("Blue hydrometer")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /2 active brews/ })).toBeInTheDocument()
    await waitFor(() => expect(screen.getAllByText("2.6%")).toHaveLength(2))
    const links = screen.getAllByRole("link", { name: /Test IPA|Older/ })
    // freshest reading first
    expect(links[0]).toHaveTextContent("Test IPA")
    expect(links[0]).toHaveAttribute("href", "/brews/b1")
  })

  it("flags brews that have stopped reporting", async () => {
    setup({ brews: [brew({ latestReading: reading({ recordedAt: "2020-01-01T00:00:00Z" }) })] })
    expect(await screen.findByRole("button", { name: /1 offline/ })).toBeInTheDocument()
    expect(screen.getByTitle(/may be offline/)).toBeInTheDocument()
  })

  it("counts alerts fired in the last 24h", async () => {
    setup({ rules: [alertRule({ lastTriggeredAt: new Date().toISOString() }), alertRule({ id: "r2", lastTriggeredAt: "2020-01-01T00:00:00Z" })] })
    expect(await screen.findByRole("button", { name: /1 alert \(24h\)/ })).toBeInTheDocument()
  })

  it.each([
    ["rising", [1.04, 1.045, 1.05], "lucide-trending-up"],
    ["falling", [1.05, 1.045, 1.04], "lucide-trending-down"],
    ["flat", [1.04, 1.0402, 1.04], "lucide-minus"],
  ])("shows a %s gravity trend", async (_n, gravities, icon) => {
    const { container } = setup({
      brews: [brew()],
      readings: (brewScoped) => (brewScoped ? gravities.map((g, i) => readingRow(i, { gravity: g })) : []),
    })
    await screen.findByText("Test IPA")
    await waitFor(() => expect(container.querySelector(`a svg.${icon}`)).not.toBeNull())
  })

  it("navigates from the chips and the new-brew button", async () => {
    setup({ brews: [brew()] })
    await userEvent.click(await screen.findByRole("button", { name: /1 hydrometer/ }))
    expect(await screen.findByText("at /hydrometers")).toBeInTheDocument()
  })

  it("new brew FAB goes to the form", async () => {
    setup()
    await userEvent.click(await screen.findByRole("button", { name: "New Brew" }))
    expect(await screen.findByText("at /brews/new")).toBeInTheDocument()
  })

  it("refresh refetches data", async () => {
    let hits = 0
    setup()
    server.use(http.get("/api/v1/hydrometers", () => (hits++, HttpResponse.json([hydrometer()]))))
    await screen.findByText("No active brews")
    const before = hits
    await userEvent.click(screen.getByRole("button", { name: "Refresh" }))
    await waitFor(() => expect(hits).toBeGreaterThan(before))
  })
})
