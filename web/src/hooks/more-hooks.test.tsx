import { describe, expect, it, vi } from "vitest"
import { renderHook, waitFor } from "@testing-library/react"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { Providers, makeQueryClient } from "@/test/render"
import { alertRule, alertTarget } from "@/test/fixtures"
import { useAlertRule, useAlertRules, useCreateAlertRule, useDeleteAlertRule, useUpdateAlertRule } from "./use-alert-rules"
import {
  useAlertTarget, useAlertTargets, useCreateAlertTarget, useDeleteAlertTarget, useTestFireTarget, useUpdateAlertTarget,
} from "./use-alert-targets"
import { useApiKeys } from "./use-api-keys"
import { useBrewAnalytics } from "./use-brew-analytics"
import { useBrewEvents, useCreateBrewEvent, useDeleteBrewEvent, useUpdateBrewEvent } from "./use-brew-events"
import { useDeleteAttachment, useUploadAttachment } from "./use-attachments"
import { useNutrientSchedule, useUpdateBrewNutrientSetup } from "./use-nutrient-schedule"
import { useReadings } from "./use-readings"
import { downloadFirmware, useFirmwareManifest } from "./use-firmware"

const wrapper = ({ children }: { children: React.ReactNode }) => <Providers>{children}</Providers>
const ok204 = () => new HttpResponse(null, { status: 204 })

describe("alert rule hooks", () => {
  it("builds the query string from filters", async () => {
    const urls: string[] = []
    server.use(http.get("/api/v1/alert-rules", ({ request }) => (urls.push(request.url), HttpResponse.json([]))))
    const a = renderHook(() => useAlertRules(), { wrapper })
    const b = renderHook(() => useAlertRules("b1", "h1"), { wrapper })
    await waitFor(() => expect(a.result.current.isSuccess && b.result.current.isSuccess).toBe(true))
    expect(urls.some((u) => u.includes("brew_id=b1") && u.includes("hydrometer_id=h1"))).toBe(true)
    expect(urls.some((u) => !u.includes("?"))).toBe(true)
  })

  it("get / create / update / delete", async () => {
    const calls: string[] = []
    server.use(
      http.get("/api/v1/alert-rules/r1", () => HttpResponse.json(alertRule())),
      http.post("/api/v1/alert-rules", () => (calls.push("post"), HttpResponse.json(alertRule()))),
      http.put("/api/v1/alert-rules/r1", () => (calls.push("put"), HttpResponse.json(alertRule()))),
      http.delete("/api/v1/alert-rules/r1", () => (calls.push("delete"), ok204())),
    )
    const one = renderHook(() => useAlertRule("r1"), { wrapper })
    await waitFor(() => expect(one.result.current.data?.id).toBe("r1"))
    await renderHook(() => useCreateAlertRule(), { wrapper }).result.current.mutateAsync({
      name: "n", metric: "gravity", operator: "lte", threshold: 1, alertTargetId: "t1",
    })
    await renderHook(() => useUpdateAlertRule("r1"), { wrapper }).result.current.mutateAsync({ name: "m" })
    await renderHook(() => useDeleteAlertRule(), { wrapper }).result.current.mutateAsync("r1")
    expect(calls).toEqual(["post", "put", "delete"])
  })
})

describe("alert target hooks", () => {
  it("list / get / create / update / delete / test", async () => {
    const calls: string[] = []
    server.use(
      http.get("/api/v1/alert-targets", () => HttpResponse.json([alertTarget()])),
      http.get("/api/v1/alert-targets/t1", () => HttpResponse.json(alertTarget())),
      http.post("/api/v1/alert-targets", () => (calls.push("post"), HttpResponse.json(alertTarget()))),
      http.put("/api/v1/alert-targets/t1", () => (calls.push("put"), HttpResponse.json(alertTarget()))),
      http.delete("/api/v1/alert-targets/t1", () => (calls.push("delete"), ok204())),
      http.post("/api/v1/alert-targets/t1/test", () => (calls.push("test"), HttpResponse.json({ ok: true, statusCode: 200 }))),
    )
    const list = renderHook(() => useAlertTargets(), { wrapper })
    const one = renderHook(() => useAlertTarget("t1"), { wrapper })
    await waitFor(() => expect(list.result.current.data).toHaveLength(1))
    await waitFor(() => expect(one.result.current.data?.id).toBe("t1"))
    await renderHook(() => useCreateAlertTarget(), { wrapper }).result.current.mutateAsync({ name: "a", url: "u", format: "slack" })
    await renderHook(() => useUpdateAlertTarget("t1"), { wrapper }).result.current.mutateAsync({ name: "b" })
    await renderHook(() => useDeleteAlertTarget(), { wrapper }).result.current.mutateAsync("t1")
    const fire = await renderHook(() => useTestFireTarget(), { wrapper }).result.current.mutateAsync("t1")
    expect(fire.ok).toBe(true)
    expect(calls).toEqual(["post", "put", "delete", "test"])
  })
})

describe("data hooks", () => {
  it("api keys", async () => {
    server.use(http.get("/api/v1/api-keys", () => HttpResponse.json([{ id: "k" }])))
    const { result } = renderHook(() => useApiKeys(), { wrapper })
    await waitFor(() => expect(result.current.data).toHaveLength(1))
  })

  it("brew analytics skips when id is empty", async () => {
    server.use(http.get("/api/v1/brews/b1/analytics", () => HttpResponse.json({ currentGravity: 1.01, gaps: [] })))
    const { result } = renderHook(() => useBrewAnalytics("b1"), { wrapper })
    await waitFor(() => expect(result.current.data?.currentGravity).toBe(1.01))
    expect(renderHook(() => useBrewAnalytics(""), { wrapper }).result.current.fetchStatus).toBe("idle")
  })

  it("readings serialize all filters", async () => {
    let url = ""
    server.use(http.get("/api/v1/readings", ({ request }) => ((url = request.url), HttpResponse.json([]))))
    const { result } = renderHook(
      () => useReadings({ brewId: "b", hydrometerId: "h", since: "s", until: "u", limit: 10 }),
      { wrapper },
    )
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    const q = new URL(url).searchParams
    expect(Object.fromEntries(q)).toEqual({ brew_id: "b", hydrometer_id: "h", since: "s", until: "u", limit: "10" })
    const bare = renderHook(() => useReadings(), { wrapper })
    await waitFor(() => expect(bare.result.current.isSuccess).toBe(true))
  })

  it("brew events CRUD invalidates related queries", async () => {
    const calls: string[] = []
    server.use(
      http.get("/api/v1/brews/b1/events", () => HttpResponse.json([])),
      http.post("/api/v1/brews/b1/events", () => (calls.push("post"), HttpResponse.json({ id: "e1" }))),
      http.put("/api/v1/brews/b1/events/e1", () => (calls.push("put"), HttpResponse.json({ id: "e1" }))),
      http.delete("/api/v1/brews/b1/events/e1", () => (calls.push("delete"), ok204())),
    )
    const client = makeQueryClient()
    const inv = vi.spyOn(client, "invalidateQueries")
    const w = ({ children }: { children: React.ReactNode }) => <Providers client={client}>{children}</Providers>
    const list = renderHook(() => useBrewEvents("b1"), { wrapper: w })
    await waitFor(() => expect(list.result.current.isSuccess).toBe(true))
    await renderHook(() => useCreateBrewEvent("b1"), { wrapper: w }).result.current.mutateAsync({
      brewId: "b1", eventType: "note", label: "l", eventTime: "t",
    })
    expect(inv).toHaveBeenCalledWith({ queryKey: ["nutrient-schedule", "b1"] })
    await renderHook(() => useUpdateBrewEvent("b1", "e1"), { wrapper: w }).result.current.mutateAsync({ label: "x" })
    await renderHook(() => useDeleteBrewEvent("b1"), { wrapper: w }).result.current.mutateAsync("e1")
    expect(calls).toEqual(["post", "put", "delete"])
  })

  it("attachments upload and delete", async () => {
    let contentType = ""
    let deleted = false
    server.use(
      http.post("/api/v1/brews/b1/events/e1/attachments", ({ request }) => {
        contentType = request.headers.get("content-type") ?? ""
        return HttpResponse.json({ id: "a1" })
      }),
      http.delete("/api/v1/brews/b1/events/e1/attachments/a1", () => ((deleted = true), ok204())),
    )
    const up = renderHook(() => useUploadAttachment("b1", "e1"), { wrapper })
    await up.result.current.mutateAsync(new File(["x"], "p.png", { type: "image/png" }))
    expect(contentType).toContain("multipart/form-data")
    await renderHook(() => useDeleteAttachment("b1", "e1"), { wrapper }).result.current.mutateAsync("a1")
    expect(deleted).toBe(true)
  })

  it("nutrient schedule only runs when the brew is fully configured", async () => {
    server.use(
      http.get("/api/v1/brews/b1/nutrient-schedule", () => HttpResponse.json({ additions: [] })),
      http.put("/api/v1/brews/b1", () => HttpResponse.json({ id: "b1" })),
    )
    const base = { id: "b1", og: 1.1, targetFg: 1.0, batchSizeGallons: 5, pitchTime: "2026-01-01T00:00:00Z" }
    const ready = renderHook(() => useNutrientSchedule(base), { wrapper })
    await waitFor(() => expect(ready.result.current.isSuccess).toBe(true))
    const notReady = renderHook(() => useNutrientSchedule({ ...base, og: null }), { wrapper })
    expect(notReady.result.current.fetchStatus).toBe("idle")
    await renderHook(() => useUpdateBrewNutrientSetup("b1"), { wrapper }).result.current.mutateAsync({ yeastStrain: "x" })
  })
})

describe("firmware", () => {
  it("manifest does not retry on 404", async () => {
    let hits = 0
    server.use(http.get("/api/v1/firmware/manifest", () => (hits++, new HttpResponse(null, { status: 404 }))))
    const { result } = renderHook(() => useFirmwareManifest(), { wrapper })
    await waitFor(() => expect(result.current.isError).toBe(true))
    expect(hits).toBe(1)
  })

  async function sha(bytes: Uint8Array) {
    const d = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))
    return Array.from(d).map((b) => b.toString(16).padStart(2, "0")).join("")
  }

  it("downloadFirmware verifies the checksum", async () => {
    const bytes = new Uint8Array([1, 2, 3])
    server.use(http.get("/api/v1/firmware/files/fw.bin", () => new HttpResponse(bytes)))
    const good = await downloadFirmware({ name: "fw.bin", sha256: (await sha(bytes)).toUpperCase(), size: 3 })
    expect([...good]).toEqual([1, 2, 3])
    await expect(downloadFirmware({ name: "fw.bin", sha256: "00", size: 3 })).rejects.toThrow(/checksum mismatch/)
  })

  it("downloadFirmware surfaces HTTP errors", async () => {
    server.use(http.get("/api/v1/firmware/files/x.bin", () => new HttpResponse(null, { status: 404 })))
    await expect(downloadFirmware({ name: "x.bin", sha256: "", size: 0 })).rejects.toThrow("HTTP 404")
  })
})
