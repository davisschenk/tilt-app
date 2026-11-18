import { describe, expect, it } from "vitest"
import { renderHook, waitFor } from "@testing-library/react"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { Providers } from "@/test/render"
import { hydrometer } from "@/test/fixtures"
import {
  useCreateHydrometer,
  useDeleteHydrometer,
  useHydrometer,
  useHydrometers,
  useUpdateHydrometer,
} from "./use-hydrometers"

const wrapper = ({ children }: { children: React.ReactNode }) => <Providers>{children}</Providers>

describe("hydrometer hooks", () => {
  it("lists hydrometers", async () => {
    server.use(http.get("/api/v1/hydrometers", () => HttpResponse.json([hydrometer()])))
    const { result } = renderHook(() => useHydrometers(), { wrapper })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data).toHaveLength(1)
  })

  it("fetches one by id and skips when id empty", async () => {
    server.use(http.get("/api/v1/hydrometers/h1", () => HttpResponse.json(hydrometer())))
    const { result } = renderHook(() => useHydrometer("h1"), { wrapper })
    await waitFor(() => expect(result.current.data?.id).toBe("h1"))
    const empty = renderHook(() => useHydrometer(""), { wrapper })
    expect(empty.result.current.fetchStatus).toBe("idle")
  })

  it("create/update/delete hit the right endpoints", async () => {
    const calls: string[] = []
    server.use(
      http.post("/api/v1/hydrometers", () => (calls.push("post"), HttpResponse.json(hydrometer()))),
      http.put("/api/v1/hydrometers/h1", () => (calls.push("put"), HttpResponse.json(hydrometer()))),
      http.delete("/api/v1/hydrometers/h1", () => (calls.push("delete"), new HttpResponse(null, { status: 204 }))),
    )
    const create = renderHook(() => useCreateHydrometer(), { wrapper })
    const update = renderHook(() => useUpdateHydrometer("h1"), { wrapper })
    const del = renderHook(() => useDeleteHydrometer(), { wrapper })
    await create.result.current.mutateAsync({ color: "Red" })
    await update.result.current.mutateAsync({ name: "x" })
    await del.result.current.mutateAsync("h1")
    expect(calls).toEqual(["post", "put", "delete"])
  })

  it("surfaces errors", async () => {
    server.use(http.get("/api/v1/hydrometers", () => new HttpResponse(null, { status: 500 })))
    const { result } = renderHook(() => useHydrometers(), { wrapper })
    await waitFor(() => expect(result.current.isError).toBe(true))
  })
})
