/* eslint-disable react-refresh/only-export-components -- test mock records its latest props for assertions */
import { forwardRef, useImperativeHandle, type CSSProperties } from "react"
import { vi } from "vitest"

type EChartsProps = {
  option: Record<string, unknown>
  onEvents?: Record<string, (params: unknown) => void>
  style?: CSSProperties
}

/** The most recently rendered chart's props, so tests can assert on `option` and fire chart events. */
export const chart: { props: EChartsProps | null; instance: ReturnType<typeof makeInstance> } = {
  props: null,
  instance: makeInstance(),
}

function makeInstance() {
  return {
    resize: vi.fn(),
    setOption: vi.fn(),
    convertFromPixel: vi.fn<(...a: unknown[]) => unknown>(() => null),
  }
}

export function resetChart() {
  chart.props = null
  chart.instance = makeInstance()
}

export function series(name: string) {
  const all = (chart.props?.option.series ?? []) as { name: string; data: unknown[]; markLine?: { data: { name?: string }[] } }[]
  return all.find((s) => s.name === name)
}

/** jsdom has no canvas, so render a placeholder that mirrors the option for inspection. */
const MockECharts = forwardRef<unknown, EChartsProps>(function MockECharts(props, ref) {
  chart.props = props
  useImperativeHandle(ref, () => ({ getEchartsInstance: () => chart.instance }))
  return <div data-testid="echart" style={props.style} />
})

export default MockECharts
