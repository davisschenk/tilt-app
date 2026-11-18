import { execFileSync } from "node:child_process"
import path from "node:path"
import { fileURLToPath } from "node:url"

/** Runs after the server has started (and therefore migrated): wipe + reseed. */
export default function globalSetup() {
  const databaseUrl =
    process.env.E2E_DATABASE_URL ?? "postgres://tilt:password@127.0.0.1:55432/tilt_e2e"
  execFileSync(
    "cargo",
    ["run", "-q", "-p", "server", "--bin", "seed", "--", "--profile", "minimal", "--force"],
    {
      cwd: path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../.."),
      env: { ...process.env, DATABASE_URL: databaseUrl },
      stdio: "inherit",
    },
  )
}
