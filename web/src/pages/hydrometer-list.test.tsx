import { describe, expect, it } from "vitest"
import { screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { brew, hydrometer, reading } from "@/test/fixtures"
import HydrometerList from "./hydrometer-list"

function mock(hs = [hydrometer()]) {
  server.use(
    http.get("/api/v1/hydrometers", () => HttpResponse.json(hs)),
    http.get("/api/v1/brews", () => HttpResponse.json([brew()])),
  )
}

describe("HydrometerList", () => {
  it("renders hydrometers with live badge and readings", async () => {
    mock()
    renderWithProviders(<HydrometerList />)
    expect(await screen.findByText("Kitchen Tilt")).toBeInTheDocument()
    expect(screen.getByText("Live")).toBeInTheDocument()
    expect(screen.getByText("1.050 SG")).toBeInTheDocument()
  })

  it("shows Offline for stale readings and Disabled for disabled", async () => {
    mock([
      hydrometer({ id: "a", latestReading: reading({ recordedAt: "2020-01-01T00:00:00Z" }) }),
      hydrometer({ id: "b", color: "Blue", name: "B", isDisabled: true }),
    ])
    renderWithProviders(<HydrometerList />)
    expect(await screen.findByText("Offline")).toBeInTheDocument()
    expect(screen.getByText("Disabled")).toBeInTheDocument()
  })

  it("toggles disabled via PUT", async () => {
    mock()
    let body: unknown
    server.use(
      http.put("/api/v1/hydrometers/h1", async ({ request }) => {
        body = await request.json()
        return HttpResponse.json(hydrometer({ isDisabled: true }))
      }),
    )
    renderWithProviders(<HydrometerList />)
    await screen.findByText("Kitchen Tilt")
    const btn = screen.getAllByRole("button").find((b) => /disable/i.test(b.textContent ?? ""))
    expect(btn).toBeTruthy()
    await userEvent.click(btn!)
    await screen.findByText("Kitchen Tilt")
    expect(body).toEqual({ isDisabled: true })
  })
})
