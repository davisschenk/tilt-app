import { describe, expect, it } from "vitest"
import { screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { Route, Routes } from "react-router-dom"
import { renderWithProviders } from "@/test/render"
import AppShell from "./app-shell"

function setup(route = "/brews") {
  return renderWithProviders(
    <Routes>
      <Route element={<AppShell />}>
        <Route path="/" element={<p>dash</p>} />
        <Route path="/brews" element={<p>brews</p>} />
        <Route path="/hydrometers" element={<p>hydros</p>} />
      </Route>
    </Routes>,
    { route },
  )
}

describe("AppShell", () => {
  it("renders navigation with the active route highlighted and outlet content", () => {
    setup("/brews")
    expect(screen.getByText("brews")).toBeInTheDocument()
    const link = screen.getAllByRole("link", { name: "Brews" })[0]
    expect(link).toHaveClass("bg-primary")
    expect(screen.getAllByRole("link", { name: "Dashboard" })[0]).not.toHaveClass("bg-primary")
  })

  it("navigates via the sidebar", async () => {
    setup("/")
    await userEvent.click(screen.getAllByRole("link", { name: "Hydrometers" })[0])
    expect(await screen.findByText("hydros")).toBeInTheDocument()
  })

  it("signs out from the user menu", async () => {
    const loc = { href: "" }
    setup()
    const menu = await screen.findByRole("button", { name: /Dev/ })
    Object.defineProperty(window, "location", { value: loc, writable: true })
    await userEvent.click(menu)
    await userEvent.click(await screen.findByRole("menuitem", { name: /Sign out/ }))
    await waitFor(() => expect(loc.href).toBe("/"))
  })

  it("opens the mobile drawer and closes it after choosing a link", async () => {
    setup("/")
    await userEvent.click(screen.getByRole("button", { name: "Toggle navigation" }))
    const dialog = await screen.findByRole("dialog")
    expect(dialog).toBeInTheDocument()
    const links = screen.getAllByRole("link", { name: "Brews" })
    await userEvent.click(links[links.length - 1])
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument())
  })
})
