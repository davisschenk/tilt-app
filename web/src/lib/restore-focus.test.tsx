import { describe, expect, it } from "vitest"
import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { useState } from "react"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import { AlertDialog, AlertDialogContent, AlertDialogTitle, AlertDialogCancel } from "@/components/ui/alert-dialog"

function Controlled({ alert = false }: { alert?: boolean }) {
  const [open, setOpen] = useState(false)
  return (
    <>
      <button onClick={() => setOpen(true)}>opener</button>
      {alert ? (
        <AlertDialog open={open} onOpenChange={setOpen}>
          <AlertDialogContent>
            <AlertDialogTitle>t</AlertDialogTitle>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
          </AlertDialogContent>
        </AlertDialog>
      ) : (
        <Dialog open={open} onOpenChange={setOpen}>
          <DialogContent>
            <DialogTitle>t</DialogTitle>
          </DialogContent>
        </Dialog>
      )}
    </>
  )
}

describe("focus restoration", () => {
  it("returns focus to the opener when a controlled Dialog closes", async () => {
    render(<Controlled />)
    const opener = screen.getByRole("button", { name: "opener" })
    await userEvent.click(opener)
    await screen.findByRole("dialog")
    await userEvent.keyboard("{Escape}")
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument())
    expect(opener).toHaveFocus()
  })

  it("returns focus to the opener when a controlled AlertDialog closes", async () => {
    render(<Controlled alert />)
    const opener = screen.getByRole("button", { name: "opener" })
    await userEvent.click(opener)
    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }))
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument())
    expect(opener).toHaveFocus()
  })
})
