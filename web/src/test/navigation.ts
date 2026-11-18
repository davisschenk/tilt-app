/**
 * Replace window.location with a stub that records `href` assignments but still
 * reports the real URL for reads, so relative fetches keep resolving.
 * Restored automatically by the global afterEach in setup.ts.
 */
export function mockNavigation() {
  const real = window.location
  const state = { assigned: "" }
  Object.defineProperty(window, "location", {
    configurable: true,
    value: {
      get href() { return real.href },
      set href(v: string) { state.assigned = v },
      origin: real.origin,
      pathname: real.pathname,
      search: real.search,
      hash: real.hash,
      host: real.host,
      hostname: real.hostname,
      protocol: real.protocol,
    },
  })
  return state
}
