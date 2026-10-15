# Phase 07 — MonKey desktop UI v2.0

## Authority and scope
User: redesign from old UI, minimal retro pixel. Operator authorization on t_033482d4 permits inline planning without external GSD/delegation. Research: Scout t_be53c466. This is a new desktop UI over existing v1 core, not a new hardware protocol or a rewrite. macOS first; Linux/Windows architecture-compatible, not certified by this phase. Signing, notarization, App Store/sandbox certification, daemon, RTC, audio visualization and updates are deferred.

## Functional parity / boundaries
| OEM/workflow | Desktop treatment | Evidence / guard |
|---|---|---|
| Device identity, connection, interfaces | Device page; refresh, explicit probe; show A/B separately, sleep/permission/unknown states | core device.rs, InterfacePolicy; never infer both endpoints from VID/PID alone |
| LCD images, animations, patterns | LCD page; choose image/GIF/sequence directory, converted preview, explicit send/start/stop, progress | existing core conversion/streaming; 128x128 RGB565, 8x4096; supported formats only |
| RGB modes, brightness, speed, color/direction | Lighting page; only modes/ranges supported by core | RgbManager / LightingConfig; RAM preview separate from persistent save |
| RGB profiles/status | export/import local profile, validate, preview, explicit save-to-device with readback | distinguish local file export from hardware persistence |
| Environment diagnostics | Diagnostics page: doctor checks, remediation and copy sanitized report; no automatic benchmark writes | run_doctor_checks; hardware bench remains CLI in this phase, documented route |
| Remap/Fn, macros | visible read-only capabilities section marked unavailable, reason text | unverified protocol; no command or enabled control |
| Firmware/ISP, erase/reset, music visualization | explanation in capability reference only, no executable route | prohibited/unverified; never reproduce OEM destructive actions |
| Mouse DPI/speed/report tabs | omitted as unrelated generic OEM assets | vendor skins are not keyboard capabilities |
| CLI completions, synthetic/transport benchmarks | linked CLI help in Diagnostics, not fake GUI parity | explicit deferred desktop workflows |

Scout source pointers: vendor_driver/language/1033.lan:64–125 (keys), 176–208 (macro), 228–234 and 600–606 (firmware), 246–263 (audio); vendor_driver/skins/theme1/icon/tab_mouse*.png (generic mouse); research/lcd_and_macro_engine.md; docs/dev/protocol-spec.md. Do not copy proprietary vendor binaries/art into the new app.

## Design contract
- Stack: Tauri v2, React + TypeScript, Vite, plain CSS custom properties; pnpm with committed lock and exact packageManager version selected from available supported tooling. No UI framework, remote fonts, telemetry, updater, CDN or network permission.
- Information architecture: persistent left nav Device / LCD / Lighting / Diagnostics. Header has selected device and textual connection + A/B badges. Main content has one primary action. Unsupported capabilities live on Device rather than empty top-level tabs.
- Default window 1040x720, minimum 760x560; below 900px nav becomes compact labeled navigation, not unlabeled icons. At 200% zoom and narrow width use vertical stacking and scroll, no clipped controls. Preserve platform titlebar and macOS window controls.
- Palette: warm paper #F4F0E6, panel #FFFCF4, ink #20251F, muted #525A50, border #72796A, accent #315B3A, caution #775000, danger #9A2929. Validate actual contrast; adjust if thresholds fail. Dark mode is deferred, not a second unfinished theme.
- Pixel vocabulary: 4px spacing unit; 1px hard borders; square corners; small stepped/pixel glyphs drawn locally, no scanlines, gradients, fake CRT blur or decorative motion. One restrained 2px hard shadow for selected panels only.
- Typography: readable local system sans body 14–16px, line-height 1.5, full Vietnamese glyph fallback; local system monospace for short labels/numbers. Pixel character comes from geometry/icons, not unreadable bitmap paragraphs. No new font licensing dependency.
- Target WCAG 2.2 AA: semantic headings/forms/buttons, labels and inline validation, 3px visible focus, text/icon status beyond color; 44px practical interaction targets, no keyboard traps, logical tab order, skip link, aria-live polite job updates (not every frame), focus restoration after dialog/stop. Respect reduced-motion. Toasts never the only error record.
- LCD preview displays actual converted pixels with image-rendering: pixelated at integer sizes 128 or 256px, with text identifying dimensions/format. If image cannot be sent, preview never implies success. Native file picker plus optional drag/drop only if safe scoped handling is implemented.

## Screen/state acceptance
Device: empty with reconnect instructions; loading; multiple-device selection; disconnected/asleep; A-only/B-only; permission-denied; ready; probe unknown values render 'Unknown', never guessed. Refresh/probe performs no configuration write. Device changes invalidate old jobs and asset sessions.
LCD: empty picker; selected/converted asset; unsupported/corrupt/oversized input; preview; queued/running/completed/cancelled/failed. Start requires A and any B prerequisite required by existing InterfacePolicy; backend is authority. Disable duplicate start. Stop and window-close cooperate with core cleanup; stale progress cannot revive stopped jobs.
Lighting: current readback or explicit unknown, accessible labeled mode/color/range controls, unsaved RAM state, Save to device confirmation. Slider changes coalesce to at most 30Hz RAM previews; never auto-flash. Explicit persistent intent must still honor core 500ms debounce/read-before-write/readback and wireless <20% gate. Unknown wireless battery blocks persistence. Export file is not save-to-device. Import validates schema then previews; never writes on picker/import alone.
Diagnostics: run explicit checks, grouped pass/warn/fail/unknown, remediation; sanitized copy omits serials/device paths/file paths by default. No request for Input Monitoring on startup merely to send feature reports. No 'all OK' until checks complete.
Every screen has loading, error + recovery, disabled-reason and demo/no-hardware state. Demo is explicit opt-in with persistent 'DEMO — no hardware writes' banner; never silently fall back to mock after hardware failure.

## Architecture / shared IPC contract v1 (locked before fan-out)
Directories: `apps/desktop/src-tauri/` owns standalone nested Cargo workspace and all native configuration; do not modify root Rust workspace or core/CLI. `apps/desktop/contracts/ipc-v1.json` is backend-owned versioned cross-language request/result fixtures. Frontend owns all remaining `apps/desktop/` files. Native local assets stay inside frontend public directory, not OEM skins.

Only native invoke command: `desktop_request({ request })`.
Request shape: `{version:1, requestId:string, deviceId:string|null, action:Action, payload:object}`. requestId is bounded nonempty, deviceId is backend-issued opaque session ID, never an arbitrary HID path. Reject unknown fields/actions and malformed payloads in native layer. Native serde uses camelCase fields, snake_case action tags. Response: `{version:1, requestId:string, ok:true, data:object}` or `{version:1, requestId:string, ok:false, error:{code:ErrorCode,message:string,retryable:boolean}}`. No raw HID/FS/CLI command invocation.

Action payloads/results:
- `list_devices {}` -> `{devices:DeviceSnapshot[]}`.
- `probe {}` -> `{device:DeviceSnapshot}` (requires selected device).
- `diagnostics {}` -> `{checks:[{id,label,status:'pass'|'warn'|'fail'|'unknown',detail,remediation:string|null}]}`.
- `pick_asset {kind:'image'|'animation'|'sequence'}` -> `{asset:AssetPreview|null}`; native chooser grants scoped session asset ID. Cancel is null, not error. Animation covers GIF, sequence covers explicit directory.
- `start_lcd {assetId:string|null,pattern:'red'|'green'|'blue'|'white'|'black'|'grayscale'|'color_bars'|'geometry'|null}` -> `{jobId:string}`; exactly one of asset/pattern, core validates formats. `color_bars` maps to existing TestPatternType::RgbBars; other tags map one-to-one to the current native enum. Expose all built-in patterns, not only two. Animation pacing remains core-controlled.
- `stop_lcd {jobId:string}` -> `{stopped:boolean}`; idempotent, cleanup acknowledged before next writer.
- `job_status {jobId:string}` -> `{job:JobSnapshot}`; frontend polls at most 4Hz only while active; no custom progress events in v1.
- `rgb_status {}` -> `{config:LightingConfigDto|null,persistence:'unknown'|'ram'|'verified',supportedModes:string[]}`.
- `rgb_preview {config:LightingConfigDto}` -> `{config:LightingConfigDto,persistence:'ram'}`; cannot commit.
- `rgb_commit {}` -> `{persistence:'verified'}`; commits last accepted preview only; backend must track per-device intent/config and reject if absent/stale, concurrent or unsafe. No UI boolean bypass of battery or whitelist.
- `export_profile {}` -> `{saved:boolean}`; native save chooser and validated current readback, no arbitrary path from JS.
- `import_profile {}` -> `{config:LightingConfigDto|null}`; native open chooser + validation only; UI submits separate preview if user accepts.

DeviceSnapshot: `{id,label,connection:'wired'|'wireless'|'asleep'|'disconnected'|'unknown',interfaces:{lcd:boolean,config:boolean},capabilities:{lcd:boolean,rgb:boolean,persist:boolean,remap:false,macro:false,firmware:false},batteryPercent:number|null,firmware:string|null}`. Null is unknown; capabilities derived from actual core policy/readback, no guess. LightingConfigDto: `{mode:string,color:{r:number,g:number,b:number},brightness:number,speed:number,direction:string|null}`. Backend maps only supported core enums and exact native ranges: mode `off|static|breathing|wave|rainbow|ripple|reactive`; r/g/b integers 0..255; brightness/speed integers 0..100; direction `left_to_right|right_to_left`. Verify against rgb/mode.rs:8–16 and :156–199; fixtures expose only capability-approved modes. Null direction is unknown readback only and is rejected for rgb_preview (no implicit write default). AssetPreview: `{id,name,width:128,height:128,frameCount:number,previewDataUrl:string}` using converted RGB565 round-trip PNG preview, no paths. JobSnapshot: `{id,deviceId,state:'queued'|'running'|'completed'|'cancelled'|'failed',framesSent:number,error:{code:ErrorCode,message:string,retryable:boolean}|null}`. Poll responses include their requestId; frontend ignores stale request/device results.
ErrorCode: `not_connected|permission_denied|unsupported|busy|unsafe_operation|low_battery|invalid_input|io_error|cancelled|internal`. Native messages sanitized; no secrets, device paths or serials in ordinary logs.

Native layer owns one serialized hardware worker and backpressure. No HID handles in JS or async UI executor; reuse MonkaDevice, InterfacePolicy, SafeTransport and existing safety/timing APIs. LCD/RGB cross-operation races return busy or queue under one owner, never separate independent handles. Bounded input: image file <=32 MiB, <=40M decoded pixels, animation <=300 frames and <=64 MiB decoded RGB565 budget; reject early, use bounded/streamed decoding if needed. Sequence must remain under chosen directory with no symlink escape. Native picker capabilities limited to app commands; no generic filesystem/shell/network permissions, no remote content. CSP self-only with narrowly required Tauri IPC, data/blob only for preview. Backend validates regardless of frontend gating.

## Verification / delivery boundary
Mock/contract tests do not prove physical parity, timing, App Sandbox access or production OS compatibility. Require human hardware smoke after integration: read-only discovery/probe/doctor first; then opt-in LCD static/animation/stop and RGB RAM preview; separately consented persistent save/readback on healthy power. Capture result as UAT, never fabricate it. No GUI-triggered write during automated tests.
Two-way-door UI packaging and IPC are internal/unpublished; high safety risk is native write exposure, mitigated by backend deny tests, existing guard reuse and independent review. Rollback removes apps/desktop without driver/CLI changes; stop active jobs before quitting/uninstalling. No core capability expansion allowed.
