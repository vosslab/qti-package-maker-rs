import { defineConfig } from "@playwright/test";

// Transitional migration evidence.  The permanent browser suite deliberately
// has no dependency on the sibling Python checkout.
export default defineConfig({
  testDir: "./tests",
  testMatch: "selftest_python_parity.spec.ts",
  fullyParallel: false,
  workers: 1,
  reporter: [["list"], ["json", { outputFile: "_verification/python-selftest-parity.json" }]],
  use: { headless: true },
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "firefox", use: { browserName: "firefox" } },
    { name: "webkit", use: { browserName: "webkit" } },
  ],
});
