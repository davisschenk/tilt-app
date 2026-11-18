/* eslint-disable react-refresh/only-export-components -- test helpers */
import type { ReactElement } from "react"
import { render } from "@testing-library/react"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { MemoryRouter } from "react-router-dom"
import { AuthProvider } from "@/lib/auth"
import { ThemeProvider } from "@/components/theme-provider"

export function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
}

export function Providers({
  children,
  route = "/",
  client = makeQueryClient(),
}: {
  children: React.ReactNode
  route?: string
  client?: QueryClient
}) {
  return (
    <QueryClientProvider client={client}>
      <ThemeProvider>
        <MemoryRouter initialEntries={[route]}>
          <AuthProvider>{children}</AuthProvider>
        </MemoryRouter>
      </ThemeProvider>
    </QueryClientProvider>
  )
}

export function renderWithProviders(ui: ReactElement, opts: { route?: string } = {}) {
  const client = makeQueryClient()
  return {
    client,
    ...render(
      <Providers route={opts.route} client={client}>
        {ui}
      </Providers>,
    ),
  }
}
