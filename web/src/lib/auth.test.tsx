import { describe, expect, it } from "vitest"
import { screen, waitFor, render } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { Providers } from "@/test/render"
import { useAuth } from "./auth-context"

function Probe() {
  const { isAuthenticated, isLoading, user, logout } = useAuth()
  if (isLoading) return <p>loading</p>
  return (
    <div>
      <p>{isAuthenticated ? `hi ${user?.name}` : "anon"}</p>
      <button onClick={logout}>out</button>
    </div>
  )
}

describe("AuthProvider", () => {
  it("authenticates when /auth/me succeeds", async () => {
    render(<Providers><Probe /></Providers>)
    expect(screen.getByText("loading")).toBeInTheDocument()
    expect(await screen.findByText("hi Dev")).toBeInTheDocument()
  })

  it("is anonymous on 401", async () => {
    server.use(http.get("/api/v1/auth/me", () => new HttpResponse(null, { status: 401 })))
    render(<Providers><Probe /></Providers>)
    expect(await screen.findByText("anon")).toBeInTheDocument()
  })

  it("logout clears the user", async () => {
    const loc = { href: "" }
    Object.defineProperty(window, "location", { value: loc, writable: true })
    render(<Providers><Probe /></Providers>)
    await userEvent.click(await screen.findByText("out"))
    await waitFor(() => expect(loc.href).toBe("/"))
  })

  it("useAuth throws outside provider", () => {
    expect(() => render(<Probe />)).toThrow(/AuthProvider/)
  })
})
