import { describe, expect, it } from "vitest"
import { renderHook, waitFor } from "@testing-library/react"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { Providers } from "@/test/render"
import { brew } from "@/test/fixtures"
import { useBrew, useBrews, useCreateBrew, useDeleteBrew, useUpdateBrew } from "./use-brews"

const wrapper = ({ children }: { children: React.ReactNode }) => <Providers>{children}</Providers>

describe("brew hooks", () => {
  it("passes status filter as query param", async () => {
    let url = ""
    server.use(http.get("/api/v1/brews", ({ request }) => ((url = request.url), HttpResponse.json([brew()]))))
    const { result } = renderHook(() => useBrews("Active"), { wrapper })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(url).toContain("status=Active")
  })

  it("fetches a single brew", async () => {
    server.use(http.get("/api/v1/brews/b1", () => HttpResponse.json(brew())))
    const { result } = renderHook(() => useBrew("b1"), { wrapper })
    await waitFor(() => expect(result.current.data?.name).toBe("Test IPA"))
  })

  it("create/update/delete mutate", async () => {
    const calls: string[] = []
    server.use(
      http.post("/api/v1/brews", () => (calls.push("post"), HttpResponse.json(brew()))),
      http.put("/api/v1/brews/b1", () => (calls.push("put"), HttpResponse.json(brew()))),
      http.delete("/api/v1/brews/b1", () => (calls.push("delete"), new HttpResponse(null, { status: 204 }))),
    )
    const c = renderHook(() => useCreateBrew(), { wrapper })
    const u = renderHook(() => useUpdateBrew("b1"), { wrapper })
    const d = renderHook(() => useDeleteBrew(), { wrapper })
    await c.result.current.mutateAsync({ name: "n", hydrometerId: "h1" })
    await u.result.current.mutateAsync({ name: "m" })
    await d.result.current.mutateAsync("b1")
    expect(calls).toEqual(["post", "put", "delete"])
  })
})
