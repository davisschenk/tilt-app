import { afterEach, describe, expect, it, vi } from "vitest"
import { act, render, renderHook, screen } from "@testing-library/react"
import { useIsMobile, useMediaQuery } from "./use-media-query"
import { useElementWidth } from "./use-element-width"

function mockMatchMedia(initial: boolean) {
  const listeners = new Set<() => void>()
  let matches = initial
  const original = window.matchMedia
  window.matchMedia = vi.fn((query: string) => ({
    get matches() { return matches },
    media: query,
    addEventListener: (_: string, cb: () => void) => listeners.add(cb),
    removeEventListener: (_: string, cb: () => void) => listeners.delete(cb),
  })) as unknown as typeof window.matchMedia
  return {
    set(next: boolean) { matches = next; listeners.forEach((cb) => cb()) },
    listeners,
    restore() { window.matchMedia = original },
  }
}

describe("useMediaQuery", () => {
  let mm: ReturnType<typeof mockMatchMedia>
  afterEach(() => mm.restore())

  it("reflects the query and follows changes", () => {
    mm = mockMatchMedia(true)
    const { result, unmount } = renderHook(() => useMediaQuery("(max-width: 10px)"))
    expect(result.current).toBe(true)
    act(() => mm.set(false))
    expect(result.current).toBe(false)
    unmount()
    expect(mm.listeners.size).toBe(0)
  })

  it("useIsMobile queries below the md breakpoint", () => {
    mm = mockMatchMedia(false)
    const { result } = renderHook(() => useIsMobile())
    expect(result.current).toBe(false)
    expect(window.matchMedia).toHaveBeenCalledWith("(max-width: 767.98px)")
  })
})

describe("useElementWidth", () => {
  const original = globalThis.ResizeObserver
  afterEach(() => { globalThis.ResizeObserver = original })

  it("observes the element once it mounts, even after the first render", () => {
    let fire: (w: number) => void = () => {}
    const disconnect = vi.fn()
    globalThis.ResizeObserver = class {
      constructor(cb: ResizeObserverCallback) {
        fire = (w) => cb([{ contentRect: { width: w } } as ResizeObserverEntry], this as unknown as ResizeObserver)
      }
      observe() {}
      unobserve() {}
      disconnect = disconnect
    } as unknown as typeof ResizeObserver

    function Probe({ show }: { show: boolean }) {
      const { ref, width } = useElementWidth<HTMLDivElement>(800)
      return <>{show && <div ref={ref} />}<span>width:{width}</span></>
    }
    const { rerender } = render(<Probe show={false} />)
    expect(screen.getByText("width:800")).toBeInTheDocument()
    rerender(<Probe show />)
    act(() => fire(308))
    expect(screen.getByText("width:308")).toBeInTheDocument()
    rerender(<Probe show={false} />)
    expect(disconnect).toHaveBeenCalled()
  })
})
