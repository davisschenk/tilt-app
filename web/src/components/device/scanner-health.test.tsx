import { describe, it, expect } from "vitest";
import { screen } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import { server } from "@/test/msw/server";
import { renderWithProviders } from "@/test/render";
import ScannerHealth from "./scanner-health";
describe("ScannerHealth", () => {
  it("distinguishes scanner contact from failed BLE scans", async () => {
    server.use(
      http.get("/api/v1/scanners", () =>
        HttpResponse.json([
          {
            id: "s",
            deviceId: "scanner-1",
            lastSeenAt: new Date().toISOString(),
            scanIntervalSeconds: 60,
            firmwareVersion: "1.0",
            uptimeSeconds: 100,
            wifiRssi: -55,
            bufferedReadings: 4,
            scanOk: false,
            uploadOk: false,
          },
        ]),
      ),
    );
    renderWithProviders(<ScannerHealth />);
    expect(await screen.findByText("scanner-1 · Online")).toBeInTheDocument();
    expect(screen.getByText(/Last scan: Failed/)).toBeInTheDocument();
    expect(screen.getByText(/Buffered readings: 4/)).toBeInTheDocument();
  });
  it("shows older firmware as unavailable", async () => {
    renderWithProviders(<ScannerHealth />);
    expect(
      await screen.findByText(/Scanner status unavailable/),
    ).toBeInTheDocument();
  });
});
