import { defineConfig, mergeConfig } from "vitest/config"
import viteConfig from "./vite.config"

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: "jsdom",
      globals: true,
      setupFiles: ["src/test/setup.ts"],
      include: ["src/**/*.test.{ts,tsx}"],
      css: false,
      // Interaction-heavy tests run slower with v8 coverage and many parallel jsdom workers.
      testTimeout: 20_000,
      coverage: {
        provider: "v8",
        reporter: ["text", "json-summary", "lcov", "html"],
        include: ["src/**"],
        exclude: [
          "src/components/ui/**",
          "src/main.tsx",
          "src/types/**",
          "src/test/**",
          "src/**/*.test.{ts,tsx}",
          "**/*.d.ts",
          "src/pages/privacy.tsx",
          "src/pages/terms.tsx",
        ],
        thresholds: { lines: 70, statements: 70, functions: 65, branches: 60 },
      },
    },
  }),
)
