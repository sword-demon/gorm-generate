import { defineConfig, devices } from "@playwright/test"

// Browser-side end-to-end tests: the real App.vue in a real browser, with the
// Tauri IPC boundary stubbed (see e2e/app.spec.ts). The Rust core has its own
// cargo tests; this covers what only a browser shows — boot, generation
// wiring, settings persistence and the error paths.
// Same port as vite.config.ts (`server.port`).
const appUrl = "http://localhost:1420"

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["github"], ["list"]] : "list",
  timeout: 30_000,
  use: {
    baseURL: appUrl,
    trace: "on-first-retry",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "npm run dev",
    url: appUrl,
    // A dev server already running matches the config Tauri uses.
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
})
