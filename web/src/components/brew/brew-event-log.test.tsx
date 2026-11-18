import { beforeEach, describe, expect, it, vi } from "vitest"
import { screen, waitFor, within, fireEvent } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import type { BrewEventResponse } from "@/types"
import BrewEventLog from "./brew-event-log"

const { success, error } = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }))
vi.mock("@/lib/toast", () => ({ success, error, info: vi.fn() }))

const ev = (over: Partial<BrewEventResponse> = {}): BrewEventResponse => ({
  id: "e1",
  brewId: "b1",
  eventType: "dry_hop",
  label: "Dry hop addition",
  notes: "2oz Citra",
  gravityAtEvent: 1.02,
  tempAtEvent: 66.5,
  eventTime: "2026-01-05T12:00:00Z",
  createdAt: "2026-01-05T12:00:00Z",
  attachments: [],
  ...over,
})

const photo = (i: number) => ({
  id: `a${i}`, eventId: "e1", filename: `p${i}.png`, contentType: "image/png", sizeBytes: 10,
  createdAt: "2026-01-05T12:00:00Z", url: `/api/v1/brews/b1/events/e1/attachments/a${i}`,
})

const serveEvents = (events: BrewEventResponse[]) =>
  server.use(http.get("/api/v1/brews/b1/events", () => HttpResponse.json(events)))

beforeEach(() => { success.mockClear(); error.mockClear() })

describe("BrewEventLog", () => {
  it("shows an empty state", async () => {
    serveEvents([])
    renderWithProviders(<BrewEventLog brewId="b1" />)
    expect(await screen.findByText(/No events logged yet/)).toBeInTheDocument()
  })

  it("lists events oldest-first with details, and collapses", async () => {
    serveEvents([
      ev({ id: "late", label: "Cold crash started", eventType: "cold_crash", eventTime: "2026-01-09T00:00:00Z", notes: null, gravityAtEvent: null, tempAtEvent: null }),
      ev(),
    ])
    renderWithProviders(<BrewEventLog brewId="b1" />)
    const first = await screen.findByText("Dry hop addition")
    const second = screen.getByText("Cold crash started")
    expect(first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(screen.getByText("SG 1.020")).toBeInTheDocument()
    expect(screen.getByText("66.5°F")).toBeInTheDocument()
    expect(screen.getByText("2oz Citra")).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", { name: /Fermentation Log/ }))
    expect(screen.queryByText("Dry hop addition")).not.toBeInTheDocument()
  })

  it("adds an event with defaults and parsed numbers", async () => {
    serveEvents([])
    let body: Record<string, unknown> = {}
    server.use(http.post("/api/v1/brews/b1/events", async ({ request }) => {
      body = (await request.json()) as Record<string, unknown>
      return HttpResponse.json(ev())
    }))
    renderWithProviders(<BrewEventLog brewId="b1" />)
    await userEvent.click(await screen.findByRole("button", { name: /Add Event/ }))
    const dialog = await screen.findByRole("dialog", { name: "Add Fermentation Event" })
    await userEvent.type(within(dialog).getByPlaceholderText("1.012"), "1.015")
    await userEvent.type(within(dialog).getByPlaceholderText("68.0"), "67.5")
    await userEvent.type(within(dialog).getByPlaceholderText("Optional notes..."), "hello")
    await userEvent.click(within(dialog).getByRole("button", { name: "Add Event" }))
    await waitFor(() => expect(success).toHaveBeenCalledWith("Event added"))
    expect(body).toMatchObject({ brewId: "b1", eventType: "note", label: "Note", notes: "hello", gravityAtEvent: 1.015, tempAtEvent: 67.5 })
    expect(typeof body.eventTime).toBe("string")
  })

  it("picks a specific event type and custom label", async () => {
    serveEvents([])
    let body: Record<string, unknown> = {}
    server.use(http.post("/api/v1/brews/b1/events", async ({ request }) => ((body = (await request.json()) as Record<string, unknown>), HttpResponse.json(ev()))))
    renderWithProviders(<BrewEventLog brewId="b1" />)
    await userEvent.click(await screen.findByRole("button", { name: /Add Event/ }))
    const dialog = await screen.findByRole("dialog")
    await userEvent.click(within(dialog).getByRole("combobox"))
    await userEvent.click(await screen.findByRole("option", { name: /Dry Hop/i }))
    await userEvent.type(within(dialog).getByPlaceholderText(/Dry Hop/i), "Citra")
    await userEvent.click(within(dialog).getByRole("button", { name: "Add Event" }))
    await waitFor(() => expect(body).toMatchObject({ eventType: "dry_hop", label: "Citra" }))
  })

  it("reports a failed add and keeps the dialog open", async () => {
    serveEvents([])
    server.use(http.post("/api/v1/brews/b1/events", () => new HttpResponse(null, { status: 500 })))
    renderWithProviders(<BrewEventLog brewId="b1" />)
    await userEvent.click(await screen.findByRole("button", { name: /Add Event/ }))
    await userEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Add Event" }))
    await waitFor(() => expect(error).toHaveBeenCalledWith("Failed to add event"))
    expect(screen.getByRole("dialog")).toBeInTheDocument()
  })

  it("cancel closes the add dialog", async () => {
    serveEvents([])
    renderWithProviders(<BrewEventLog brewId="b1" />)
    await userEvent.click(await screen.findByRole("button", { name: /Add Event/ }))
    await userEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Cancel" }))
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument())
  })

  it("edits an event", async () => {
    serveEvents([ev()])
    let body: Record<string, unknown> = {}
    server.use(http.put("/api/v1/brews/b1/events/e1", async ({ request }) => ((body = (await request.json()) as Record<string, unknown>), HttpResponse.json(ev()))))
    renderWithProviders(<BrewEventLog brewId="b1" />)
    await userEvent.click(await screen.findByRole("button", { name: "Edit event Dry hop addition" }))
    const dialog = await screen.findByRole("dialog", { name: "Edit Event" })
    const label = within(dialog).getAllByRole("textbox")[0]
    expect(label).toHaveValue("Dry hop addition")
    await userEvent.clear(label)
    await userEvent.type(label, "Dry hop #2")
    await userEvent.click(within(dialog).getByRole("button", { name: "Save Changes" }))
    await waitFor(() => expect(success).toHaveBeenCalledWith("Event updated"))
    expect(body).toMatchObject({ label: "Dry hop #2", notes: "2oz Citra", gravityAtEvent: 1.02, tempAtEvent: 66.5 })
  })

  it("deletes an event after confirmation, and surfaces failures", async () => {
    serveEvents([ev()])
    let status = 500
    server.use(http.delete("/api/v1/brews/b1/events/e1", () => new HttpResponse(null, { status })))
    renderWithProviders(<BrewEventLog brewId="b1" />)
    await userEvent.click(await screen.findByRole("button", { name: "Delete event Dry hop addition" }))
    const dialog = await screen.findByRole("dialog", { name: "Delete Event" })
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }))
    await waitFor(() => expect(error).toHaveBeenCalledWith("Failed to delete event"))
    status = 204
    await userEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Delete" }))
    await waitFor(() => expect(success).toHaveBeenCalledWith("Event deleted"))
  })

  describe("attachments", () => {
    const file = (name: string, type: string, size = 10) => {
      const f = new File(["x"], name, { type })
      Object.defineProperty(f, "size", { value: size })
      return f
    }
    const input = () => document.querySelector<HTMLInputElement>('input[type="file"]')!

    it("uploads an image and rejects non-images and oversized files", async () => {
      serveEvents([ev()])
      let uploads = 0
      server.use(http.post("/api/v1/brews/b1/events/e1/attachments", () => (uploads++, HttpResponse.json(photo(1)))))
      renderWithProviders(<BrewEventLog brewId="b1" />)
      await screen.findByText("Dry hop addition")
      fireEvent.change(input(), { target: { files: [file("a.txt", "text/plain")] } })
      expect(error).toHaveBeenCalledWith("Only image files are supported")
      fireEvent.change(input(), { target: { files: [file("big.png", "image/png", 11 * 1024 * 1024)] } })
      expect(error).toHaveBeenCalledWith("File must be under 10 MB")
      fireEvent.change(input(), { target: { files: [file("ok.png", "image/png")] } })
      await waitFor(() => expect(success).toHaveBeenCalledWith("Photo uploaded"))
      expect(uploads).toBe(1)
    })

    it("shows thumbnails with overflow count, opens the lightbox, and deletes with confirmation", async () => {
      serveEvents([ev({ attachments: [1, 2, 3, 4, 5, 6].map(photo) })])
      let deleted = ""
      server.use(http.delete("/api/v1/brews/b1/events/e1/attachments/:id", ({ params }) => ((deleted = String(params.id)), new HttpResponse(null, { status: 204 }))))
      renderWithProviders(<BrewEventLog brewId="b1" />)
      expect(await screen.findByText("+2 more")).toBeInTheDocument()
      expect(screen.getAllByRole("img")).toHaveLength(4)

      await userEvent.click(screen.getByTitle("p1.png"))
      const lightbox = await screen.findByRole("dialog")
      expect(within(lightbox).getByAltText("Attachment")).toBeInTheDocument()
      await userEvent.keyboard("{Escape}")
      await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument())

      const removeButtons = screen.getAllByRole("button").filter((b) => b.className.includes("bg-destructive") && b.className.includes("-top-1.5"))
      await userEvent.click(removeButtons[0])
      const confirm = await screen.findByRole("dialog", { name: "Delete Photo" })
      await userEvent.click(within(confirm).getByRole("button", { name: "Delete" }))
      await waitFor(() => expect(success).toHaveBeenCalledWith("Photo deleted"))
      expect(deleted).toBe("a1")
    })
  })
})
