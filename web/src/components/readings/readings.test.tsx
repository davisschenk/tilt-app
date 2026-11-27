import { describe, expect, it, vi } from "vitest"
import { screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { readingRow } from "@/test/fixtures"
import FermentationStats from "./fermentation-stats"
import ReadingsTable from "./readings-table"

const rows = (n: number) => Array.from({ length: n }, (_, i) => readingRow(i))
const serve = (data: unknown[]) => server.use(http.get("/api/v1/readings", () => HttpResponse.json(data)))

describe("FermentationStats", () => {
  it("renders nothing without readings", async () => {
    serve([])
    const { container } = renderWithProviders(<FermentationStats brewId="b1" og={1.06} />)
    // Wait out the loading skeleton; a fixed sleep raced it under load.
    await waitFor(() => expect(container).toBeEmptyDOMElement())
  })

  it("computes attenuation and ABV from the newest reading", async () => {
    serve([readingRow(0, { gravity: 1.05 }), readingRow(5, { gravity: 1.02 })])
    renderWithProviders(<FermentationStats brewId="b1" og={1.06} targetFg={1.01} />)
    expect(await screen.findByText("1.020 SG")).toBeInTheDocument()
    expect(screen.getByText("66.7%")).toBeInTheDocument() // (1.06-1.02)/0.06
    expect(screen.getByText("5.3%")).toBeInTheDocument() // 0.04*131.25
    expect(screen.getByText("6.6%")).toBeInTheDocument() // 0.05*131.25
  })

  it("shows dashes without OG", async () => {
    serve(rows(2))
    renderWithProviders(<FermentationStats brewId="b1" />)
    await screen.findByText("Current Gravity")
    expect(screen.getAllByText("—")).toHaveLength(3)
  })

  it.each([
    ["rising", [70, 69, 68], "lucide-trending-up"],
    ["falling", [66, 67, 68], "lucide-trending-down"],
    ["steady", [68.1, 68, 68], "lucide-minus"],
  ])("detects a %s temperature trend", async (_n, temps, icon) => {
    // newest first by recordedAt: index 2 is newest
    serve(temps.slice().reverse().map((t, i) => readingRow(i, { temperatureF: t })))
    const { container } = renderWithProviders(<FermentationStats brewId="b1" og={1.06} />)
    await screen.findByText("Temperature Trend")
    expect(container.querySelector(`svg.${icon}`)).not.toBeNull()
  })
})

describe("ReadingsTable", () => {
  it("shows the empty state", async () => {
    serve([])
    renderWithProviders(<ReadingsTable brewId="b1" />)
    expect(await screen.findByText("No readings recorded yet")).toBeInTheDocument()
    expect(screen.queryByRole("button", { name: /Export CSV/ })).not.toBeInTheDocument()
  })

  it("paginates newest-first, 25 per page", async () => {
    serve(rows(60))
    renderWithProviders(<ReadingsTable brewId="b1" />)
    expect(await screen.findByText(/60 readings · Page 1 of 3/)).toBeInTheDocument()
    expect(screen.getAllByRole("row")).toHaveLength(26) // header + 25
    const prev = screen.getByRole("button", { name: /Previous/ })
    const next = screen.getByRole("button", { name: /Next/ })
    expect(prev).toBeDisabled()
    await userEvent.click(next)
    await userEvent.click(next)
    expect(screen.getByText(/Page 3 of 3/)).toBeInTheDocument()
    expect(screen.getAllByRole("row")).toHaveLength(11) // 10 left + header
    expect(next).toBeDisabled()
    await userEvent.click(prev)
    expect(screen.getByText(/Page 2 of 3/)).toBeInTheDocument()
  })

  it("exports CSV", async () => {
    serve(rows(2))
    let blob: Blob | undefined
    Object.assign(URL, {
      createObjectURL: vi.fn((b: Blob) => ((blob = b), "blob:x")),
      revokeObjectURL: vi.fn(),
    })
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {})
    renderWithProviders(<ReadingsTable brewId="b1" exportFilename="mine.csv" />)
    await userEvent.click(await screen.findByRole("button", { name: /Export CSV/ }))
    expect(click).toHaveBeenCalled()
    const text = await blob!.text()
    expect(text.split("\n")[0]).toBe("Recorded At,Temperature (°F),Gravity (SG),RSSI")
    expect(text.split("\n")).toHaveLength(3)
    click.mockRestore()
  })
})
