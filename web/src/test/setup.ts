import "@testing-library/jest-dom/vitest"
import { afterAll, afterEach, beforeAll, vi } from "vitest"
import { cleanup } from "@testing-library/react"
import { server } from "./msw/server"

const originalLocation = Object.getOwnPropertyDescriptor(window, "location")!

beforeAll(() => server.listen({ onUnhandledFrame: "error" }))
afterEach(() => {
  cleanup()
  Object.defineProperty(window, "location", originalLocation)
  server.resetHandlers()
})
afterAll(() => server.close())

if (!window.matchMedia) {
  window.matchMedia = vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }))
}

class RO {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= RO as unknown as typeof ResizeObserver
Element.prototype.scrollIntoView ??= vi.fn()
Element.prototype.hasPointerCapture ??= vi.fn(() => false)
