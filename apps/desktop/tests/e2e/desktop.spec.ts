import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import fs from "node:fs";
import path from "node:path";

const ARTIFACTS_DIR = path.resolve("tests/artifacts/screenshots");

test.beforeAll(() => {
  if (!fs.existsSync(ARTIFACTS_DIR)) {
    fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });
  }
});

test.describe("MonKey Desktop Retro Pixel E2E & Accessibility Suite", () => {
  test("Page 1: Device Discovery, Interfaces, and A11y Audit", async ({ page }) => {
    await page.goto("/");

    // 1. Verify Demo mode banner is persistently visible
    const demoBanner = page.locator(".demo-banner");
    await expect(demoBanner).toBeVisible();
    await expect(demoBanner).toContainText("DEMO");
    await expect(demoBanner).toContainText("NO HARDWARE WRITES WILL BE PERFORMED");

    // 2. Verify Skip Link accessibility
    const skipLink = page.locator(".skip-link");
    await expect(skipLink).toHaveAttribute("href", "#main-content");

    // 3. Verify Device List & Active Device
    await expect(page.locator("h1")).toContainText("Device Identity & Status");
    await expect(
      page.getByRole("button", { name: "Select Monka 3075 Pro (USB Wired)" })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Select Monka 3075 Pro (2.4G Wireless)" })
    ).toBeVisible();

    // Active wired device details
    await expect(page.getByText("Interface A (LCD 0xFF68):")).toBeVisible();
    await expect(page.getByText("Active").first()).toBeVisible();
    await expect(page.getByText("95%")).toBeVisible();
    await expect(page.getByText("Firmware: v1.0.0-hfd")).toBeVisible();

    // Truthful disabled capabilities
    await expect(page.getByText("Key Remap: Unsupported")).toBeVisible();
    await expect(page.getByText("Macro Engine: Unsupported")).toBeVisible();
    await expect(page.getByText("Firmware Flasher: Unsupported")).toBeVisible();

    // Capture Device page screenshot
    await page.screenshot({
      path: path.join(ARTIFACTS_DIR, "01-device-page.png"),
      fullPage: true,
    });

    // Axe Accessibility Check
    const accessibilityScanResults = await new AxeBuilder({ page })
      .disableRules(["color-contrast"])
      .analyze();

    const seriousOrCritical = accessibilityScanResults.violations.filter(
      (v) => v.impact === "serious" || v.impact === "critical"
    );
    expect(seriousOrCritical).toEqual([]);
  });

  test("Page 2: LCD Image/Pattern Streaming, Preview and A11y Audit", async ({ page }) => {
    await page.goto("/");

    // Navigate to LCD Screen tab
    await page.getByRole("navigation", { name: "Main Navigation" }).getByRole("button", { name: "LCD Screen" }).click();
    await expect(page.locator("h1")).toContainText("LCD Display Management");

    // Verify diagnostic test patterns
    const patternSelect = page.locator("#pattern-select");
    await expect(patternSelect).toBeVisible();
    await patternSelect.selectOption("geometry");

    // Test asset chooser in demo mode
    const chooseGifBtn = page.getByRole("button", { name: /Choose Animation \(GIF\)/i });
    await chooseGifBtn.click();

    // Converted preview should appear with 128x128 dimensions and notice
    await expect(page.getByText("cyber_cat_128x128.gif")).toBeVisible();
    await expect(page.getByText("128x128 px")).toBeVisible();
    await expect(page.getByText(/\[Converted Local Preview\]/i)).toBeVisible();

    // Start streaming pattern
    const sendPatternBtn = page.getByRole("button", { name: /Send Pattern to Screen/i });
    await sendPatternBtn.click();

    // Verify active streaming job appears with stop button
    const stopBtn = page.getByRole("button", { name: /Stop Streaming/i });
    await expect(stopBtn).toBeVisible();
    await stopBtn.click();

    // Capture LCD page screenshot
    await page.screenshot({
      path: path.join(ARTIFACTS_DIR, "02-lcd-page.png"),
      fullPage: true,
    });

    // Axe Accessibility Check
    const accessibilityScanResults = await new AxeBuilder({ page })
      .disableRules(["color-contrast"])
      .analyze();

    const seriousOrCritical = accessibilityScanResults.violations.filter(
      (v) => v.impact === "serious" || v.impact === "critical"
    );
    expect(seriousOrCritical).toEqual([]);
  });

  test("Page 3: Lighting Parameters, RAM Preview, Commit, and A11y Audit", async ({ page }) => {
    await page.goto("/");

    // Navigate to RGB Lighting tab
    await page.getByRole("navigation", { name: "Main Navigation" }).getByRole("button", { name: "RGB Lighting" }).click();
    await expect(page.locator("h1")).toContainText("RGB Lighting Configuration");

    // Verify lighting mode selection
    const modeSelect = page.locator("#mode-select");
    await expect(modeSelect).toBeVisible();
    await modeSelect.selectOption("rainbow");

    // Adjust brightness slider
    const brightnessSlider = page.locator("#brightness-slider");
    await brightnessSlider.fill("70");

    // RAM Preview
    const previewBtn = page.getByRole("button", { name: /Preview in RAM/i });
    await previewBtn.click();
    await expect(page.getByText("RAM Preview (Unsaved)")).toBeVisible();

    // Save to Flash Commit
    const commitBtn = page.getByRole("button", { name: /Save to Keyboard Flash/i });
    await commitBtn.click();
    await expect(page.getByText("Verified in Flash")).toBeVisible();

    // Open import dialog and test dialog accessibility
    const importBtn = page.getByRole("button", { name: /Import Profile \(JSON\)/i });
    await importBtn.click();

    const dialog = page.locator("div[role='dialog']");
    await expect(dialog).toBeVisible();

    const cancelDialogBtn = dialog.getByRole("button", { name: /Cancel/i });
    await cancelDialogBtn.click();
    await expect(dialog).not.toBeVisible();

    // Capture Lighting page screenshot
    await page.screenshot({
      path: path.join(ARTIFACTS_DIR, "03-lighting-page.png"),
      fullPage: true,
    });

    // Axe Accessibility Check
    const accessibilityScanResults = await new AxeBuilder({ page })
      .disableRules(["color-contrast"])
      .analyze();

    const seriousOrCritical = accessibilityScanResults.violations.filter(
      (v) => v.impact === "serious" || v.impact === "critical"
    );
    expect(seriousOrCritical).toEqual([]);
  });

  test("Page 4: Diagnostics Doctor, CLI Guidance, and A11y Audit", async ({ page }) => {
    await page.goto("/");

    // Navigate to Diagnostics tab
    await page.getByRole("navigation", { name: "Main Navigation" }).getByRole("button", { name: "Diagnostics" }).click();
    await expect(page.locator("h1")).toContainText("Hardware Doctor & Diagnostics");

    // Verify doctor checks rendered
    await expect(page.getByText("Interface A (LCD Bulk Display Pipe)")).toBeVisible();
    await expect(page.getByText("Interface B (Config & Feature Reports)")).toBeVisible();
    await expect(page.getByText("Flash Write Rails & Safety Gate")).toBeVisible();

    // Verify CLI Guidance commands
    await expect(page.getByText("monkey bench --fps 15")).toBeVisible();
    await expect(page.getByText("monkey info --json")).toBeVisible();

    // Test Copy Sanitized Report button
    const copyReportBtn = page.getByRole("button", { name: /Copy Sanitized Report/i });
    await expect(copyReportBtn).toBeVisible();
    await copyReportBtn.click();

    // Capture Diagnostics screenshot
    await page.screenshot({
      path: path.join(ARTIFACTS_DIR, "04-diagnostics-page.png"),
      fullPage: true,
    });

    // Axe Accessibility Check
    const accessibilityScanResults = await new AxeBuilder({ page })
      .disableRules(["color-contrast"])
      .analyze();

    const seriousOrCritical = accessibilityScanResults.violations.filter(
      (v) => v.impact === "serious" || v.impact === "critical"
    );
    expect(seriousOrCritical).toEqual([]);
  });

  test("Responsive Compact View (<900px) and Stacked Layout (<760px)", async ({ page }) => {
    // Test compact nav at 800px width
    await page.setViewportSize({ width: 800, height: 600 });
    await page.goto("/");
    await expect(page.locator(".sidebar-nav")).toBeVisible();

    // Test stacked layout at 700px width (simulating narrow / 200% zoom)
    await page.setViewportSize({ width: 700, height: 560 });
    await expect(page.locator(".main-body")).toBeVisible();

    // Capture narrow layout screenshot
    await page.screenshot({
      path: path.join(ARTIFACTS_DIR, "05-narrow-layout.png"),
      fullPage: true,
    });
  });

  test("Failure States: Inactive Interface B, Low Battery Warning, and Error Panels", async ({ page }) => {
    await page.goto("/");

    // Select wireless device which has 18% battery and inactive Interface B
    const selectWirelessBtn = page.getByRole("button", {
      name: "Select Monka 3075 Pro (2.4G Wireless)",
    });
    await selectWirelessBtn.click();
    await expect(page.getByText("18%")).toBeVisible();

    // Capture device page in low-battery/partial interface state
    await page.screenshot({
      path: path.join(ARTIFACTS_DIR, "06-failure-low-battery.png"),
      fullPage: true,
    });

    // Navigate to Lighting page where Interface B is inactive
    await page
      .getByRole("navigation", { name: "Main Navigation" })
      .getByRole("button", { name: "RGB Lighting" })
      .click();

    await expect(page.getByText("Interface B Inactive")).toBeVisible();
    await expect(
      page.getByText(
        "Interface B (0xFFFF:0x0001) is inactive on this device. Lighting control requires USB connection or valid macOS shared-device access."
      )
    ).toBeVisible();

    // Verify lighting controls are disabled
    const modeSelect = page.locator("#mode-select");
    await expect(modeSelect).toBeDisabled();

    // Capture failure state screenshot for inactive interface
    await page.screenshot({
      path: path.join(ARTIFACTS_DIR, "07-failure-interface-inactive.png"),
      fullPage: true,
    });

    // Axe Accessibility Check on Failure State
    const accessibilityScanResults = await new AxeBuilder({ page })
      .disableRules(["color-contrast"])
      .analyze();

    const seriousOrCritical = accessibilityScanResults.violations.filter(
      (v) => v.impact === "serious" || v.impact === "critical"
    );
    expect(seriousOrCritical).toEqual([]);
  });

  test("Accessibility: Keyboard Navigation and Reduced Motion", async ({ page }) => {
    // Emulate reduced motion
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/");

    // Skip Link keyboard interaction
    await page.keyboard.press("Tab");
    const skipLink = page.locator(".skip-link");
    await expect(skipLink).toBeFocused();
    await page.keyboard.press("Enter");

    // Main content focus target
    const mainContent = page.locator("#main-content");
    await expect(mainContent).toBeFocused();

    // Navigate to RGB Lighting tab
    const rgbNavBtn = page
      .getByRole("navigation", { name: "Main Navigation" })
      .getByRole("button", { name: "RGB Lighting" });
    await rgbNavBtn.click();
    await expect(page.locator("h1")).toContainText("RGB Lighting Configuration");

    // Open import dialog
    const importBtn = page.getByRole("button", { name: /Import Profile \(JSON\)/i });
    await importBtn.click();

    const dialog = page.locator("div[role='dialog']");
    await expect(dialog).toBeVisible();

    // Cancel dialog via keyboard Escape
    await page.keyboard.press("Escape");
    await expect(dialog).not.toBeVisible();
  });
});
