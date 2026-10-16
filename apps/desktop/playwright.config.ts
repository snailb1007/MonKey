import { defineConfig, devices } from "@playwright/test";

if (!process.env.PLAYWRIGHT_BROWSERS_PATH) {
  process.env.PLAYWRIGHT_BROWSERS_PATH = "/Volumes/D/.playwright-browsers";
}

export default defineConfig({
  testDir: "./tests/e2e",
  outputDir: "./tests/artifacts/test-results",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: [["list"]],
  use: {
    baseURL: "http://127.0.0.1:1420",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1040, height: 720 },
        headless: true,
        launchOptions: {
          args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-gpu", "--disable-dev-shm-usage"],
        },
      },
    },
  ],
  webServer: {
    command: "VITE_MONKEY_DEMO=1 pnpm dev",
    url: "http://127.0.0.1:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 30000,
  },
});
