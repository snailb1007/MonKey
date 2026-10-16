# MonKey Desktop (Retro Pixel UI v2.0)

A lightweight, accessible retro-pixel desktop application for the Monka 3075 Pro mechanical keyboard (and compatible HFD/RKGK890 OEM devices). Built with Tauri v2, React 18, TypeScript, and Vite.

## Toolchain & Versions

- **Node.js**: `v22.23.1` (or compatible Node 20+)
- **pnpm**: `12.11.2` (pinned via `packageManager` in `package.json`)
- **Rust**: `1.80+` (Edition 2021)
- **Tauri**: `v2.2`

## Architecture & IPC Design

MonKey Desktop strictly decouples the UI from direct hardware access:
1. **Frontend**: React 18 + TypeScript + Vite + plain CSS styling. Zero external CDN, telemetry, or heavy UI megaframeworks.
2. **IPC Transport**: All communication routes through a single strongly-typed command `desktop_request({ request })`. Actions are snake_case (`list_devices`, `probe`, `diagnostics`, `pick_asset`, `start_lcd`, `stop_lcd`, `job_status`, `rgb_status`, `rgb_preview`, `rgb_commit`, `export_profile`, `import_profile`).
3. **Safety Rails**:
   - Zero blind flash writes.
   - Live lighting changes go to RAM preview (throttled <= 30Hz).
   - Flash commits require explicit user action and are debounced (500ms) with readback verification.
   - Wireless devices below 20% battery reject flash commits (`low_battery` error code).
   - Unsupported OEM features (key remap, macro engine, firmware flasher) are visibly disabled with clear explanations.

## Physical Device vs Demo Mode Behavior

### Physical Device Mode (Production Tauri App)
When launched via Tauri (`pnpm desktop:build` or `tauri dev`):
- Communicates directly with the Monka 3075 Pro USB HID interfaces:
  - **Interface A (`0xFF68:0x0061`)**: Standalone vendor collection for bulk LCD frame streaming (4096-byte OUT reports).
  - **Interface B (`0xFFFF:0x0001`)**: Shared composite interface for 64-byte feature reports (RGB lighting configuration).
- No demo banner is displayed.
- Hardware writes are verified against capability matrices.

### Demo / Browser Mode (Local Development & E2E Tests)
When launched in a standard browser (`pnpm dev`) or during Playwright E2E tests (`VITE_MONKEY_DEMO=1` or window lacking Tauri internals):
- An explicit, permanent banner is displayed: `[DEMO MODE: SIMULATED HARDWARE - NO HARDWARE WRITES WILL BE PERFORMED]`.
- Provides simulated wired (`dev-demo-wired`) and wireless (`dev-demo-wireless`) devices.
- Accurately simulates interface constraints, battery levels, asset conversions (128x128), RAM preview, flash commit, and error codes (`low_battery`, `invalid_input`).
- Ensures automated accessibility and UI tests can run reliably without physical hardware connected.

## Installation & Setup

```bash
# Install dependencies using pinned frozen lockfile
pnpm --dir apps/desktop install --frozen-lockfile

# Provision Playwright browsers for E2E tests (first-time setup)
pnpm --dir apps/desktop exec playwright install chromium
```

## Available Scripts

| Command | Description |
|---------|-------------|
| `pnpm --dir apps/desktop run dev` | Start Vite dev server on `127.0.0.1:1420` (strict port) |
| `pnpm --dir apps/desktop run typecheck` | Run TypeScript type checking (`tsc --noEmit`) |
| `pnpm --dir apps/desktop run lint` | Run ESLint static analysis (`eslint .`) |
| `pnpm --dir apps/desktop run test:unit` | Run Vitest unit & contract tests |
| `pnpm --dir apps/desktop run build` | Compile TypeScript and bundle frontend with Vite |
| `pnpm --dir apps/desktop run test:e2e` | Run Playwright E2E & `@axe-core/playwright` accessibility tests |
| `pnpm --dir apps/desktop run desktop:build` | Build native Tauri desktop binary (`tauri build --no-bundle`) |

## Accessibility & Design Standards

- Conforms to **WCAG 2.2 AA** standards with automated axe-core audits on all pages.
- Pixel-aligned palette: Paper `#F4F0E6`, Panel `#FFFCF4`, Ink `#20251F`, Accent `#315B3A`, Caution `#775000`, Danger `#9A2929`.
- Full keyboard navigation with skip-to-content links, visible focus outlines, and ESC key dismissal for dialogs.
- Reduced motion support via `@media (prefers-reduced-motion: reduce)`.
- Responsive layout supporting compact (<900px) and stacked (<760px / 200% zoom) viewports.
