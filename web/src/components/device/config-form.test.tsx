import { describe, expect, it, vi } from "vitest"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { useState } from "react"
import { defaultValues, type ConfigFormValues } from "@/lib/device-config"
import ConfigForm from "./config-form"

function Harness(props: Partial<React.ComponentProps<typeof ConfigForm>> & { onValues?: (v: ConfigFormValues) => void }) {
  const [values, setValues] = useState<ConfigFormValues>({ ...defaultValues(null, "AA:BB:CC:DD"), serverUrl: "http://192.168.1.5:8000" })
  return (
    <ConfigForm
      values={values}
      onChange={(v) => { setValues(v); props.onValues?.(v) }}
      networks={[]}
      scanning={false}
      reconfigure={false}
      error={null}
      submitting={false}
      onSubmit={() => {}}
      {...props}
    />
  )
}

describe("ConfigForm", () => {
  it("edits fields through onChange", async () => {
    const seen: ConfigFormValues[] = []
    render(<Harness onValues={(v) => seen.push(v)} />)
    await userEvent.type(screen.getByLabelText("WiFi network"), "A")
    expect(seen.at(-1)?.wifiSsid).toBe("A")
    await userEvent.type(screen.getByLabelText("WiFi password"), "p")
    expect(seen.at(-1)?.wifiPass).toBe("p")
    await userEvent.click(screen.getByRole("switch"))
    expect(seen.at(-1)?.newApiKey).toBe(false)
  })

  it("submits without a page reload", async () => {
    const onSubmit = vi.fn()
    render(<Harness onSubmit={onSubmit} />)
    await userEvent.click(screen.getByRole("button", { name: /Save/ }))
    expect(onSubmit).toHaveBeenCalledOnce()
  })

  it("lists scanned networks and wires up Scan", async () => {
    const onScan = vi.fn()
    const { container } = render(<Harness networks={[{ ssid: "Home", rssi: -50, auth: "wpa2" }]} onScan={onScan} />)
    expect(container.querySelector("datalist option")).toHaveAttribute("value", "Home")
    await userEvent.click(screen.getByRole("button", { name: /Scan/ }))
    expect(onScan).toHaveBeenCalled()
  })

  it("hides Scan without a handler and disables it while scanning", () => {
    const { rerender } = render(<Harness />)
    expect(screen.queryByRole("button", { name: /Scan/ })).not.toBeInTheDocument()
    rerender(<Harness onScan={() => {}} scanning />)
    expect(screen.getByRole("button", { name: /Scan/ })).toBeDisabled()
  })

  it("warns about loopback server URLs", async () => {
    render(<Harness />)
    expect(screen.queryByText(/can't reach "localhost"/)).not.toBeInTheDocument()
    const url = screen.getByLabelText("Server URL")
    await userEvent.clear(url)
    await userEvent.type(url, "http://localhost:8000")
    expect(screen.getByText(/can't reach "localhost"/)).toBeInTheDocument()
  })

  it("reveals advanced settings", async () => {
    const seen: ConfigFormValues[] = []
    render(<Harness onValues={(v) => seen.push(v)} />)
    expect(screen.queryByLabelText("Scan interval (s)")).not.toBeInTheDocument()
    await userEvent.click(screen.getByText(/Show advanced settings/))
    const scan = screen.getByLabelText("Scan interval (s)")
    await userEvent.clear(scan)
    await userEvent.type(scan, "30")
    expect(seen.at(-1)?.scanIntervalSecs).toBe(30)
    await userEvent.click(screen.getByText(/Hide advanced settings/))
  })

  it("shows errors and busy state, and adapts the password hint when reconfiguring", () => {
    render(<Harness error="Bad SSID" submitting reconfigure />)
    expect(screen.getByText("Bad SSID")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: /Save/ })).toBeDisabled()
    expect(screen.getByPlaceholderText("Leave blank to keep current")).toBeInTheDocument()
  })
})
