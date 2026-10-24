import { defineConfig, devices } from "@playwright/test";

// End-to-end tests for the workbench (§12 M2 exit criteria). They run against
// the production build served by `vite preview`, so run `npm run build:wasm`
// and `npm run build --workspace @cetus/web` first (CI does).
//
//   npm run test:e2e --workspace @cetus/web
//
// Files are named *.e2e.ts so that Vitest (which picks up *.test.ts and
// *.spec.ts) never tries to run them.

const PORT = 4173;

export default defineConfig({
  testDir: "e2e",
  testMatch: "**/*.e2e.ts",
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL: `http://localhost:${PORT}`,
    locale: "en-US",
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], locale: "en-US" },
    },
  ],
  webServer: {
    command: `npx vite preview --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
