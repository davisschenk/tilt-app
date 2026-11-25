import { describe, it, expect } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Routes, Route } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { server } from "@/test/msw/server";
import { renderWithProviders } from "@/test/render";
import { brew } from "@/test/fixtures";
import BrewReport, { PublicBrewReport } from "./brew-report";
describe("batch reports", () => {
  it("shares selected interventions and keeps journal notes and photos opt-in", async () => {
    let body: unknown;
    server.use(
      http.get("/api/v1/brews/b1", () =>
        HttpResponse.json(brew({ notes: "Private recipe note" })),
      ),
      http.get("/api/v1/brews/b1/events", () =>
        HttpResponse.json([
          {
            id: "e1",
            label: "Dry hop",
            eventType: "dry_hop",
            eventTime: new Date().toISOString(),
            gravityAtEvent: 1.02,
            tempAtEvent: 68,
            notes: "Private entry note",
            attachments: [],
          },
          {
            id: "e2",
            label: "Personal tasting",
            eventType: "tasting_note",
            eventTime: new Date().toISOString(),
            gravityAtEvent: null,
            tempAtEvent: null,
            notes: "Private tasting",
            attachments: [],
          },
        ]),
      ),
      http.get("/api/v1/readings", () => HttpResponse.json([])),
      http.get("/api/v1/brews/b1/reports", () => HttpResponse.json([])),
      http.post("/api/v1/brews/b1/reports", async ({ request }) => {
        body = await request.json();
        return HttpResponse.json({ id: "r", token: "token" });
      }),
    );
    renderWithProviders(
      <Routes>
        <Route path="/brews/:id/report" element={<BrewReport />} />
      </Routes>,
      { route: "/brews/b1/report" },
    );
    expect(
      await screen.findByRole("heading", { name: "Report preview" }),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText("Include batch and entry notes"),
    ).not.toBeChecked();
    expect(
      screen.getByLabelText("Include selected entries’ photos"),
    ).not.toBeChecked();
    expect(screen.queryByText("Private recipe note")).not.toBeInTheDocument();
    expect(screen.getByLabelText(/Personal tasting/)).not.toBeChecked();
    await userEvent.click(
      screen.getByRole("button", { name: "Create share link" }),
    );
    await waitFor(() =>
      expect(body).toEqual({
        eventIds: ["e1"],
        includeNotes: false,
        includePhotos: false,
        units: { temperature: "F", gravity: "SG" },
      }),
    );
  });
  it("handles revoked public links without offering editing controls", async () => {
    server.use(
      http.get(
        "/api/v1/reports/revoked",
        () => new HttpResponse(null, { status: 404 }),
      ),
    );
    renderWithProviders(
      <Routes>
        <Route path="/reports/:token" element={<PublicBrewReport />} />
      </Routes>,
      { route: "/reports/revoked" },
    );
    expect(
      await screen.findByText(/unavailable or its link has been revoked/),
    ).toBeInTheDocument();
    expect(screen.queryByText("Create share link")).not.toBeInTheDocument();
  });
});
