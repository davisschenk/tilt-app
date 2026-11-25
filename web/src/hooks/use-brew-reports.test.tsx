import { describe, expect, it } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import { server } from "@/test/msw/server";
import { Providers } from "@/test/render";
import {
  useBrewReports,
  usePublicBrewReport,
  usePublishBrewReport,
  useRevokeBrewReport,
} from "./use-brew-reports";
import { useScanners } from "./use-scanners";
import type { PublishBrewReport } from "@/types";

const wrapper = ({ children }: { children: React.ReactNode }) => (
  <Providers>{children}</Providers>
);
const options: PublishBrewReport = {
  eventIds: ["e1"],
  includeNotes: false,
  includePhotos: false,
  units: { temperature: "F", gravity: "SG" },
};

describe("brew report hooks", () => {
  it("creates, refreshes and revokes links and refetches the list", async () => {
    const calls: string[] = [];
    let listed = 0;
    server.use(
      http.get("/api/v1/brews/b1/reports", () => {
        listed += 1;
        return HttpResponse.json([]);
      }),
      http.post("/api/v1/brews/b1/reports", () => {
        calls.push("post");
        return HttpResponse.json({ id: "r1", token: "t" });
      }),
      http.put("/api/v1/brews/b1/reports/r1", () => {
        calls.push("put");
        return HttpResponse.json({ id: "r1", token: "t" });
      }),
      http.delete("/api/v1/brews/b1/reports/r1", () => {
        calls.push("delete");
        return new HttpResponse(null, { status: 204 });
      }),
    );
    const { result } = renderHook(
      () => ({
        list: useBrewReports("b1"),
        publish: usePublishBrewReport("b1"),
        revoke: useRevokeBrewReport("b1"),
      }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.list.isSuccess).toBe(true));
    await result.current.publish.mutateAsync({ options });
    await result.current.publish.mutateAsync({ options, reportId: "r1" });
    await result.current.revoke.mutateAsync("r1");
    expect(calls).toEqual(["post", "put", "delete"]);
    await waitFor(() => expect(listed).toBeGreaterThan(1));
  });

  it("loads a public report and does not retry a revoked one", async () => {
    let hits = 0;
    server.use(
      http.get("/api/v1/reports/gone", () => {
        hits += 1;
        return new HttpResponse(null, { status: 404 });
      }),
    );
    const { result } = renderHook(() => usePublicBrewReport("gone"), {
      wrapper,
    });
    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(hits).toBe(1);
  });
});

describe("useScanners", () => {
  it("lists scanner heartbeats", async () => {
    server.use(
      http.get("/api/v1/scanners", () =>
        HttpResponse.json([{ id: "s1", deviceId: "scanner-1" }]),
      ),
    );
    const { result } = renderHook(() => useScanners(), { wrapper });
    await waitFor(() => expect(result.current.data?.[0].deviceId).toBe("scanner-1"));
  });
});
