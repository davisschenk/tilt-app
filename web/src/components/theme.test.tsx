import { beforeEach, describe, expect, it, vi } from "vitest"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { ThemeProvider, useTheme } from "./theme-provider"
import ThemeToggle from "./theme-toggle"

function Show() {
  return <p>theme:{useTheme().theme}</p>
}

beforeEach(() => {
  localStorage.clear()
  document.documentElement.classList.remove("dark")
})

describe("theme", () => {
  it("defaults to system and follows the OS preference", () => {
    window.matchMedia = vi.fn().mockImplementation(() => ({
      matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn(),
    })) as never
    render(<ThemeProvider><Show /></ThemeProvider>)
    expect(screen.getByText("theme:system")).toBeInTheDocument()
    expect(document.documentElement).toHaveClass("dark")
  })

  it("restores a stored theme", () => {
    localStorage.setItem("tilt-theme", "dark")
    render(<ThemeProvider><Show /></ThemeProvider>)
    expect(screen.getByText("theme:dark")).toBeInTheDocument()
    expect(document.documentElement).toHaveClass("dark")
  })

  it("toggle switches and persists the theme", async () => {
    render(<ThemeProvider><ThemeToggle /><Show /></ThemeProvider>)
    await userEvent.click(screen.getByRole("button", { name: "Toggle theme" }))
    await userEvent.click(await screen.findByRole("menuitem", { name: "Dark" }))
    expect(localStorage.getItem("tilt-theme")).toBe("dark")
    expect(document.documentElement).toHaveClass("dark")
    await userEvent.click(screen.getByRole("button", { name: "Toggle theme" }))
    await userEvent.click(await screen.findByRole("menuitem", { name: "Light" }))
    expect(localStorage.getItem("tilt-theme")).toBe("light")
    expect(document.documentElement).not.toHaveClass("dark")
  })
})
