# Phase 1 Research: Foundation and Hardware Probing

**Project:** MonKey (MonkaKeyboard)  
**Phase:** `01-foundation-and-hardware-probing`  
**Scope:** Cargo workspace, synchronous HID transport, dual-interface discovery, report-ID handling, and read-only macOS probing.  
**Research date:** 2026-09-13

## Research boundary

This note covers only the implementation decisions needed to establish `crates/monkey-core`, `crates/monkey-cli`, a synchronous transport seam, safe HID enumeration/opening, and non-destructive probing. LCD rendering, RGB mutation, packet codecs/checksums, flash commits, DFU/ISP behavior, and streaming pacing are Phase 2+ concerns and are intentionally not specified here.

## Project-captured hardware inputs

The following are locked project inputs, copied from the Phase 1 context rather than independently verified by the sources below. They must remain visibly separate from externally verified API behavior:

- Target: VID/PID `0x05AC:0x024F` (`1452:591`).
- Interface A: `(UsagePage, Usage) = (0xFF68, 0x0061)`, maximum input 64 B, maximum output 4096 B, unnumbered Report ID 0.
- Interface B: primary `(UsagePage, Usage) = (0x000C, 0x0001)` with vendor elements on usage page `0xFFFF`; maximum input 16 B, maximum output 1 B, and maximum feature sizes recorded as 1 B/64 B.
- Phase 1 policy: `monkey info` and `monkey probe` are read-only; no flash writes, DFU/ISP triggers, speculative opcodes, or guessed report layouts.

These values are implementation inputs, not proof that every hardware revision exposes the same descriptor. The local decision record is [`01-CONTEXT.md`](./01-CONTEXT.md), especially D-01 through D-12.

## Cargo workspace structure

Cargo defines a workspace as a collection of packages managed together. Members share the workspace-root `Cargo.lock` and default `target` directory, and workspace-wide commands can operate on all members.[1] A virtual workspace has a root `Cargo.toml` containing `[workspace]` but no `[package]`; Cargo requires the resolver to be set explicitly for a virtual workspace.[1]

Use a virtual root manifest for this two-crate project:

```toml
[workspace]
members = ["crates/monkey-core", "crates/monkey-cli"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"

[workspace.dependencies]
hidapi = { version = "2.6.7", features = ["macos-shared-device"] }
```

`resolver = "2"` is the locked project choice; the Cargo requirement that matters here is that a virtual workspace declares a resolver in the root manifest.[1] `monkey-core` should be the library package containing the transport trait, HID implementation, device identity, probing policy, and `MockTransport`; `monkey-cli` should be the binary package with a path dependency on `monkey-core`. This keeps CLI formatting and process concerns out of the reusable driver library.

Declare shared dependency versions and required features in `[workspace.dependencies]`, then inherit them in each member with `workspace = true`. Cargo documents that inherited dependencies may add features in the member, that those features are additive, and that the workspace dependency table itself cannot mark a dependency optional.[1][2] For this phase, the `macos-shared-device` feature belongs in the root `hidapi` declaration so both member manifests consume the same macOS behavior.

## `hidapi` 2.6.x on macOS

The `hidapi` 2.6.7 Rust source also documents this feature as the opt-in shared-access switch.[17] Its crate documentation states that, since hidapi 0.12, macOS devices can be opened with shared access so multiple `HidDevice` handles can access the same physical device; shared access is opt-in for backward compatibility.[3] The 2.6.7 published manifest lists `macos-shared-device` as a feature with no additional dependency.[7]

The Rust crate's `HidDevice` is `Send` but not `Sync` according to its generated API documentation.[5] That supports a single-owner design: create/open the handles in one owner (the Phase 1 CLI can use direct synchronous ownership), and if a future caller needs concurrency, move the transport into one worker thread rather than sharing a `HidDevice` reference across threads. Rust's `Send` contract is the marker for values that may be transferred to another thread; it does not itself serialize operations.[15]

The upstream macOS backend defaults to exclusive opening for backward compatibility. In the backend, `hid_darwin_set_open_exclusive(0)` selects `kIOHIDOptionsTypeNone`, while the exclusive setting selects `kIOHIDOptionsTypeSeizeDevice`.[8] Therefore, enabling `macos-shared-device` is necessary for the project's dual-handle strategy, but it is not a guarantee that every open succeeds: OS permissions, device state, descriptor matching, and disconnects remain runtime conditions.

Open the exact enumerated path, not just VID/PID. `HidApi::open(vid, pid)` uses the first matching device and its documentation gives no guarantee which same-VID/PID device is selected.[6] `DeviceInfo::open_device` uses the device path by default, which is the correct primitive after matching the intended interface.[4]

## HID interface discovery and usage matching

The Rust API exposes `vendor_id`, `product_id`, `path`, `usage_page`, `usage`, and `interface_number` through `DeviceInfo`.[4] `usage_page()` and `usage()` are documented as unavailable on Linux's libusb backend, while the hidraw backend supports them with caveats; this makes the `(UsagePage, Usage)` strategy directly suitable for macOS but requires the project's interface-number fallback on other backends.[3][4]

Apple's HID documentation describes the usage page plus usage number as the unique constant identifying a device type or part of a device. It also identifies `kIOHIDPrimaryUsagePageKey`, `kIOHIDPrimaryUsageKey`, and `kIOHIDElementKey` among the useful HID properties.[12][13] The implementation should therefore match the project tuples exactly, rather than matching on a product string or assuming that the first HID entry is the LCD pipe.

The upstream macOS enumerator first creates a `DeviceInfo` record from the primary usage page/usage, then inspects additional usage pairs and appends records for pairs not equal to the primary pair.[8] Those records originate from the same `IOHIDDevice` and carry the same underlying path-generation context. Consequently, discovery should group candidates by path (or another stable physical-device key), retain all observed usage tuples for validation, and open each required logical interface by its matched path. Do not treat every usage-pair record as proof of a separate physical device.

Recommended discovery sequence:

1. Construct `HidApi` and refresh/index its device list; `device_list()` iterates the currently indexed `DeviceInfo` values.[6]
2. Filter candidates by the locked VID/PID and, where available, product/manufacturer/serial metadata.
3. Match Interface A only on `(usage_page, usage) == (0xFF68, 0x0061)`; match Interface B on the project-captured primary `(0x000C, 0x0001)` while recording any `0xFFFF` vendor elements for later validation.
4. Group duplicate usage-pair records by path, require the expected interface set, and open with `DeviceInfo::open_device` (or `HidApi::open_path`) after the match.[4][6]
5. On non-macOS backends, use `interface_number` only as the documented fallback where usage fields are unavailable; do not silently reinterpret missing usage data as a match.[4]

The project-captured report sizes and exact `0xFFFF` element layout are not established by the generic HID APIs. `info` should report what the live descriptor exposes and mark missing or conflicting attributes as unknown rather than substituting project defaults.

## Report ID 0 handling

The HID 1.11 specification distinguishes a single unnumbered report structure from devices that use Report ID items. If no Report ID item appears in the report descriptor, the device can be treated as having one Input, Output, and Feature report structure; when Report ID items are used, transfers carry a one-byte identifier prefix.[14]

`hidapi` deliberately presents a uniform userspace buffer contract: `write` and `send_feature_report` require the first byte of the buffer to be the Report ID, using `0x00` for a device with a single/un-numbered report; the buffer is therefore one byte longer than the report payload.[5] `get_feature_report` likewise takes the requested ID in the buffer's first byte and returns the ID there, with report data beginning at byte index 1.[9] `read_timeout` is synchronous, takes a millisecond timeout, and reports the actual bytes read.[5]

The macOS backend makes the wire/user-space distinction explicit. It reads the first byte of the supplied buffer as `report_id`; when it is zero, it passes `data + 1` and `length - 1` to `IOHIDDeviceSetReport`, while still passing report ID zero as the API argument.[8] For `IOHIDDeviceGetReport`, it similarly removes the leading zero from the destination passed to IOKit and adds that byte back to the returned length.[8] The transport wrapper must therefore prepend/retain the `0x00` userspace slot for hidapi, while codecs and hardware assertions must not treat that slot as an on-wire payload byte.

Phase 1 tests should cover both paths explicitly: a zero-ID write must record a leading `0x00` byte followed by the payload at the hidapi boundary, and a zero-ID feature read must seed the buffer's first byte with `0x00` and parse returned data from index 1. The live hardware check must still confirm the descriptor's actual report structure before any non-descriptor operation.

## Synchronous transport abstraction

Use the locked synchronous trait in `monkey-core`:

```rust
pub trait Transport: Send {
    fn write_bulk(&mut self, report_id: u8, data: &[u8]) -> Result<usize, TransportError>;
    fn send_feature_report(&mut self, data: &[u8]) -> Result<(), TransportError>;
    fn get_feature_report(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, TransportError>;
    fn read_input_report(
        &mut self,
        buf: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, TransportError>;
}
```

The trait intentionally models the four operations required by Phase 1 and keeps the blocking boundary visible. `hidapi::HidDevice::write`, `send_feature_report`, `get_feature_report`, and `read_timeout` provide the corresponding synchronous operations.[5] `timeout_ms` should normally be finite for CLI probing; reserve `-1` (the hidapi blocking value) for an explicit caller that is prepared to wait indefinitely.[5]

`Transport: Send` is compatible with moving the transport owner to a worker later, while `&mut self` makes the serialization rule explicit at the trait boundary. The trait should not require `Sync` or introduce Tokio in `monkey-core`; a future async UI can bridge to a dedicated owner thread without changing the hardware-facing API. If a shared transport handle is added later, its synchronization must be implemented by the owner/queue, not inferred from `Send`.[15][16]

### Deterministic `MockTransport`

`MockTransport` is a project test double, not a claim about hidapi behavior. Implement it as an in-memory state machine with:

- an ordered `Vec<TransportCall>` recording operation kind, report ID, timeout, and an owned copy of bytes;
- keyed canned feature responses and a FIFO input-report queue;
- explicit injected outcomes (`Ok(n)`, transport error, or empty/timeout result) for each operation;
- fixed response bytes and no wall-clock sleeps, randomness, host-device probing, or background threads;
- inspection/reset methods so tests can assert exact ordering, buffer lengths, report-ID prefixes, and that probing emitted no writes.

The standard library's `mpsc` documentation describes FIFO channels for sending values between threads, but Phase 1 does not need a channel inside the mock; a plain deterministic in-memory model is simpler and keeps unit tests independent of scheduling.[16] If a worker is added later, test the worker separately with the same mock and assert the recorded call sequence.

## Read-only probing and macOS IOHID/App Sandbox access

Apple documents that the HID Manager supports access to devices conforming to the USB HID specification, exposes HID properties, and provides raw-report APIs through the HID Manager/IOKit interfaces.[12] The upstream hidapi macOS backend creates an `IOHIDManager`, applies VID/PID matching when requested, copies the matching device set, and converts each device into `hid_device_info` records.[8] This is the supported access path for Phase 1; raw USB interface claiming is outside this phase.

Implement `monkey info` as descriptor/metadata inspection only: enumerate, show VID/PID, product/manufacturer/serial where available, list matched usage tuples and interface numbers, and report whether A/B are present. Implement `monkey probe` as the same inspection plus only explicitly approved, capture-verified read operations. `get_feature_report` and input reads are read APIs, but the generic HID sources do not define MonKey's firmware query opcode or response layout; no opcode should be invented here.[5][9]

The no-write invariant should be testable: the default probe path must not call `write_bulk` or `send_feature_report`, and any optional read query must be represented by a named capability/profile rather than arbitrary bytes. A timeout, missing interface, permission failure, or malformed response should produce an explicit unavailable/unknown result and stop; it must not trigger retries with guessed report IDs or opcodes.

Apple's current App Sandbox documentation says sandboxing limits access to system resources and user data through entitlements.[10] Apple's `com.apple.security.device.usb` entitlement is a Boolean that allows a sandboxed app to interact with USB devices through USB device access APIs; Apple directs developers to enable App Sandbox and select USB under Hardware.[11] This entitlement is a packaging requirement for a future sandboxed GUI, not evidence that a particular HID collection will open or that a TCC prompt will be absent.

For Phase 1, keep the macOS implementation entitlement-aware but do not promise sandbox success from source inspection alone:

- the standalone CLI can exercise native, unsandboxed IOHID access;
- a future sandboxed app must declare and test `com.apple.security.device.usb`;
- `macos-shared-device` addresses the exclusive/seize choice, while App Sandbox authorization is a separate gate; and
- actual access to both project interfaces, feature reports, input reports, and any TCC behavior must be validated on the target macOS build with the keyboard attached.

## Phase 1 implementation checklist

- [ ] Add virtual root workspace with exactly `crates/monkey-core` and `crates/monkey-cli`; set the locked resolver explicitly.[1]
- [ ] Centralize `hidapi = 2.6.7` and `macos-shared-device` in workspace dependencies.[1][7][18]
- [ ] Define the synchronous `Transport: Send` seam and typed transport errors.[15]
- [ ] Implement `HidapiTransport` using exact enumerated paths and non-exclusive macOS opening.[4][8]
- [ ] Match VID/PID plus exact usage tuples; record duplicate usage-pair records by path.[4][8]
- [ ] Implement explicit hidapi report-ID-0 prefix/offset tests.[8][9]
- [ ] Implement deterministic `MockTransport` with call recording, canned reads, and injected errors.
- [ ] Ensure default `info`/`probe` performs no writes and does not guess opcodes.
- [ ] Run live descriptor/open/read-only validation on macOS before claiming hardware support.

## Unknowns / hardware validation still required

1. Re-read the live report descriptors and confirm the exact Interface A/B tuples, report sizes, report IDs, interface numbers, and whether the two logical records share a physical path on the current macOS release.
2. Confirm that the project-captured Interface B primary usage `(0x000C, 0x0001)` and vendor elements on `0xFFFF` are exposed through hidapi exactly as expected; the generic API does not guarantee this device-specific layout.
3. Confirm both handles open with `macos-shared-device` while the system keyboard driver remains active, and record the actual error behavior for denied, disconnected, or sleeping devices.
4. Identify and validate a benign, capture-verified firmware/version query and its response shape. Until then, descriptor-only probing is the only fully source-grounded behavior; do not guess an opcode.
5. Determine whether the target's input and feature reports are unnumbered or numbered in each interface, then verify the zero-ID userspace prefix versus on-wire payload with a controlled read-only test.
6. Test finite read timeouts and wireless dongle sleep/awake detection on real hardware; generic hidapi documentation does not establish MonKey's heartbeat semantics.
7. Build and run a signed sandboxed macOS test app with `com.apple.security.device.usb`; verify which IOHID operations succeed. The entitlement documentation authorizes USB device access APIs but does not prove every HID operation or resolve unrelated privacy/TCC policy.
8. Verify the exact locked `hidapi` patch version in `Cargo.lock` after workspace creation; this research used the published 2.6.7 crate/API and the corresponding upstream macOS backend behavior.

## Package Legitimacy Audit

> **Required** whenever this phase installs external packages. Run the Package Legitimacy Gate protocol before completing this section.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `hidapi` (2.6.7) | crates.io | 8+ yrs | 5M+ | github.com/ruabmbua/hidapi-rs | [OK] | Approved (with `macos-shared-device`) |
| `zerocopy` (0.8.57) | crates.io | 6+ yrs | 100M+ | github.com/google/zerocopy | [OK] | Approved |
| `thiserror` (2.0.20) | crates.io | 5+ yrs | 200M+ | github.com/dtolnay/thiserror | [OK] | Approved |
| `clap` (4.6.6) | crates.io | 9+ yrs | 250M+ | github.com/clap-rs/clap | [OK] | Approved |
| `anyhow` (1.0.104) | crates.io | 5+ yrs | 300M+ | github.com/dtolnay/anyhow | [OK] | Approved |
| `serde` (1.0.229) | crates.io | 9+ yrs | 500M+ | github.com/serde-rs/serde | [OK] | Approved |
| `serde_json` (1.0.151) | crates.io | 9+ yrs | 400M+ | github.com/serde-rs/json | [OK] | Approved |
| `tracing` (0.1.44) | crates.io | 6+ yrs | 300M+ | github.com/tokio-rs/tracing | [OK] | Approved |
| `tracing-subscriber` (0.3.23) | crates.io | 5+ yrs | 200M+ | github.com/tokio-rs/tracing | [OK] | Approved |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Sources

[1] https://doc.rust-lang.org/cargo/reference/workspaces.html
[2] https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html
[3] https://docs.rs/hidapi/2.6.7/hidapi
[4] https://docs.rs/hidapi/2.6.7/hidapi/struct.DeviceInfo.html
[5] https://docs.rs/hidapi/2.6.7/hidapi/struct.HidDevice.html
[6] https://docs.rs/hidapi/2.6.7/hidapi/struct.HidApi.html
[7] https://crates.io/api/v1/crates/hidapi/2.6.7
[8] https://raw.githubusercontent.com/libusb/hidapi/master/mac/hid.c
[9] https://raw.githubusercontent.com/libusb/hidapi/master/hidapi/hidapi.h
[10] https://developer.apple.com/documentation/security/app-sandbox.md
[11] https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.device.usb.md
[12] https://developer.apple.com/library/archive/documentation/DeviceDrivers/Conceptual/HID/overview/overview.html
[13] https://developer.apple.com/library/archive/documentation/DeviceDrivers/Conceptual/HID/workingwith/workingwith.html
[14] https://usb.org/sites/default/files/hid1_11.pdf
[15] https://doc.rust-lang.org/std/marker/trait.Send.html
[16] https://doc.rust-lang.org/std/sync/mpsc/index.html
[17] https://docs.rs/crate/hidapi/2.6.7/source/src/lib.rs
[18] https://docs.rs/crate/hidapi/2.6.7/source/Cargo.toml
