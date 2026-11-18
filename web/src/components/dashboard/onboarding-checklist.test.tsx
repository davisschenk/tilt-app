import { beforeEach, describe, expect, it } from "vitest"
import { screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { hydrometer } from "@/test/fixtures"
import OnboardingChecklist from "./onboarding-checklist"

function mock(keys: unknown[], hs: unknown[]) {
  server.use(
    http.get("/api/v1/api-keys", () => HttpResponse.json(keys)),
    http.get("/api/v1/hydrometers", () => HttpResponse.json(hs)),
  )
}

beforeEach(() => localStorage.clear())

describe("OnboardingChecklist", () => {
  it("guides a brand-new account", async () => {
    mock([], [])
    renderWithProviders(<OnboardingChecklist />)
    expect(await screen.findByText("Get started")).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Create key" })).toHaveAttribute("href", "/settings/api-keys")
    expect(screen.getByRole("link", { name: "Open device setup" })).toHaveAttribute("href", "/devices/setup")
  })

  it("marks the key step done once a key exists", async () => {
    mock([{ id: "k" }], [])
    renderWithProviders(<OnboardingChecklist />)
    await screen.findByText("Get started")
    expect(screen.queryByRole("link", { name: "Create key" })).not.toBeInTheDocument()
    expect(screen.getByText("Create an API key")).toHaveClass("line-through")
  })

  it("disappears once a reading has arrived", async () => {
    mock([{ id: "k" }], [hydrometer()])
    renderWithProviders(<OnboardingChecklist />)
    await screen.findByText("Get started").then(
      () => { throw new Error("should not render") },
      () => {},
    )
    expect(screen.queryByText("Get started")).not.toBeInTheDocument()
  })

  it("can be dismissed and stays dismissed", async () => {
    mock([], [])
    const first = renderWithProviders(<OnboardingChecklist />)
    await userEvent.click(await screen.findByRole("button", { name: "Dismiss" }))
    expect(screen.queryByText("Get started")).not.toBeInTheDocument()
    expect(localStorage.getItem("tiltbrew.onboarding.dismissed")).toBe("1")
    first.unmount()
    renderWithProviders(<OnboardingChecklist />)
    expect(screen.queryByText("Get started")).not.toBeInTheDocument()
  })
})
