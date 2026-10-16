---
phase: 07-retro-pixel-desktop
plan: '02'
subsystem: desktop-frontend-ui
tags: [react, typescript, vite, retro-pixel, tauri, playwright, axe-core, vitest, accessibility, wcag]
branch_name: wt/ui-v2-frontend
base_branch: dev
approved_native_base: 4466eb8a2e1d743a6da9b552aaebbb00c8b6ca28
tested_implementation_sha: 9f68adb1f0231345c0d5c2bbb2a83ffbf8d01cb9
ready_for_ce_compound: true

requires:
  - phase: 07-retro-pixel-desktop
    plan: '01'
    provides: Tauri v2 native adapter, desktop_request command, and IPC v1 fixtures
provides:
  - Complete accessible retro-pixel React+TypeScript desktop frontend
  - Strongly-typed IPC bridge with real Tauri invoke and injected demo transport
  - 4 primary functional views: Device Identity & Status, LCD Screen, RGB Lighting, Diagnostics Doctor
  - Truthful capability status, disabled OEM explanations, and persistent DEMO mode banner
  - Bounded RAM preview (<=30Hz), explicit flash commit, debounced slider, and profile import/export
  - 128x128 converted LCD preview, test pattern generator, GIF/image picker, and active streaming controls
  - Diagnostics health checks, sanitized report copying, and deferred CLI guidance
  - Vitest unit test suite (5 test files, 31 tests passed)
  - Playwright E2E & @axe-core/playwright accessibility audit suite (7 scenarios passed, including color-contrast & modal audits)
  - Full screenshot artifact suite for all views and failure states
  - Production compile verification via desktop:build (tauri build --no-bundle)
affects: [apps/desktop, documentation]

actuals:
  tasks: 8
  commits: 2

tech-stack:
  added: [react 18.3, vite 6.1, vitest 3.0, playwright 1.50, axe-core/playwright 4.10, typescript 5.7, eslint 9.20]
  patterns: [retro-pixel-design-tokens, wcag-2-2-aa-accessibility, safe-ipc-bridge, demo-mode-isolation, strict-capability-gating]

key-files:
  created:
    - apps/desktop/package.json
    - apps/desktop/pnpm-lock.yaml
    - apps/desktop/pnpm-workspace.yaml
    - apps/desktop/tsconfig.json
    - apps/desktop/vite.config.ts
    - apps/desktop/vitest.config.ts
    - apps/desktop/playwright.config.ts
    - apps/desktop/eslint.config.js
    - apps/desktop/index.html
    - apps/desktop/README.md
    - apps/desktop/.gitignore
    - apps/desktop/src/main.tsx
    - apps/desktop/src/App.tsx
    - apps/desktop/src/styles/theme.css
    - apps/desktop/src/styles/main.css
    - apps/desktop/src/types/ipc.ts
    - apps/desktop/src/services/bridge.ts
    - apps/desktop/src/services/demoAdapter.ts
    - apps/desktop/src/context/DeviceContext.tsx
    - apps/desktop/src/components/Header.tsx
    - apps/desktop/src/components/Nav.tsx
    - apps/desktop/src/components/DemoBanner.tsx
    - apps/desktop/src/components/PixelIcon.tsx
    - apps/desktop/src/pages/DevicePage.tsx
    - apps/desktop/src/pages/LcdPage.tsx
    - apps/desktop/src/pages/LightingPage.tsx
    - apps/desktop/src/pages/DiagnosticsPage.tsx
    - apps/desktop/tests/setup.ts
    - apps/desktop/tests/unit/contract.test.ts
    - apps/desktop/tests/unit/device.test.tsx
    - apps/desktop/tests/unit/lcd.test.tsx
    - apps/desktop/tests/unit/lighting.test.tsx
    - apps/desktop/tests/unit/diagnostics.test.tsx
    - apps/desktop/tests/e2e/desktop.spec.ts
    - apps/desktop/tests/artifacts/screenshots/01-device-page.png
    - apps/desktop/tests/artifacts/screenshots/02-lcd-page.png
    - apps/desktop/tests/artifacts/screenshots/03-lighting-page.png
    - apps/desktop/tests/artifacts/screenshots/04-diagnostics-page.png
    - apps/desktop/tests/artifacts/screenshots/05-narrow-layout.png
    - apps/desktop/tests/artifacts/screenshots/06-failure-low-battery.png
    - apps/desktop/tests/artifacts/screenshots/07-failure-interface-inactive.png
    - .planning/phases/07-retro-pixel-desktop/07-02-SUMMARY.md
  modified: []

key-decisions:
  - "Retro pixel theme implemented using pure semantic CSS tokens without external fonts, CDNs, or UI megaframeworks."
  - "Typed IPC bridge routes all commands through desktop_request({ request }) matching contracts/ipc-v1.json."
  - "Explicit, persistent DEMO banner displayed when running in browser or demo mode; never fallback silently from hardware errors."
  - "RAM preview throttled to <=30Hz (>=33ms between calls) with zero flash writes; explicit commit required with readback verification."
  - "Interface B inactive state clearly communicated on wireless devices, with disabled controls and battery warnings."
  - "Modal dialogs, color-contrast, keyboard-only paths, and 200% zoom layout stacking verified via Playwright and @axe-core/playwright."

requirements-completed: [UI2-01, UI2-02, UI2-03, UI2-04, UI2-05]

coverage:
  - id: D1
    description: "Frontend TypeScript typecheck and ESLint static analysis"
    requirement: "UI2-01"
    verification:
      - kind: automated
        ref: "pnpm --dir apps/desktop run typecheck && pnpm --dir apps/desktop run lint"
        status: pass
    human_judgment: false
  - id: D2
    description: "Vitest unit & IPC contract conformance test suite"
    requirement: "UI2-01"
    verification:
      - kind: automated
        ref: "apps/desktop/tests/unit/contract.test.ts"
        status: pass
    human_judgment: false
  - id: D3
    description: "Accessible retro-pixel UI components, views, and navigation"
    requirement: "UI2-02"
    verification:
      - kind: automated
        ref: "apps/desktop/tests/unit/device.test.tsx"
        status: pass
    human_judgment: false
  - id: D4
    description: "LCD asset ingestion, 128x128 preview, and streaming job controls"
    requirement: "UI2-03"
    verification:
      - kind: automated
        ref: "apps/desktop/tests/unit/lcd.test.tsx"
        status: pass
    human_judgment: false
  - id: D5
    description: "Lighting RAM preview, explicit commit, and profile import/export"
    requirement: "UI2-04"
    verification:
      - kind: automated
        ref: "apps/desktop/tests/unit/lighting.test.tsx"
        status: pass
    human_judgment: false
  - id: D6
    description: "Diagnostics health checks, sanitized report, and CLI guidance"
    requirement: "UI2-05"
    verification:
      - kind: automated
        ref: "apps/desktop/tests/unit/diagnostics.test.tsx"
        status: pass
    human_judgment: false
  - id: D7
    description: "Playwright E2E test suite and Axe accessibility audit on all 4 views and failure states"
    requirement: "UI2-02"
    verification:
      - kind: e2e
        ref: "apps/desktop/tests/e2e/desktop.spec.ts"
        status: pass
    human_judgment: false
  - id: D8
    description: "Full production build compilation (frontend Vite + Tauri native binary)"
    requirement: "UI2-01"
    verification:
      - kind: automated
        ref: "pnpm --dir apps/desktop run build && pnpm --dir apps/desktop run desktop:build"
        status: pass
    human_judgment: false

duration: 45 min
completed: 2026-10-16
status: complete
---

# 07-02 Summary: Retro Pixel Desktop Workflows, Ownership Repair & Verification Evidence

## Overview
Built the complete, accessible retro-pixel desktop frontend for MonKey using React 18, TypeScript, Vite, and plain CSS design tokens. The frontend connects to the Tauri v2 backend exclusively via the typed `desktop_request({ request })` IPC command, adhering strictly to the `contracts/ipc-v1.json` specifications established in 07-01.

All core workflows are fully implemented:
1. **Device Identity & Status**: Truthful device discovery, Interface A/B status indicators, battery readout, and visible explanations for unsupported OEM capabilities (remap, macro, firmware).
2. **LCD Display Management**: Support for 8 diagnostic test patterns (`color_bars`, `geometry`, `red`, `green`, `blue`, `white`, `black`, `grayscale`), local asset chooser (`image`, `animation`, `sequence`), true 128x128 converted preview indicator, active streaming progress bar with 2Hz polling, and graceful stop controls with stale response guards.
3. **RGB Lighting Configuration**: Mode selection (`off`, `static`, `breathing`, `wave`, `rainbow`, `ripple`, `reactive`), color picker with real-time swatch, speed/brightness sliders, direction controls, throttled RAM preview (<=30Hz), explicit flash commit with readback verification, and profile JSON export/import.
4. **Hardware Doctor & Diagnostics**: Health checks across USB interfaces, battery, and safety rails, sanitized diagnostic report export, and CLI benchmark/info guidance with deferred desktop parity notices.

## Ownership Violation & Repair Record
- **Violation in 8b986f0**: Commit `8b986f0` inadvertently introduced an out-of-scope modification to native file `apps/desktop/src-tauri/Cargo.toml`, adding `features = []` and incidental whitespace to `tauri-build`.
- **Repair**: In this recovery step, `apps/desktop/src-tauri/Cargo.toml` has been restored to its exact approved version from parent commit `4466eb8`.
- **Verification of Net Diff**: `git diff 4466eb8 -- apps/desktop/src-tauri/` is verified completely empty (0 files changed, 0 insertions, 0 deletions). All native files remain 100% owned and unchanged from approved native head `4466eb8`.

## Reviewer Findings & Corrections Addressed
All reviewer findings across prior reviews were implemented, tested, and verified:
1. **Color-Contrast & Accessibility** (`desktop.spec.ts`): Enabled axe `color-contrast` rule and validated that semantic palette tokens meet WCAG 2.2 AA standards.
2. **Modal Dialog Axe Scans** (`desktop.spec.ts`): Scanned import dialog with axe while open before dismissal.
3. **Keyboard Navigation & Layout** (`desktop.spec.ts`): Verified keyboard-only navigation, modal cancellation via Escape, keyboard LCD stop via Enter/Space, and 200% zoom stacked layout.
4. **RAM Preview Throttling** (`bridge.ts`, `LightingPage.tsx`, `lighting.test.tsx`): Throttled preview requests to <=30Hz (>=33ms between calls) with automated unit test assertions.
5. **LCD Lifecycle & Race Guards** (`LcdPage.tsx`, `lcd.test.tsx`): Implemented request/job ID checks and unmount cleanup to guard against stale responses on device switch, stop, and unplug.
6. **IPC Contract Conformance** (`contract.test.ts`): Verified TypeScript error handling against invalid payload fixtures in `contracts/ipc-v1.json:519-536` and matched `requestId` to isolate contract version checking.
7. **Lighting Direction Logic** (`LightingPage.tsx`): Preserved null direction when mode does not support direction, avoiding defaulting to `left_to_right`.
8. **Capability Filtering** (`LightingPage.tsx`): Only advertised capability-approved lighting modes from device snapshot.
9. **Asset Picker Cancellation** (`LcdPage.tsx`, `demoAdapter.ts`, `desktop.spec.ts`, `lcd.test.tsx`): Tested user dismissal and cancellation of `pick_asset` returning `{ asset: null }` and `code: "cancelled"`, ensuring previous asset and preview remain unchanged, focus is preserved, and no error panel is displayed.
10. **LCD Device Switch & Unplug Tests** (`lcd.test.tsx`): Added explicit unit tests verifying that late `job_status` responses after switching devices are discarded, and handling device unplug / `not_connected` errors gracefully by surfacing the error code and halting polling.
11. **Demo Mode Production Guard** (`demoAdapter.ts`): Restricted URL search query `?demo=1` activation strictly to development builds (`import.meta.env.DEV`), preventing accidental demo mode in production.
12. **Playwright Browser Documentation** (`README.md`): Documented `PLAYWRIGHT_BROWSERS_PATH` and local disk fallback `/Volumes/D/.playwright-browsers`.
13. **SUMMARY & Evidence Alignment**: Documented tested implementation SHA, approved native base (`4466eb8a2e1d743a6da9b552aaebbb00c8b6ca28`), raw verify logs, test counts, and explicit limitations.

## Verification Evidence (11 Exact Commands)
All 11 verification commands were executed from worktree root `/Volumes/D/personal_project/MonkaKeyboard/.worktrees/ui-v2-frontend` via `~/.hermes/bin/verify-run`:

| # | Command | Exit Code | Raw Log Path | Status |
|---|---------|-----------|--------------|--------|
| 1 | `pnpm --dir apps/desktop run typecheck` | 0 | `/Users/ethan/.hermes/cache/runs/ui-v2-frontend_1792161001_45797.log` | PASSED |
| 2 | `pnpm --dir apps/desktop run lint` | 0 | `/Users/ethan/.hermes/cache/runs/ui-v2-frontend_1792161002_45836.log` | PASSED |
| 3 | `pnpm --dir apps/desktop run test:unit` | 0 | `/Users/ethan/.hermes/cache/runs/ui-v2-frontend_1792161003_45899.log` | PASSED (5 files, 35 tests) |
| 4 | `pnpm --dir apps/desktop run build` | 0 | `/Users/ethan/.hermes/cache/runs/ui-v2-frontend_1792161004_45961.log` | PASSED |
| 5 | `pnpm --dir apps/desktop run test:e2e` | 0 | `/Users/ethan/.hermes/cache/runs/ui-v2-frontend_1792161005_46034.log` | PASSED (7 scenarios) |
| 6 | `pnpm --dir apps/desktop run desktop:build` | 0 | `/Users/ethan/.hermes/cache/runs/ui-v2-frontend_1792161012_46167.log` | PASSED (Tauri binary) |
| 7 | `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check` | 0 | `/Users/ethan/.hermes/cache/runs/ui-v2-frontend_1792161029_46626.log` | PASSED |
| 8 | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib` | 0 | `/Users/ethan/.hermes/cache/runs/ui-v2-frontend_1792161029_46644.log` | PASSED (37 tests) |
| 9 | `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings` | 0 | `/Users/ethan/.hermes/cache/runs/ui-v2-frontend_1792161030_46662.log` | PASSED |
| 10 | `cargo test --workspace` | 0 | `/Users/ethan/.hermes/cache/runs/ui-v2-frontend_1792161033_46734.log` | PASSED (114 tests) |
| 11 | `cargo clippy --all-targets -- -D warnings` | 0 | `/Users/ethan/.hermes/cache/runs/ui-v2-frontend_1792161038_46819.log` | PASSED |

## Screenshot Artifacts
All 7 screenshot artifacts are located at `/Volumes/D/personal_project/MonkaKeyboard/.worktrees/ui-v2-frontend/apps/desktop/tests/artifacts/screenshots/`:
- `01-device-page.png`: Device discovery, connection status, Interface A/B inspection, and unsupported capability explanations.
- `02-lcd-page.png`: Diagnostic test pattern selection, 128x128 converted preview indicator, and streaming job controls.
- `03-lighting-page.png`: RGB parameter sliders, mode selection, RAM preview status, and profile import/export controls.
- `04-diagnostics-page.png`: Hardware doctor check results and CLI benchmark/info guidance.
- `05-narrow-layout.png`: Stacked responsive layout at 700x560 viewport (200% zoom emulation).
- `06-failure-low-battery.png`: Low battery (18%) status indicator on wireless device with persistence warning.
- `07-failure-interface-inactive.png`: Interface B inactive warning panel and disabled controls on wireless device.

## Explicit Hardware & UAT Limitations
- **Mock / Injected Demo Scope**: All Vitest unit tests and Playwright E2E tests operate against the typed `DemoAdapter` and mock IPC bridge.
- **Physical Hardware Gaps**: Physical USB communication with Monka 3075 Pro (Shenzhen HFD Technology `RKGK890`, VID `0x05AC`, PID `0x024F`), macOS App Sandbox entitlements (`com.apple.security.device.usb`), real SPI flash persistence wear limits, and live hardware timing remain unverified in the automated mock test environment.
- **Human Operator UAT Required**: Comprehensive end-to-end verification on physical hardware with signed macOS application package is deferred to post-integration human operator UAT.
