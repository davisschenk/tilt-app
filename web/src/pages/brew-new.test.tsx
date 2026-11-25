import { describe, expect, it } from "vitest"
import { screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { Route, Routes } from "react-router-dom"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { brew, hydrometer } from "@/test/fixtures"
import BrewNew from "./brew-new"

function setup() {
  server.use(
    http.get("/api/v1/hydrometers", () => HttpResponse.json([hydrometer()])),
    http.get("/api/v1/brews", () => HttpResponse.json([])),
  )
  return renderWithProviders(
    <Routes>
      <Route path="/" element={<BrewNew />} />
      <Route path="/brews" element={<p>brews list</p>} />
      <Route path="/brews/:id" element={<p>detail b9</p>} />
    </Routes>,
  )
}

describe("BrewNew", () => {
  it("validates required fields", async () => {
    setup()
    await userEvent.click(await screen.findByRole("button", { name: "Create Brew" }))
    expect(screen.getByText("Name is required")).toBeInTheDocument()
    expect(screen.getByText("Hydrometer is required")).toBeInTheDocument()
  })

  it("creates a brew, prefilling OG from the hydrometer reading", async () => {
    let body: Record<string, unknown> = {}
    server.use(
      http.post("/api/v1/brews", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>
        return HttpResponse.json(brew({ id: "b9" }))
      }),
    )
    setup()
    await userEvent.type(await screen.findByLabelText("Name *"), "  My Mead ")
    await userEvent.type(screen.getByLabelText("Style"), "Mead")
    await userEvent.click(screen.getByRole("combobox"))
    await userEvent.click(await screen.findByRole("option", { name: /Kitchen Tilt/ }))
    expect(screen.getByLabelText("Original Gravity (SG)")).toHaveValue(1.05)
    await userEvent.type(screen.getByLabelText("Target FG (SG)"), "1.010")
    await userEvent.click(screen.getByRole("button", { name: "Create Brew" }))
    expect(await screen.findByText("detail b9")).toBeInTheDocument()
    expect(body).toMatchObject({ name: "My Mead", style: "Mead", hydrometerId: "h1", og: 1.05, targetFg: 1.01, notes: null })
  })

  it("stays on the form when creation fails", async () => {
    server.use(http.post("/api/v1/brews", () => new HttpResponse(null, { status: 500 })))
    setup()
    await userEvent.type(await screen.findByLabelText("Name *"), "X")
    await userEvent.click(screen.getByRole("combobox"))
    await userEvent.click(await screen.findByRole("option", { name: /Kitchen Tilt/ }))
    await userEvent.click(screen.getByRole("button", { name: "Create Brew" }))
    await waitFor(() => expect(screen.getByRole("button", { name: "Create Brew" })).toBeEnabled())
    expect(screen.queryByText("detail b9")).not.toBeInTheDocument()
  })

  it("cancel returns to the list", async () => {
    setup()
    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }))
    expect(await screen.findByText("brews list")).toBeInTheDocument()
  })
})
describe("duplicate batch", () => {
  it("reviews copied setup and creates a fresh batch without historical data", async () => {
    let body: Record<string, unknown> | undefined;
    server.use(
      http.get("/api/v1/brews/b1", () =>
        HttpResponse.json(
          brew({
            status: "Completed",
            name: "House IPA",
            yeastStrain: "US-05",
            batchSizeGallons: 5,
            nutrientProtocol: "tosna_3",
            pitchTime: "2026-01-01T00:00:00Z",
            fg: 1.01,
          }),
        ),
      ),
      http.get("/api/v1/brews", () => HttpResponse.json([])),
      http.get("/api/v1/hydrometers", () =>
        HttpResponse.json([hydrometer({ id: "h2", name: "Free Tilt" })]),
      ),
      http.post("/api/v1/brews", async ({ request }) => {
        body = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json(brew({ id: "new" }), { status: 201 });
      }),
    );
    renderWithProviders(<BrewNew />, { route: "/brews/new?duplicate=b1" });
    expect(
      await screen.findByDisplayValue("House IPA (copy)"),
    ).toBeInTheDocument();
    expect(screen.getByText(/Yeast: US-05/)).toBeInTheDocument();
    const user = userEvent.setup();
    await user.click(screen.getByRole("combobox"));
    await user.click(await screen.findByRole("option", { name: /Free Tilt/ }));
    await user.click(screen.getByRole("button", { name: "Create Brew" }));
    await screen.findByRole("heading", { name: "Duplicate Brew" });
    expect(body).toMatchObject({
      name: "House IPA (copy)",
      hydrometerId: "h2",
      yeastStrain: "US-05",
      batchSizeGallons: 5,
      nutrientProtocol: "tosna_3",
    });
    expect(body).not.toHaveProperty("fg");
    expect(body).not.toHaveProperty("pitchTime");
    expect(body).not.toHaveProperty("endDate");
  });
});
