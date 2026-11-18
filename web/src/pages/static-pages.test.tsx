import { describe, expect, it } from "vitest"
import { screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import ProtectedRoute from "@/components/auth/protected-route"
import NotFound from "./not-found"
import Landing from "./landing"

describe("NotFound", () => {
  it("links home", () => {
    renderWithProviders(<NotFound />)
    expect(screen.getByText("404")).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Go to Dashboard" })).toHaveAttribute("href", "/")
  })
})

describe("Landing", () => {
  it("renders features and signs in", async () => {
    const loc = { href: "" }
    Object.defineProperty(window, "location", { value: loc, writable: true })
    renderWithProviders(<Landing />)
    expect(screen.getByText("Wireless readings")).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", { name: "Sign in" }))
    expect(loc.href).toBe("/api/v1/auth/login")
  })
})

describe("ProtectedRoute", () => {
  it("renders children when authenticated", async () => {
    renderWithProviders(<ProtectedRoute><p>secret</p></ProtectedRoute>)
    expect(await screen.findByText("secret")).toBeInTheDocument()
  })

  it("shows the landing page when anonymous", async () => {
    server.use(http.get("/api/v1/auth/me", () => new HttpResponse(null, { status: 401 })))
    renderWithProviders(<ProtectedRoute><p>secret</p></ProtectedRoute>)
    expect(await screen.findByText("Wireless readings")).toBeInTheDocument()
    expect(screen.queryByText("secret")).not.toBeInTheDocument()
  })
})
