import { afterEach, describe, expect, it, vi } from "vitest"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { ApiError, downloadFile } from "./api"

describe("downloadFile", () => {
  afterEach(() => vi.restoreAllMocks())

  it("saves the response as a named download", async () => {
    server.use(http.get("/api/v1/export", () => new HttpResponse("a,b", { headers: { "content-type": "text/csv" } })))
    const create = vi.fn().mockReturnValue("blob:x")
    const revoke = vi.fn()
    Object.assign(URL, { createObjectURL: create, revokeObjectURL: revoke })
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {})
    await downloadFile("/export", "data.csv")
    expect(create).toHaveBeenCalled()
    expect(click).toHaveBeenCalled()
    expect(revoke).toHaveBeenCalledWith("blob:x")
    expect(document.querySelector("a[download]")).toBeNull()
  })

  it("throws ApiError on failure", async () => {
    server.use(http.get("/api/v1/export", () => new HttpResponse(null, { status: 403 })))
    await expect(downloadFile("/export", "x")).rejects.toBeInstanceOf(ApiError)
  })
})
