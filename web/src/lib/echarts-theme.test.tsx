import { expect, it } from "vitest"
import { renderHook, waitFor } from "@testing-library/react"
import { ThemeProvider } from "@/components/theme-provider"
import { useEChartsTheme } from "./echarts-theme"

it("builds a theme and flags dark mode from the root class", async () => {
  document.documentElement.classList.add("dark")
  const { result } = renderHook(() => useEChartsTheme(), { wrapper: ThemeProvider })
  expect(result.current.gridColor).toContain("rgba")
  await waitFor(() => expect(result.current.isDark).toBe(true))
  document.documentElement.classList.remove("dark")
})
