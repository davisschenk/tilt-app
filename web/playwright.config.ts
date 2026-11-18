import { defineConfig, devices } from "@playwright/test"

const PORT = Number(process.env.E2E_PORT ?? 8123)
const DATABASE_URL =
  process.env.E2E_DATABASE_URL ?? "postgres://tilt:password@127.0.0.1:55432/tilt_e2e"

/**
 * E2E runs against the real stack: Rocket (AUTH_MODE=disabled) serving the built
 * web/dist, backed by Postgres. Global setup reseeds the DB before every run.
 * Needs: a reachable Postgres at E2E_DATABASE_URL and `npm run build` done first.
 */
export default defineConfig({
  testDir: "e2e",
  globalSetup: "./e2e/global-setup.ts",
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? [["html", { open: "never" }], ["github"]] : [["list"]],
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "cargo run -p server",
    cwd: "..",
    url: `http://127.0.0.1:${PORT}/api/v1/health`,
    reuseExistingServer: !process.env.CI,
    timeout: 600_000,
    env: {
      DATABASE_URL,
      ROCKET_ADDRESS: "127.0.0.1",
      ROCKET_PORT: String(PORT),
      // Test-only key (32 random bytes, base64) — never used outside this throwaway E2E DB.
      ROCKET_SECRET_KEY: "x7vV0o3mQ0bJm2kqz9sPq1t8nYw4cH5rL6dE2aF8uXg=",
      AUTH_MODE: "disabled",
      AUTH_DISABLED_ACK: "true",
      WEB_DIST_DIR: "web/dist",
      UPLOAD_DIR: "/tmp/tilt-e2e-uploads",
    },
  },
})
