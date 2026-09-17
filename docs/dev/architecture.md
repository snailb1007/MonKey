# Kiến trúc hệ thống MonKey (Architecture)

Tài liệu này mô tả chi tiết kiến trúc phần mềm, mô hình điều phối phần cứng (`MonkaDevice`), cơ chế an toàn phân tầng (`SafeTransport`), mô hình đa luồng và trừu tượng hóa giao vận trong dự án **MonKey**.

---

## 🏛️ Tổng quan kiến trúc phân tầng (Layered Architecture)

Hệ thống được tổ chức theo triết lý **Deep Module** (John Ousterhout): giao diện bên ngoài đơn giản, nhất quán, che giấu độ phức tạp về quản lý bộ đệm, nhịp độ phần cứng (pacing), kiểm tra an toàn và xử lý điểm cuối USB HID composite.

```
┌─────────────────────────────────────────────────────────────┐
│                       monkey-cli                            │
│  - Phân tích tham số dòng lệnh (Clap v4)                    │
│  - Thanh tiến trình & đo đạc thông lượng (indicatif)        │
│  - Định dạng xuất bản (Human-readable & JSON)               │
│  - Điều hướng lệnh: info, probe, bench, lcd, rgb, doctor    │
│  - Phân loại mã thoát chuẩn POSIX (ExitCode 0..=5)          │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                  MonkaDevice (Coordinator)                  │
│  - Quản lý vòng đời thiết bị: open(policy), discover()      │
│  - Chính sách giao diện: InterfacePolicy (A/B/Prefer/Any)   │
│  - Phân loại lỗi khởi tạo phân tầng: OpenError              │
│  - Chẩn đoán không dừng sớm: MonkaDevice::diagnose()        │
│  - Vận hành sâu (Deep Operations): LCD, RGB, Probe, Bench   │
│  - Sở hữu nội bộ SafetyRails (Zero speculative flash)       │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│               SafeTransport<'a> (Guard Façade)               │
│  - Cổng kiểm soát ủy quyền ghi phần cứng (Write Consent)    │
│  - Thẩm tra opcode, giới hạn tần suất ghi flash (Debounce)  │
│  - Tách rời Transport thuần túy khỏi logic an toàn          │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                   Trừu tượng hóa Transport                  │
│  - Pure I/O Trait: write_bulk, send_feature, get_feature    │
│                                                             │
│   ┌───────────────────────────┐ ┌─────────────────────────┐ │
│   │       HidTransport        │ │      MockTransport      │ │
│   │   (hidapi + macos-shared) │ │ (In-memory, headless CI)│ │
│   └─────────────┬─────────────┘ └─────────────────────────┘ │
└─────────────────┼───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│                     Hệ điều hành / OS                       │
│    macOS (IOKit)  │  Linux (hidraw)  │  Windows (Win32 HID) │
└─────────────────┼───────────────────────────────────────────┘
                  │
                  ▼
       [Monka 3075 Pro Hardware (VID 0x05AC / PID 0x024F)]
```

---

## 📦 Phân chia ranh giới giữa các Crate (Crate Boundaries)

### 1. `crates/monkey-core`
- **Mục đích:** Thư viện điều khiển phần cứng cốt lõi, an toàn bộ nhớ, không phụ thuộc vào UI hay CLI.
- **Ràng buộc thiết kế:** 
  - Hoàn toàn **đồng bộ (synchronous)** ở tầng lõi. Không tích hợp runtime bất đồng bộ Tokio vào `monkey-core` để đảm bảo định thời milisecond-level chính xác, tránh jitter khi truyền dữ liệu hiển thị màn hình LCD.
  - Phụ thuộc tối thiểu: `hidapi` (giao tiếp HID đa nền tảng), `zerocopy` (chuyển đổi layout bộ nhớ an toàn không cấp phát heap), `image` (giải mã ảnh), `thiserror` (định nghĩa lỗi định kiểu mạnh).
- **Các module cốt lõi:**
  - `device`: Bộ điều phối `MonkaDevice`, chính sách `InterfacePolicy`, lỗi khởi tạo `OpenError`, chẩn đoán `DeviceDiagnostics`.
  - `transport`: Trait thuần túy `Transport`, bộ bảo vệ an toàn `SafeTransport<'a>`, triển khai `HidTransport` và bộ giả lập `MockTransport` / `SharedMockTransport`.
  - `protocol`: Mã hóa/giải mã gói tin (framing, CRC16, opcode definitions, transaction safety).
  - `lcd`: Bộ nạp ảnh, dither màu (Floyd-Steinberg), phân mảnh 8 chunks / frame (`LCD_CHUNK_COUNT = 8`, `LCD_CHUNK_SIZE = 4096`).
  - `rgb`: Mã hóa chế độ màu, quản lý trạng thái hai tầng (`RgbManager`: RAM preview & Flash commit debouncing).
  - `doctor`: Động cơ kiểm tra sức khỏe hệ thống và môi trường dựa trên `MonkaDevice::diagnose()`.
  - `bench`: Harness đo đạc thông lượng bulk và độ trễ feature report.

### 2. `crates/monkey-cli`
- **Mục đích:** Cung cấp giao diện dòng lệnh cho người dùng (`monkey`).
- **Trách nhiệm:**
  - Nhận cờ và tham số cấu hình qua `clap` v4.
  - Khởi tạo thiết bị phần cứng hoàn toàn qua `MonkaDevice::open(policy)` hoặc mock qua `MonkaDevice::from_transport(mock)`.
  - Quản lý thanh tiến trình và hiển thị thông số thời gian thực với `indicatif`.
  - Phân loại lỗi và duy trì hợp đồng mã thoát POSIX (ExitCode 0..=5).

---

## 🎮 Bộ điều phối phần cứng `MonkaDevice` (D-01, D-04)

`MonkaDevice` đóng vai trò là điểm chạm phần cứng duy nhất cho toàn bộ các lệnh ứng dụng:

```rust
pub struct MonkaDevice {
    transport: Box<dyn Transport>,
    safety: SafetyRails,
    device_set: Option<MonkaDeviceSet>,
    role: Option<InterfaceRole>,
    is_wireless: bool,
}
```

### 1. Nguyên lý điểm kiểm thử duy nhất (Single Test Seam Principle - D-01)
- `Transport` là điểm trừu tượng hóa duy nhất cho toàn bộ I/O phần cứng.
- Mọi kiểm thử headless CI có thể khởi tạo `MonkaDevice::from_transport(Box::new(mock))` mà không cần khởi tạo HID API thật.
- `SharedMockTransport` cho phép kiểm tra lịch sử gọi I/O và xác thực bất biến an toàn ghi sau khi chuyển quyền sở hữu vào `MonkaDevice`.

### 2. Vận hành sâu (Deep Module Operations - D-04)
Thay vì để các lệnh CLI tự khởi tạo `LcdStreamer`, `RgbManager`, hoặc gọi raw I/O, `MonkaDevice` cung cấp các phương thức điều phối cấp cao an toàn:
- `stream_frame_with_progress(&mut self, frame, config, on_chunk)`: Truyền 32,768 bytes (8 chunks x 4096) với nhịp độ an toàn.
- `apply_rgb_preview(&mut self, config)`: Áp dụng hiệu ứng ánh sáng vào RAM tạm thời (không ghi flash).
- `apply_rgb_commit(&mut self, config, is_wireless, battery, force)`: Ghi cấu hình vĩnh viễn vào SPI NOR flash có kiểm tra ngưỡng pin và chống hao mòn (debounced).
- `readback_rgb_status(&mut self)`: Đọc trạng thái ánh sáng hiện tại từ thiết bị.
- `probe(&mut self)`: Thu thập cấu hình phần cứng và phiên bản firmware ở chế độ chỉ đọc (Read-Only Safety Invariant).
- `run_bulk_benchmark(&mut self, config, on_frame)` & `run_transaction_benchmark(&mut self, config, on_sample)`.

---

## 🔀 Chính sách lựa chọn giao diện `InterfacePolicy` (D-02)

Bàn phím Monka 3075 Pro sở hữu hai giao diện USB composite với các vai trò chuyên biệt:
- **Interface A (`0xFF68:0x0061`)**: Standalone Vendor Bulk Pipe (OUT reports 4096 bytes cho màn hình LCD). Không yêu cầu quyền đặc biệt trên macOS.
- **Interface B (`0x000C:0x0001` / `0xFFFF:0x0001`)**: Shared Control & Configuration Pipe (Feature reports 64 bytes cho RGB và cài đặt). Yêu cầu chế độ chia sẻ `macos-shared-device`.

`InterfacePolicy` biểu diễn tường minh mục đích mở thiết bị:

| Biến thể Policy | Ý nghĩa điều phối | Phù hợp cho |
|-----------------|-------------------|-------------|
| `RequireA` | Bắt buộc phải có Interface A; báo lỗi nếu thiếu | Lệnh `monkey lcd` |
| `RequireB` | Bắt buộc phải có Interface B; báo lỗi nếu thiếu | Lệnh `monkey probe` |
| `PreferB` | Ưu tiên Interface B; nếu không có thì fallback sang Interface A | Lệnh `monkey rgb` |
| `BulkFirst` | Ưu tiên Interface A; nếu không có thì fallback sang Interface B | Lệnh `monkey bench --type bulk` |
| `ControlFirst` | Ưu tiên Interface B; nếu không có thì fallback sang Interface A | Lệnh `monkey bench --type transaction` |
| `Any` | Chấp nhận bất kỳ giao diện Monka nào khả dụng | Kiểm tra kết nối tổng quát |

---

## 🛡️ Phân loại lỗi khởi tạo `OpenError` & Chẩn đoán `diagnose()` (D-03)

Quá trình kết nối phần cứng được phân định rõ ràng thành các giai đoạn độc lập:

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    #[error("Failed to initialize HID subsystem: {0}")]
    HidInit(#[from] TransportError),

    #[error("No Monka 3075 Pro / RKGK890 keyboard detected (VID: 0x05ac, PID: 0x024f). Please check USB connection.")]
    NoDevice,

    #[error("Requested interface {0:?} is not available on detected keyboard")]
    InterfaceUnavailable(InterfaceRole),

    #[error("Failed to open interface {0:?}: {1}")]
    InterfaceOpenFailed(InterfaceRole, TransportError),
}
```

### Chẩn đoán không dừng sớm (Non-Fail-Fast Diagnostic Inspection)
`MonkaDevice::diagnose() -> DeviceDiagnostics` thực hiện kiểm tra toàn diện cây thiết bị mà không ngắt giữa chừng:
1. Trạng thái khởi tạo HID API subsystem.
2. Thiết bị Monka 3075 Pro có hiện diện trên bus hay không.
3. Khả năng mở Interface A (màn hình LCD).
4. Khả năng mở Interface B (điều khiển RGB / cấu hình).

Cơ chế này cho phép `monkey doctor` đưa ra chẩn đoán đầy đủ và hướng dẫn khắc phục chính xác (ví dụ: cấp quyền Input Monitoring nếu Interface B bị chặn) ngay cả khi một phần giao diện gặp lỗi.

---

## 🔒 Lớp bảo vệ `SafeTransport<'a>` (D-07)

Để tuân thủ nguyên lý tách biệt trách nhiệm (Separation of Concerns):
- `Transport` trait là giao diện I/O thô, thuần túy đọc/ghi không chứa logic nghiệp vụ hay quyền hạn.
- `SafeTransport<'a>` là một RAII guard façade kết hợp tham chiếu có thể thay đổi `&'a mut dyn Transport` và bộ quy tắc `&'a SafetyRails`.
- Mọi thao tác ghi (`write_bulk`, `send_feature_report`) bắt buộc phải thông qua `SafeTransport`, nơi kiểm tra thẩm quyền `--allow-hardware-writes`, opcode whitelist, và chống hao mòn flash (T-06-01, T-06-06).

---

## 🧹 Xóa bỏ biến môi trường ẩn và thống nhất Test Seams (D-05)

Nhằm đảm bảo tính tin cậy tuyệt đối và loại bỏ các cửa sau (backdoor) tiềm ẩn:
- Biến môi trường `MONKEY_SIMULATE_EMPTY` đã bị **xóa bỏ hoàn toàn** khỏi toàn bộ codebase (T-06-05 mitigation).
- Năm điểm nối test tùy biến cục bộ đã được loại bỏ:
  - `run_probe_with_transport` & `probe_device_with_transport` trong `probe.rs`
  - `open_lcd_transport` & `stream_one` trong `lcd.rs`
  - `resolve_transport` & `TransportResolution` trong `rgb.rs`
  - `run_bench_with_transport` trong `bench.rs`
- Mọi kiểm thử phần cứng và lệnh CLI đều sử dụng chung một quy trình chuẩn qua `MonkaDevice`.

---

## 🚦 Bảng mã thoát chuẩn POSIX (POSIX Exit Codes)

CLI `monkey` duy trì hợp đồng mã thoát nghiêm ngặt giúp tích hợp tin cậy vào shell script và CI pipeline:

| Mã thoát | Danh mục | Nguyên nhân kích hoạt |
|----------|----------|----------------------|
| `0` | `Success` | Thao tác hoàn thành thành công |
| `1` | `General` | Lỗi nội bộ không xác định hoặc lỗi hệ thống chung |
| `2` | `Usage` | Tham số dòng lệnh không hợp lệ, định dạng màu/ảnh sai |
| `3` | `NoDevice` | Không tìm thấy bàn phím Monka 3075 Pro trên bus USB |
| `4` | `Permission` | Thiếu quyền OS (TCC Input Monitoring, Exclusive Access conflict) |
| `5` | `Blocked` | Rào chắn an toàn can thiệp (thiếu `--allow-hardware-writes`, pin < 20%) |
