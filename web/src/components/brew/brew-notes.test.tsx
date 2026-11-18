import { describe, expect, it, vi } from "vitest"
import { screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { http, HttpResponse } from "msw"
import { server } from "@/test/msw/server"
import { renderWithProviders } from "@/test/render"
import { brew } from "@/test/fixtures"
import BrewNotes from "./brew-notes"

vi.mock("@uiw/react-md-editor", () => {
  const MDEditor = ({ value, onChange }: { value: string; onChange: (v?: string) => void }) => (
    <textarea aria-label="editor" value={value} onChange={(e) => onChange(e.target.value)} />
  )
  MDEditor.Markdown = ({ source }: { source: string }) => <div data-testid="md">{source}</div>
  return { default: MDEditor }
})

describe("BrewNotes", () => {
  it("renders existing notes as markdown", () => {
    renderWithProviders(<BrewNotes brewId="b1" notes="**hi**" />)
    expect(screen.getByTestId("md")).toHaveTextContent("**hi**")
  })

  it("prompts to add notes when empty, edits and saves trimmed text", async () => {
    let body: unknown
    server.use(http.put("/api/v1/brews/b1", async ({ request }) => ((body = await request.json()), HttpResponse.json(brew()))))
    renderWithProviders(<BrewNotes brewId="b1" notes={null} />)
    await userEvent.click(screen.getByText("Click to add notes..."))
    await userEvent.type(screen.getByLabelText("editor"), "  dry hop day 3  ")
    await userEvent.click(screen.getByRole("button", { name: /Save/ }))
    await waitFor(() => expect(body).toEqual({ notes: "dry hop day 3" }))
    await waitFor(() => expect(screen.queryByLabelText("editor")).not.toBeInTheDocument())
  })

  it("cancel discards the draft", async () => {
    renderWithProviders(<BrewNotes brewId="b1" notes="orig" />)
    await userEvent.click(screen.getAllByRole("button")[0])
    await userEvent.type(screen.getByLabelText("editor"), " more")
    await userEvent.click(screen.getByRole("button", { name: /Cancel/ }))
    expect(screen.getByTestId("md")).toHaveTextContent("orig")
  })

  it("stays in edit mode when saving fails", async () => {
    server.use(http.put("/api/v1/brews/b1", () => new HttpResponse(null, { status: 500 })))
    renderWithProviders(<BrewNotes brewId="b1" notes="orig" />)
    await userEvent.click(screen.getAllByRole("button")[0])
    await userEvent.click(screen.getByRole("button", { name: /Save/ }))
    await waitFor(() => expect(screen.getByRole("button", { name: /Save/ })).toBeEnabled())
    expect(screen.getByLabelText("editor")).toBeInTheDocument()
  })
})
