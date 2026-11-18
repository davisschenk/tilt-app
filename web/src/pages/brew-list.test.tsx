import { describe, expect, it } from "vitest"
import { screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { Route, Routes } from "react-router-dom"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { brew, hydrometer } from "@/test/fixtures"
import BrewList from "./brew-list"

describe("BrewList", () => {
  it("lists active brews and navigates on row click", async () => {
    server.use(
      http.get("/api/v1/brews", () => HttpResponse.json([brew()])),
      http.get("/api/v1/hydrometers", () => HttpResponse.json([hydrometer()])),
    )
    renderWithProviders(
      <Routes>
        <Route path="/" element={<BrewList />} />
        <Route path="/brews/:id" element={<p>detail page</p>} />
      </Routes>,
    )
    const row = await screen.findByText("Test IPA")
    expect(screen.getByText("1.060")).toBeInTheDocument()
    expect(screen.getByText("1.3%")).toBeInTheDocument()
    await userEvent.click(row)
    expect(await screen.findByText("detail page")).toBeInTheDocument()
  })

  it("shows empty state", async () => {
    server.use(
      http.get("/api/v1/brews", () => HttpResponse.json([])),
      http.get("/api/v1/hydrometers", () => HttpResponse.json([])),
    )
    renderWithProviders(<BrewList />)
    expect(await screen.findByText("No brews found")).toBeInTheDocument()
  })

  it("requests the selected status tab", async () => {
    const statuses: string[] = []
    server.use(
      http.get("/api/v1/brews", ({ request }) => {
        statuses.push(new URL(request.url).searchParams.get("status") ?? "")
        return HttpResponse.json([])
      }),
      http.get("/api/v1/hydrometers", () => HttpResponse.json([])),
    )
    renderWithProviders(<BrewList />)
    await userEvent.click(await screen.findByRole("tab", { name: "Completed" }))
    await screen.findByText("No brews found")
    expect(statuses).toContain("Active")
    expect(statuses).toContain("Completed")
  })
})
