import { beforeEach, describe, expect, it, vi } from "vitest"
import { screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { Route, Routes } from "react-router-dom"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { alertRule, brew, hydrometer } from "@/test/fixtures"
import EditBrewDialog from "./brew/edit-brew-dialog"
import DeleteBrewDialog from "./brew/delete-brew-dialog"
import EditHydrometerDialog from "./hydrometer/edit-hydrometer-dialog"
import DeleteHydrometerDialog from "./hydrometer/delete-hydrometer-dialog"
import RegisterHydrometerDialog from "./hydrometer/register-hydrometer-dialog"
import EditAlertRuleDialog from "./alerts/edit-alert-rule-dialog"
import DeleteAlertRuleDialog from "./alerts/delete-alert-rule-dialog"

const { success, error } = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }))
vi.mock("@/lib/toast", () => ({ success, error, info: vi.fn() }))

beforeEach(() => { success.mockClear(); error.mockClear() })
const noop = () => {}

describe("EditBrewDialog", () => {
  it("prefills, submits parsed values, and closes", async () => {
    let body: Record<string, unknown> = {}
    server.use(http.put("/api/v1/brews/b1", async ({ request }) => ((body = (await request.json()) as Record<string, unknown>), HttpResponse.json(brew()))))
    const onOpenChange = vi.fn()
    renderWithProviders(<EditBrewDialog brew={brew({ notes: "n" })} open onOpenChange={onOpenChange} />)
    expect(screen.getByLabelText("Name")).toHaveValue("Test IPA")
    expect(screen.getByLabelText("OG")).toHaveValue(1.06)
    await userEvent.clear(screen.getByLabelText("Name"))
    await userEvent.type(screen.getByLabelText("Name"), " Renamed ")
    await userEvent.type(screen.getByLabelText("FG"), "1.012")
    await userEvent.clear(screen.getByLabelText("Style"))
    await userEvent.click(screen.getByLabelText("Status"))
    await userEvent.click(await screen.findByRole("option", { name: "Completed" }))
    await userEvent.click(screen.getByRole("button", { name: /Save|Update/ }))
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(body).toMatchObject({ name: "Renamed", style: null, og: 1.06, fg: 1.012, targetFg: 1.012, notes: "n", status: "Completed" })
    expect(success).toHaveBeenCalledWith("Brew updated")
  })

  it("reports errors and cancel closes", async () => {
    server.use(http.put("/api/v1/brews/b1", () => new HttpResponse(null, { status: 500 })))
    const onOpenChange = vi.fn()
    renderWithProviders(<EditBrewDialog brew={brew()} open onOpenChange={onOpenChange} />)
    await userEvent.click(screen.getByRole("button", { name: /Save|Update/ }))
    await waitFor(() => expect(error).toHaveBeenCalledWith("Failed to update brew"))
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }))
    expect(onOpenChange).toHaveBeenCalledWith(false)
  })
})

describe("DeleteBrewDialog", () => {
  it("deletes and navigates to the list", async () => {
    server.use(http.delete("/api/v1/brews/b1", () => new HttpResponse(null, { status: 204 })))
    renderWithProviders(
      <Routes>
        <Route path="/" element={<DeleteBrewDialog brewId="b1" brewName="Test IPA" open onOpenChange={noop} />} />
        <Route path="/brews" element={<p>list page</p>} />
      </Routes>,
    )
    expect(screen.getByText('Delete "Test IPA"?')).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", { name: "Delete" }))
    expect(await screen.findByText("list page")).toBeInTheDocument()
    expect(success).toHaveBeenCalledWith("Brew deleted")
  })

  it("reports failure", async () => {
    server.use(http.delete("/api/v1/brews/b1", () => new HttpResponse(null, { status: 500 })))
    renderWithProviders(<DeleteBrewDialog brewId="b1" brewName="X" open onOpenChange={noop} />)
    await userEvent.click(screen.getByRole("button", { name: "Delete" }))
    await waitFor(() => expect(error).toHaveBeenCalledWith("Failed to delete brew"))
  })
})

describe("EditHydrometerDialog", () => {
  it("submits offsets and name", async () => {
    let body: Record<string, unknown> = {}
    server.use(http.put("/api/v1/hydrometers/h1", async ({ request }) => ((body = (await request.json()) as Record<string, unknown>), HttpResponse.json(hydrometer()))))
    const onOpenChange = vi.fn()
    renderWithProviders(<EditHydrometerDialog hydrometer={hydrometer()} open onOpenChange={onOpenChange} />)
    expect(screen.getByLabelText("Name / Alias")).toHaveValue("Kitchen Tilt")
    await userEvent.clear(screen.getByLabelText("Name / Alias"))
    await userEvent.clear(screen.getByLabelText("Temperature Offset (°F)"))
    await userEvent.type(screen.getByLabelText("Temperature Offset (°F)"), "-1.5")
    await userEvent.clear(screen.getByLabelText(/^Gravity Offset/))
    await userEvent.type(screen.getByLabelText(/^Gravity Offset/), "0.002")
    await userEvent.click(screen.getByRole("button", { name: /Save|Update/ }))
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(body).toEqual({ name: null, tempOffsetF: -1.5, gravityOffset: 0.002 })
  })

  it("reports failure", async () => {
    server.use(http.put("/api/v1/hydrometers/h1", () => new HttpResponse(null, { status: 500 })))
    renderWithProviders(<EditHydrometerDialog hydrometer={hydrometer()} open onOpenChange={noop} />)
    await userEvent.click(screen.getByRole("button", { name: /Save|Update/ }))
    await waitFor(() => expect(error).toHaveBeenCalledWith("Failed to update hydrometer"))
  })
})

describe("DeleteHydrometerDialog", () => {
  it("shows color and alias, deletes", async () => {
    server.use(http.delete("/api/v1/hydrometers/h1", () => new HttpResponse(null, { status: 204 })))
    const onOpenChange = vi.fn()
    renderWithProviders(<DeleteHydrometerDialog hydrometerId="h1" hydrometerColor="Red" hydrometerName="Kitchen" hasActiveBrew={false} open onOpenChange={onOpenChange} />)
    expect(screen.getByText('Delete "Red (Kitchen)" hydrometer?')).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", { name: "Delete" }))
    await waitFor(() => expect(success).toHaveBeenCalledWith("Hydrometer deleted"))
  })

  it("warns about an active brew and surfaces failure", async () => {
    server.use(http.delete("/api/v1/hydrometers/h1", () => new HttpResponse(null, { status: 409 })))
    renderWithProviders(<DeleteHydrometerDialog hydrometerId="h1" hydrometerColor="Red" hasActiveBrew open onOpenChange={noop} />)
    expect(screen.getByText(/active brew/i)).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", { name: "Delete" }))
    await waitFor(() => expect(error).toHaveBeenCalledWith("Failed to delete hydrometer"))
  })
})

describe("RegisterHydrometerDialog", () => {
  it("excludes colors already registered, validates, and registers", async () => {
    let body: Record<string, unknown> = {}
    server.use(
      http.get("/api/v1/hydrometers", () => HttpResponse.json([hydrometer({ color: "Red" })])),
      http.post("/api/v1/hydrometers", async ({ request }) => ((body = (await request.json()) as Record<string, unknown>), HttpResponse.json(hydrometer()))),
    )
    const onOpenChange = vi.fn()
    renderWithProviders(<RegisterHydrometerDialog open onOpenChange={onOpenChange} />)
    await userEvent.click(screen.getByRole("button", { name: "Register" }))
    expect(await screen.findByText("Color is required")).toBeInTheDocument()
    await userEvent.click(screen.getByRole("combobox"))
    expect(screen.queryByRole("option", { name: /Red/ })).not.toBeInTheDocument()
    await userEvent.click(await screen.findByRole("option", { name: /Blue/ }))
    await userEvent.type(screen.getByLabelText("Name / Alias"), " Fridge ")
    await userEvent.click(screen.getByRole("button", { name: "Register" }))
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(body).toEqual({ color: "Blue", name: "Fridge" })
  })

  it("reports failure", async () => {
    server.use(
      http.get("/api/v1/hydrometers", () => HttpResponse.json([])),
      http.post("/api/v1/hydrometers", () => new HttpResponse(null, { status: 500 })),
    )
    renderWithProviders(<RegisterHydrometerDialog open onOpenChange={noop} />)
    await userEvent.click(screen.getByRole("combobox"))
    await userEvent.click(await screen.findByRole("option", { name: /Red/ }))
    await userEvent.click(screen.getByRole("button", { name: "Register" }))
    await waitFor(() => expect(error).toHaveBeenCalledWith("Failed to register hydrometer"))
  })
})

describe("alert rule edit/delete dialogs", () => {
  it("edits a rule", async () => {
    let body: Record<string, unknown> = {}
    server.use(
      http.get("/api/v1/alert-targets", () => HttpResponse.json([])),
      http.get("/api/v1/brews", () => HttpResponse.json([])),
      http.get("/api/v1/hydrometers", () => HttpResponse.json([])),
      http.put("/api/v1/alert-rules/r1", async ({ request }) => ((body = (await request.json()) as Record<string, unknown>), HttpResponse.json(alertRule()))),
    )
    const onOpenChange = vi.fn()
    renderWithProviders(<EditAlertRuleDialog rule={alertRule()} open onOpenChange={onOpenChange} />)
    const name = screen.getByLabelText("Name *")
    await userEvent.clear(name)
    await userEvent.type(name, "Renamed")
    await userEvent.click(screen.getByRole("button", { name: /Save|Update/ }))
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(body).toMatchObject({ name: "Renamed", metric: "gravity", threshold: 1.01 })
  })

  it("validates an empty name when editing", async () => {
    server.use(
      http.get("/api/v1/alert-targets", () => HttpResponse.json([])),
      http.get("/api/v1/brews", () => HttpResponse.json([])),
      http.get("/api/v1/hydrometers", () => HttpResponse.json([])),
    )
    renderWithProviders(<EditAlertRuleDialog rule={alertRule()} open onOpenChange={noop} />)
    await userEvent.clear(screen.getByLabelText("Name *"))
    await userEvent.click(screen.getByRole("button", { name: /Save|Update/ }))
    expect(await screen.findByText("Name is required")).toBeInTheDocument()
  })

  it("deletes a rule and reports failure", async () => {
    let status = 500
    server.use(http.delete("/api/v1/alert-rules/r1", () => new HttpResponse(null, { status })))
    renderWithProviders(<DeleteAlertRuleDialog ruleId="r1" ruleName="FG" open onOpenChange={noop} />)
    await userEvent.click(screen.getByRole("button", { name: "Delete" }))
    await waitFor(() => expect(error).toHaveBeenCalled())
    status = 204
    await userEvent.click(screen.getByRole("button", { name: "Delete" }))
    await waitFor(() => expect(success).toHaveBeenCalled())
  })
})
