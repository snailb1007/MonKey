---
phase: 01-foundation-and-hardware-probing
reviewed: 2026-09-13
reviewed_revision: a69f6e8fd110c60bc106139e8c6ffeaa81ff2423
review_mode: inline
depth: standard
files_reviewed: 23
files_reviewed_list:
  - Cargo.toml
  - crates/monkey-core/Cargo.toml
  - crates/monkey-core/src/lib.rs
  - crates/monkey-core/src/error.rs
  - crates/monkey-core/src/device.rs
  - crates/monkey-core/src/transport/mod.rs
  - crates/monkey-core/src/transport/mock.rs
  - crates/monkey-core/src/transport/hid.rs
  - crates/monkey-core/src/protocol/mod.rs
  - crates/monkey-core/src/protocol/types.rs
  - crates/monkey-cli/Cargo.toml
  - crates/monkey-cli/src/main.rs
  - crates/monkey-cli/src/output.rs
  - crates/monkey-cli/src/commands/mod.rs
  - crates/monkey-cli/src/commands/info.rs
  - crates/monkey-cli/src/commands/probe.rs
  - crates/monkey-cli/src/lib.rs
  - crates/monkey-core/tests/device_discovery_test.rs
  - crates/monkey-core/tests/hid_transport_test.rs
  - crates/monkey-core/tests/mock_transport_test.rs
  - crates/monkey-core/tests/protocol_types_test.rs
  - crates/monkey-cli/tests/cli_probe_test.rs
  - Cargo.lock
findings:
  critical: 0
  warning: 4
  info: 0
  total: 4
status: issues_found
---

# Phase 01: Code Review Report

## Review hiện tại — 2026-09-13, revision a69f6e8

**Kết luận: chưa đủ căn cứ chấp nhận Phase 1 về độ chính xác của probe và nhận diện thiết bị.** Build/test sạch nhưng còn bốn lỗi chức năng. Mức Warning bên dưới không có nghĩa là nên bỏ qua trước UAT; hai lỗi P1 ảnh hưởng trực tiếp giá trị cốt lõi. Không có bằng chứng về brick, flash write hoặc hỏng phần cứng trong lần review này.

Phạm vi: workspace manifests, core/CLI source và các test Phase 1. Review thực hiện inline theo adapter của gsd-code-review. Những thay đổi planning có sẵn được giữ nguyên. Không chạy probe trên phần cứng và không đánh dấu UAT passed.

### WR-05 [P1]: Firmware và hardware revision là hằng số, không được đọc từ thiết bị

**Vị trí:** `crates/monkey-cli/src/commands/probe.rs:45-59`, `:88-92`.

`probe_device_with_transport` gọi get-feature nhưng không parse `probe_buf`; mọi phản hồi đều dẫn đến `rev1.0` và `v1.0.0`. Ngay cả phản hồi 64 byte `0xAB` cũng cho ra các phiên bản này trong harness chạy trên thư viện hiện tại. Nhánh descriptor-only cũng trả cùng hằng số. Vì vậy DISC-03 chưa được đáp ứng: người dùng không thể phân biệt firmware/revision khác nhau, và dữ liệu này không thể làm đầu vào đáng tin cho capability negotiation.

Test `cli_probe_test.rs:99-114` assert chính những hằng số đó, không kiểm tra phiên bản được decode từ dữ liệu. Hướng sửa: biểu diễn giá trị chưa biết bằng null/unknown kèm nguồn dữ liệu; chỉ điền phiên bản khi có response format đã xác minh. Không đoán opcode để hoàn thiện probe. Bổ sung fixture phiên bản khác nhau, dữ liệu không hợp lệ và trường hợp không đọc được.

### WR-06 [P1]: Ghép interface theo khoảng cách registry ID có thể ghép chéo hai thiết bị

**Vị trí:** `crates/monkey-core/src/device.rs:173-182` và vòng chọn set đầu tiên trong `group_monka_devices`.

Điều kiện `abs_diff <= 32` được dùng làm bằng chứng cùng thiết bị khi thiếu serial. Với hai thiết bị giả lập K1=A1000/B1007 và K2=A1010/B1017, thứ tự enumerate A1000, A1010, B1017, B1007 cho kết quả A1000/B1017 và A1010/B1007. Đây là tái hiện bằng implementation Rust thực tế, không phải chỉ nhận định từ comment. Test cũ chỉ dùng hai cụm ID rất xa nhau nên không phát hiện.

Hậu quả hiện tại: info/probe tổng hợp và mở endpoint của thiết bị khác với identity của set. Rủi ro cho các phase ghi dữ liệu sau này là routing A/B sang hai bàn phím khác nhau; chưa tái hiện ghi sai phần cứng. Hướng sửa: chỉ ghép khi có identity vật lý xác minh được; nếu không, giữ set chưa đầy đủ và báo ambiguity. Test các ID gần nhau và permutation thứ tự enumerate.

### WR-07 [P2]: Nhánh không mở được transport vẫn công bố WiredUsb

**Vị trí:** `crates/monkey-cli/src/commands/probe.rs:80-96`, `:172-183`.

Khi open Interface B lỗi hoặc không có Interface B, CLI chuyển sang descriptor-only, đặt `transport_state = "WiredUsb"` và trả thành công. Metadata descriptor không chứng minh active connection state. Harness gọi trực tiếp fallback không có thiết bị cũng cho WiredUsb. Trường hợp permission/open failure vì vậy bị biến thành kết quả kết nối có dây trong stdout JSON, dù stderr có warning.

Hướng sửa: giữ descriptor inspection nhưng biểu diễn trạng thái unknown/unavailable và lý do/provenance có cấu trúc. Xác định rõ exit-code contract cho probe thất bại; không thay lỗi đọc bằng trạng thái thành công. Test nhánh open failure và thiếu B, không chỉ test helper nhận Transport.

### WR-08 [P2]: WirelessSleeping không thể được sinh từ đường get-feature thật hiện tại

**Vị trí:** `crates/monkey-core/src/transport/hid.rs:109-125`, `:147-149`; `crates/monkey-core/src/error.rs:31-34`.

State evaluator chỉ trả WirelessSleeping khi nhận `TransportError::Timeout`. Nhưng probe gọi `get_feature_report`, và mọi lỗi hidapi từ đường này được chuyển thành `TransportError::HidError(String)`. Nhánh biến zero-byte thành Timeout chỉ tồn tại trong `read_input_report`, không được probe gọi. Test sleeping inject sẵn Timeout vào mock nên không chứng minh đường runtime có thể đạt trạng thái này. Comment “finite timeout (50ms)” cũng không tương ứng với tham số/deadline nào tại chỗ gọi.

Ngoài ra evaluator coi mọi `Ok(_)`, kể cả `Ok(0)`, là awake/connected; harness xác nhận `evaluate_state_query(true, Ok(0)) == Ok(WirelessAwake)`. Hướng sửa: định nghĩa heartbeat/read protocol và error classification được kiểm chứng, kiểm tra độ dài/nội dung response, biểu diễn unknown khi không đủ bằng chứng. Không đổi mọi HID error thành sleeping và không tự thêm speculative writes. Test xuyên adapter lỗi, không chỉ enum injection. Chưa đo thời gian chặn hay sleep/awake trên thiết bị thật trong review này.

## Validation hiện tại

- `rtk cargo test --workspace`: **45 passed**, 10 suites.
- `rtk cargo clippy --workspace --all-targets -- -D warnings`: **pass**.
- `rtk cargo build --workspace`: **pass**.
- Harness `/private/tmp/monkey_phase1_review.rs` link các rlib do build hiện tại sinh ra: tái hiện ghép chéo, phiên bản cố định, fallback WiredUsb và empty-response awake. Harness không gửi lệnh HID; mock kiểm tra zero writes thành công.
- Lần compile harness đầu dùng glob chọn nhầm rlib của hai build khác nhau và thất bại; đã build workspace rồi compile lại bằng `target/debug/libmonkey_core.rlib` và `target/debug/libmonkey_cli.rlib`, chạy thành công.
- Chưa xác minh runtime Linux/Windows, OS security prompt, real-device identity grouping hoặc heartbeat sleep/wake. Không suy ra các bảo đảm đó từ 45 test.

Các điểm tốt có bằng chứng: tách core/CLI, mock ghi nhận lời gọi, probe hiện không gọi write_bulk/send_feature_report, packet size và header được kiểm tra bằng test. Những điểm này không khắc phục các lỗi về ý nghĩa dữ liệu ở trên.

## Đối chiếu báo cáo cũ

Hai lỗi Critical của báo cáo cũ bên dưới không còn đúng nguyên trạng: buffer hiện là 65 byte; constructor hiện đọc Bluetooth bus type và thêm product-string matching. Capability LCD/dual-interface đã phụ thuộc interface hiện diện, query error đã được thể hiện thành Unknown/Error, serial rỗng đã được xử lý và multi-device có warning. Không lặp lại các finding đã sửa như lỗi mới. Logic grouping vẫn có counterexample mới ở WR-06.

**Các mục từ đây trở xuống là bản review lịch sử, không phải kết luận hiện tại.** Frontmatter trên chỉ đếm bốn finding hiện tại.

---

## Historical review (preserved)

**Reviewed:** 2026-09-13T14:20:00Z  
**Depth:** standard  
**Files Reviewed:** 16  
**Status:** issues_found  

## Summary

The Phase 01 implementation establishes a clean virtual Cargo workspace (`monkey-core` and `monkey-cli`) with zero-copy packet structures, deterministic in-memory `MockTransport`, synchronous `Transport` trait, non-exclusive macOS device access via `macos-shared-device`, and CLI subcommands (`info` and `probe`). All 39 automated unit and integration tests compile and pass cleanly without compiler warnings.

However, an adversarial audit against hardware constraints, `hidapi` driver internals, and protocol requirements surfaced **2 Critical issues** and **4 Warnings**:
1. **Critical:** Feature report query buffers are sized at 64 bytes instead of 65 bytes (`1 + 64`), causing an `IOHIDDeviceGetReport` buffer overrun error on macOS for unnumbered Report ID 0.
2. **Critical:** Hardware connection state detection fails on Bluetooth keyboards (e.g. `BT5.1-KB`), erroneously classifying them as `WiredUsb` due to brittle substring checks while ignoring `hidapi`'s native `bus_type`. Downstream bulk LCD streaming could attempt to transmit over Bluetooth.
3. **Warnings:** Flawed multi-device isolation without serial numbers, mock-vs-hardware buffer layout divergence in `get_feature_report`, hardcoded capability tuples that contradict live interface availability, and silent error swallowing in `monkey probe`.

---

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: 64-byte feature report probe buffer causes `IOHIDDeviceGetReport` buffer overrun on macOS

**File:** [`crates/monkey-core/src/transport/hid.rs:105-106`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/hid.rs#L105-L106) and [`crates/monkey-cli/src/commands/probe.rs:33-34`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs#L33-L34)  
**Issue:**  
In `hidapi` on macOS (`mac/hid.c`), when invoking `IOHIDDeviceGetReport` with unnumbered Report ID 0, the driver logic reserves byte 0 for the report ID and slices the buffer:
```c
if (report_id == 0x0) {
    report = data + 1;
    report_length = length - 1;
}
res = IOHIDDeviceGetReport(dev->device_handle, type, report_id, report, &report_length);
```
Both `HidTransport::detect_connection_state` and `probe_device_with_transport` allocate a 64-byte buffer:
```rust
let mut probe_buf = [0u8; 64];
let query_res = transport.get_feature_report(0, &mut probe_buf);
```
Because `length` is 64, `report_length` passed to the kernel is only 63 bytes. When the device returns its standard 64-byte feature report, `IOHIDDeviceGetReport` fails with `kIOReturnOverrun` (`0xE00002CD`, buffer overrun). In `probe.rs`, this failure is masked by `.unwrap_or(...)`, but in `detect_connection_state`, it causes an unexpected `TransportError::HidError`. The buffer must be at least `65` bytes (`1 + FEATURE_REPORT_SIZE`).

**Fix:**  
Increase the probe buffer to 65 bytes in both locations:
```rust
// crates/monkey-core/src/transport/hid.rs
pub fn detect_connection_state(&mut self) -> Result<ConnectionState, TransportError> {
    let mut probe_buf = [0u8; 65];
    let query_res = self.get_feature_report(0, &mut probe_buf);
    Self::evaluate_state_query(self.is_wireless, query_res)
}

// crates/monkey-cli/src/commands/probe.rs
pub fn probe_device_with_transport<T: Transport>(
    transport: &mut T,
    device_set: Option<&MonkaDeviceSet>,
    is_wireless: bool,
) -> ProbeOutput {
    let mut probe_buf = [0u8; 65];
    let query_res = transport.get_feature_report(0, &mut probe_buf);
    ...
}
```

---

### CR-02: Bluetooth-connected keyboards erroneously classified as `WiredUsb`

**File:** [`crates/monkey-core/src/transport/hid.rs:52-55`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/hid.rs#L52-L55) and [`crates/monkey-core/src/transport/hid.rs:85-100`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/hid.rs#L85-L100)  
**Issue:**  
`is_wireless_product` checks only for literal substrings:
```rust
pub fn is_wireless_product(product: &str) -> bool {
    let p = product.to_lowercase();
    p.contains("wireless") || p.contains("2.4g") || p.contains("receiver") || p.contains("dongle")
}
```
When the Monka 3075 Pro is connected via Bluetooth on macOS, its product string is `"BT5.1-KB"`. Because this does not match any substring, `is_wireless` is evaluated as `false`. Consequently, `evaluate_state_query` marks the device as `ConnectionState::WiredUsb`.

This violates requirement DISC-05 and creates a severe hazard for Phase 3: the system will believe the device is connected over high-bandwidth wired USB and may attempt 32KB bulk LCD frame transfers over BLE, which is explicitly documented as incapable of supporting bulk frames. Furthermore, in `evaluate_state_query`, if a wired device query times out, it returns `WirelessSleeping`, erroneously claiming a wired device is wireless.

**Fix:**  
1. Include `"bt"` and `"bluetooth"` in `is_wireless_product` (or capture `info.bus_type()` from `hidapi::DeviceInfo`).
2. Do not return `WirelessSleeping` for non-wireless devices on timeout:
```rust
pub fn is_wireless_product(product: &str) -> bool {
    let p = product.to_lowercase();
    p.contains("wireless")
        || p.contains("2.4g")
        || p.contains("receiver")
        || p.contains("dongle")
        || p.contains("bluetooth")
        || p.starts_with("bt")
}

pub fn evaluate_state_query(
    is_wireless: bool,
    query_result: Result<usize, TransportError>,
) -> Result<ConnectionState, TransportError> {
    match query_result {
        Ok(_) => {
            if is_wireless {
                Ok(ConnectionState::WirelessAwake)
            } else {
                Ok(ConnectionState::WiredUsb)
            }
        }
        Err(TransportError::Timeout) => {
            if is_wireless {
                Ok(ConnectionState::WirelessSleeping)
            } else {
                Err(TransportError::Timeout)
            }
        }
        Err(e) => Err(e),
    }
}
```

---

## Warnings

### WR-01: `physical_device_key` fails on macOS, risking cross-pairing multiple keyboards

**File:** [`crates/monkey-core/src/device.rs:119-135`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/device.rs#L119-L135) and [`crates/monkey-core/src/device.rs:174-187`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/device.rs#L174-L187)  
**Issue:**  
Monka 3075 Pro keyboards often report an empty serial number string (`""`) on macOS. In that case, `physical_device_key` attempts to parse parent paths looking for `:1.` or `/IOUSBHostInterface@`. However, `hidapi 2.6.7` on macOS formats device paths strictly as `DevSrvsID:<IORegistryEntryID>` (e.g. `DevSrvsID:4295837800`). Consequently, neither pattern ever matches, and `physical_device_key` returns `None`.

When `physical_device_key` returns `None`, `group_monka_devices` falls back to pairing candidate interfaces with any existing set that "needs this role":
```rust
if target_set_idx.is_none() {
    for (i, set) in sets.iter().enumerate() {
        let needs_role = match role {
            InterfaceRole::InterfaceA => set.interface_a.is_none(),
            InterfaceRole::InterfaceB => set.interface_b.is_none(),
        };
        if needs_role {
            target_set_idx = Some(i);
            break;
        }
    }
}
```
If two Monka keyboards without serial numbers are connected simultaneously, Keyboard 1's Interface A and Keyboard 2's Interface B can be merged into a single `MonkaDeviceSet`, resulting in bulk transfers being sent to one physical keyboard while feature reports are sent to the other.

**Fix:**  
Do not indiscriminately cross-pair interfaces when physical parent keys are unavailable. When serial numbers or physical paths are ambiguous, create separate sets or check matching USB location/address attributes if available.

---

### WR-02: `MockTransport::get_feature_report` buffer layout diverges from `HidTransport`

**File:** [`crates/monkey-core/src/transport/mock.rs:137-138`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/mock.rs#L137-L138)  
**Issue:**  
In `HidTransport::get_feature_report`, `buf[0]` is seeded with `report_id` by `prepare_feature_buffer`, and the operating system places the payload in `buf[1..]` (for unnumbered report ID 0), returning the total byte length including `buf[0]`.

In `MockTransport::get_feature_report`, the mock directly overwrites `buf` starting at index 0:
```rust
buf[..canned.len()].copy_from_slice(canned);
Ok(canned.len())
```
This causes test doubles to behave differently from physical hardware: code developed against `MockTransport` will expect payload data at `buf[0]`, whereas against `HidTransport` it appears at `buf[1]`.

**Fix:**  
Align `MockTransport::get_feature_report` behavior with `HidTransport`: seed `buf[0] = report_id` and copy payload to `buf[1..]` when simulating unnumbered or framed feature reports, or explicitly document and standardize whether the `buf` parameter to `get_feature_report` includes or excludes the report ID framing byte.

---

### WR-03: Hardcoded capability matrix contradicts interface detection

**File:** [`crates/monkey-cli/src/commands/probe.rs:53-66`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs#L53-L66) and [`crates/monkey-cli/src/commands/probe.rs:71-89`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs#L71-L89)  
**Issue:**  
In `probe.rs`, the capability list is statically defined:
```rust
capabilities: vec![
    "LCD display 128x128 RGB565".to_string(),
    "81-key RGB matrix".to_string(),
    "dual composite interface".to_string(),
]
```
When running on real hardware connected over Bluetooth where Interface A is absent, `monkey probe` prints:
```
Hardware Capabilities:
  [✓] LCD display 128x128 RGB565
  [✓] 81-key RGB matrix
  [✓] dual composite interface

Interface Status:
  Interface A (Bulk OUT 0xFF68:0x0061): Not Present
  Interface B (Control  0x000C:0x0001): Detected
```
Displaying green checkmarks for "LCD display" and "dual composite interface" directly above "Interface A: Not Present" is contradictory and misleads users. Furthermore, `build_descriptor_only_probe_output` defaults `(interface_a, interface_b)` to `(true, true)` when `device_set` is `None`.

**Fix:**  
Make the capability list dynamic based on detected interfaces:
```rust
let mut capabilities = vec!["81-key RGB matrix".to_string()];
if interface_a {
    capabilities.push("LCD display 128x128 RGB565".to_string());
}
if interface_a && interface_b {
    capabilities.push("dual composite interface".to_string());
}
```

---

### WR-04: Silent error swallowing in `probe_device_with_transport`

**File:** [`crates/monkey-cli/src/commands/probe.rs:36-42`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs#L36-L42)  
**Issue:**  
In `probe_device_with_transport`:
```rust
let state = HidTransport::evaluate_state_query(is_wireless, query_res).unwrap_or(
    if is_wireless {
        ConnectionState::WirelessAwake
    } else {
        ConnectionState::WiredUsb
    },
);
```
If `query_res` fails due to an I/O error, device disconnection, or buffer overrun, `evaluate_state_query` returns `Err(e)`. The `.unwrap_or(...)` call silently discards this error and fabricates a healthy `WiredUsb` state. This prevents diagnostic detection of broken endpoints.

**Fix:**  
Log a warning on query errors and reflect the failure in `ProbeOutput` (e.g. `transport_state: format!("Unknown/Error: {e}")`).

---

## Info

### IN-01: Empty serial number string printed as blank line in `monkey info`

**File:** [`crates/monkey-cli/src/commands/info.rs:89`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/info.rs#L89)  
**Issue:**  
```rust
serial_number: set.serial_number.clone().or_else(|| Some("N/A".to_string())),
```
When `set.serial_number` is `Some("")` (as observed on macOS), `.or_else()` is not invoked. `format_info_human` prints `Serial Number:          ` with trailing whitespace instead of `N/A`.  
**Fix:** Use `.filter(|s| !s.trim().is_empty()).or_else(|| Some("N/A".to_string()))`.

---

### IN-02: Multi-device connections silently ignored

**File:** [`crates/monkey-cli/src/commands/info.rs:165`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/info.rs#L165) and [`crates/monkey-cli/src/commands/probe.rs:149`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs#L149)  
**Issue:**  
Both `run_info_with_writer` and `run_probe_with_writer` unconditionally index `&sets[0]`. If multiple keyboards are attached, `sets[1..]` are silently omitted without notice.  
**Fix:** If `sets.len() > 1`, log an informational message or iterate over all sets.

---

### IN-03: Heap allocation on bulk write in `HidTransport::write_bulk`

**File:** [`crates/monkey-core/src/transport/hid.rs:60-65`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/hid.rs#L60-L65)  
**Issue:**  
`frame_bulk_buffer` allocates a new `Vec<u8>` for every write call (`Vec::with_capacity(1 + data.len())`). While negligible for Phase 1 smoke probing, high-frequency 15 FPS LCD animation streaming (Phase 3, 120 packets/sec) will create unnecessary heap churn.  
**Fix:** Maintain a reusable pre-allocated buffer `[u8; 4097]` inside `HidTransport` or accept pre-framed buffers.

---

_Reviewed: 2026-09-13T14:20:00Z_  
_Reviewer: adversarial code reviewer (gsd-code-reviewer)_  
_Depth: standard_
