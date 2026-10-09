import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  testMatch: "browser.spec.ts",
  fullyParallel: false,
  workers: 1,
  reporter: [["list"], ["json", { outputFile: "_verification/browser.json" }]],
  use: { baseURL: "http://127.0.0.1:4173", headless: true },
  webServer: {
    command: "node scripts/serve.mjs",
    url: "http://127.0.0.1:4173",
    reuseExistingServer: false,
  },
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "firefox", use: { browserName: "firefox" } },
    { name: "webkit", use: { browserName: "webkit" } },
  ],
});
