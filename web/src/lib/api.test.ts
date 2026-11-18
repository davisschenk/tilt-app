import { describe, expect, it } from "vitest"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { ApiError, apiDelete, apiGet, apiPost, apiPut } from "./api"

describe("api client", () => {
  it("parses JSON on success", async () => {
    server.use(http.get("/api/v1/x", () => HttpResponse.json({ a: 1 })))
    expect(await apiGet("/x")).toEqual({ a: 1 })
  })

  it("returns undefined on 204", async () => {
    server.use(http.delete("/api/v1/x", () => new HttpResponse(null, { status: 204 })))
    expect(await apiDelete("/x")).toBeUndefined()
  })

  it("throws ApiError with JSON body", async () => {
    server.use(http.get("/api/v1/x", () => HttpResponse.json({ error: "nope" }, { status: 400 })))
    const err = (await apiGet("/x").catch((e) => e)) as ApiError
    expect(err).toBeInstanceOf(ApiError)
    expect(err.status).toBe(400)
    expect(err.body).toEqual({ error: "nope" })
  })

  it("falls back to text body when error is not JSON", async () => {
    server.use(http.get("/api/v1/x", () => new HttpResponse("boom", { status: 500 })))
    const err = (await apiGet("/x").catch((e) => e)) as ApiError
    expect(err.status).toBe(500)
    expect(err.body).toBe("boom")
  })

  it("sends JSON bodies for POST and PUT", async () => {
    const seen: unknown[] = []
    server.use(
      http.post("/api/v1/x", async ({ request }) => {
        seen.push(await request.json())
        return HttpResponse.json({ ok: true })
      }),
      http.put("/api/v1/x", async ({ request }) => {
        seen.push(await request.json())
        return HttpResponse.json({ ok: true })
      }),
    )
    await apiPost("/x", { p: 1 })
    await apiPut("/x", { q: 2 })
    expect(seen).toEqual([{ p: 1 }, { q: 2 }])
  })
})
