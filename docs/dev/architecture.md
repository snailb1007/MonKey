# Kiến trúc hệ thống MonKey (Architecture)

Tài liệu này mô tả chi tiết kiến trúc phần mềm, mô hình phân tầng, giải pháp đa luồng và trừu tượng hóa phần cứng trong dự án **MonKey**.

---

## 🏛️ Tổng quan kiến trúc phân tầng (Layered Architecture)

Dự án được chia tách nghiêm ngặt thành hai crate độc lập trong một Cargo virtual workspace nhằm tách rời giao diện người dùng/CLI khỏi logic điều khiển phần cứng:

```
┌─────────────────────────────────────────────────────────────┐
│                       monkey-cli                            │
│  - Phân tích cú pháp dòng lệnh (Clap v4)                    │
│  - Hiển thị thanh tiến trình & FPS (indicatif)              │
│  - Định dạng xuất bản (Human-readable text & JSON)          │
│  - Định tuyến lệnh: info, probe, bench, lcd, rgb, doctor    │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                      monkey-core                            │
│  - Lớp giao thức & mã hóa: Codecs, Framing, CRC16           │
│  - Lớp an toàn phần cứng: SafetyRails, TransactionManager   │
│  - Đồ họa & Xử lý hình ảnh: Resize, Dither (Floyd-Steinberg)│
│  - Quản lý trạng thái: RgbManager (RAM preview & Flash sync)│
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                   Trừu tượng hóa Transport                  │
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
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
       [Monka 3075 Pro Hardware (VID 0x05AC / PID 0x024F)]
```

---

## 📦 Phân chia ranh giới giữa các Crate (Crate Boundaries)

### 1. `crates/monkey-core`
- **Mục đích:** Thư viện điều khiển phần cứng thuần túy, an toàn bộ nhớ và không phụ thuộc vào UI hay CLI.
- **Ràng buộc thiết kế:** 
  - Hoàn toàn **đồng bộ (synchronous)** ở tầng lõi. Không kéo runtime bất đồng bộ Tokio vào `monkey-core` để tránh làm nặng thư viện và tránh jitter (độ trễ không đều) khi truyền các gói tin màn hình milisecond-level.
  - Phụ thuộc tối thiểu: `hidapi` (giao tiếp HID), `zerocopy` (chuyển đổi layout bộ nhớ an toàn), `image` (giải mã ảnh), `thiserror` (định nghĩa lỗi định kiểu mạnh `MonkeyError`).
- **Các module chính:**
  - `device`: Tìm kiếm và nhận diện cặp endpoint Monka trên USB bus.
  - `protocol`: Mã hóa/giải mã gói tin (framing, CRC16, opcode definitions).
  - `safety`: Cơ chế kiểm soát an toàn (`SafetyRails`, giới hạn tần suất flash).
  - `lcd`: Bộ nạp ảnh, dither màu và streamer 8 chunk / frame.
  - `rgb`: Mã hóa chế độ màu và quản lý trạng thái hai tầng (RAM/Flash).
  - `transport`: Định nghĩa `Transport` trait, `HidTransport` và `MockTransport`.
  - `doctor`: Động cơ kiểm tra sức khỏe hệ thống và môi trường.

### 2. `crates/monkey-cli`
- **Mục đích:** Cung cấp giao diện dòng lệnh cho người dùng (`monkey`).
- **Trách nhiệm:**
  - Nhận cờ và tham số qua `clap`.
  - Quản lý thanh tiến trình và đo đạc FPS thời gian thực với `indicatif`.
  - Xử lý các tín hiệu hủy (`SIGINT` / `Ctrl+C`).
  - Định dạng dữ liệu đầu ra: dạng bảng trực quan cho con người hoặc JSON qua cờ `--json`.

---

## 🧵 Mô hình Concurrency & Threading

Giao tiếp USB HID trên phần cứng nhúng là giao tiếp **đơn luồng vật lý (single-flight) và có trạng thái (stateful)**. Việc gửi đồng thời nhiều gói tin chồng chéo từ các luồng khác nhau sẽ làm hỏng khung hình hiển thị hoặc làm nghẽn bộ đệm của vi điều khiển (MCU).

### 1. Kênh phần cứng chuyên biệt (Hardware Channel)
Trong `monkey-core`, các thao tác phần cứng phức tạp được điều phối qua một kênh gửi nhận thông điệp dựa trên `crossbeam-channel`:
- Thiết bị `HidDevice` được sở hữu độc quyền bởi luồng xử lý phần cứng.
- Tránh việc chia sẻ con trỏ thiết bị thô giữa các thread.

### 2. Điều tiết nhịp độ truyền tin (Pacing Control)
- MCU của Monka 3075 Pro cần một khoảng thời gian nghỉ giữa các chunk truyền màn hình để kịp ghi dữ liệu từ bộ đệm USB vào RAM hiển thị của chip TFT.
- Cấu hình `LcdPacingConfig` áp dụng khoảng trễ mặc định `3ms` (có thể tinh chỉnh `0–8ms` qua cờ `--inter-chunk-delay-ms`).
- Trình phát ảnh động điều tiết tốc độ khung hình (mặc định 12 FPS) bằng đồng hồ vi sai `Instant` để đảm bảo không gửi thừa khung hình làm nghẽn bus.

---

## 🔌 Trừu tượng hóa Transport (`Transport` Trait)

Để hỗ trợ kiểm thử tự động toàn diện mà không cần cắm phần cứng thật trong môi trường CI, mọi thao tác I/O đều thông qua trait:

```rust
pub trait Transport: Send {
    fn write_output_report(&mut self, report_id: u8, data: &[u8]) -> Result<usize>;
    fn get_feature_report(&mut self, report_id: u8, data: &mut [u8]) -> Result<usize>;
    fn send_feature_report(&mut self, report_id: u8, data: &[u8]) -> Result<usize>;
    fn read_input_report(&mut self, data: &mut [u8], timeout_ms: i32) -> Result<usize>;
}
```

### 1. `HidTransport` (Phần cứng thật)
- Bọc quanh con trỏ thiết bị của thư viện `hidapi`.
- Trên macOS, sử dụng tính năng `macos-shared-device` (`hid_darwin_set_open_exclusive(0)`) để chia sẻ quyền mở thiết bị composite với hệ thống của Apple mà không làm mất kết nối bàn phím.
- **Xử lý Report ID 0:** Giao diện Interface A sử dụng unnumbered report (Report ID = 0). Khi gọi `hid_write`, thư viện yêu cầu buffer bắt đầu bằng byte `0x00`, sau đó Apple IOKit sẽ tự bóc tách byte dẫn này trước khi truyền ra bus.

### 2. `MockTransport` (Kiểm thử tự động)
- Lưu trữ toàn bộ lịch sử các lệnh gửi/nhận (`TransportCall`) trong bộ nhớ ram.
- Trả về các phản hồi giả lập định sẵn tương thích với thông số Monka 3075 Pro.
- Cho phép bộ test CI kiểm tra tính đúng đắn của giải thuật dither, framing chunk, checksum CRC và mã hóa RGB một cách tất định (deterministic).
