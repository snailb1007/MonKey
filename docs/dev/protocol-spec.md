# Đặc tả giao thức USB HID (Protocol Specification)

Tài liệu này tổng hợp đặc tả kỹ thuật giao tiếp USB HID của bàn phím **Monka 3075 Pro** (Shenzhen HFD Technology `RKGK890`, VID `0x05AC`, PID `0x024F`).

Mọi mục thông tin trong tài liệu này đều được gắn nhãn minh bạch về mức độ xác minh:
- **`[Đã đo kiểm trên phần cứng]`**: Đã được kiểm chứng thực tế qua IOKit/hidapi hoặc WebHID trên thiết bị thật.
- **`[Suy luận từ capture]`**: Được dịch ngược và suy luận từ USB packet captures của phần mềm OEM trên Windows hoặc các dự án tương đồng (`GMK-67-Driver`, `ak820pro-modder`).
- **`[Chưa kiểm chứng]`**: Giả thuyết hoặc thông số đang trong quá trình nghiên cứu, chưa được xác nhận độc lập.

---

## 🔌 Cấu trúc giao diện USB HID (Dual-Interface Model)

Bàn phím xuất hiện trên hệ điều hành dưới dạng hai giao diện HID độc lập:

```
                  ┌──────────────────────────────────────────────┐
                  │ Monka 3075 Pro (VID 0x05AC / PID 0x024F)     │
                  └──────────────┬───────────────────────────────┘
                                 │
                 ┌───────────────┴───────────────┐
                 │                               │
                 ▼                               ▼
  ┌──────────────────────────────┐┌──────────────────────────────┐
  │         Interface A          ││         Interface B          │
  │     Vendor Bulk LCD Pipe     ││   Composite Configuration    │
  │                              ││                              │
  │ Usage Page: 0xFF68           ││ Usage Page: 0xFFFF (Vendor)  │
  │ Usage:      0x0061           ││ Usage Page: 0x000C (Consumer)│
  │ Endpoint:   OUT Report 4096B ││ Usage Page: 0x0001 (Mouse)   │
  │             IN Report    64B ││ Endpoint:   Feature 64B      │
  └──────────────────────────────┘└──────────────────────────────┘
```

---

## 1. Interface A: Kênh truyền màn hình LCD (Vendor Bulk Pipe)

- **Trạng thái:** **`[Đã đo kiểm trên phần cứng]`**
- **Đặc tính kết nối:**
  - **Usage Page:** `0xFF68`
  - **Usage:** `0x0061`
  - **Báo cáo xuất (OUT Report ID 0):** `4096 bytes`
  - **Báo cáo nhập (IN Report ID 0):** `64 bytes`
  - Nằm hoàn toàn độc lập trên interface riêng, không bị chia sẻ với Mouse hay Keyboard của hệ điều hành. Có thể mở trên macOS mà không đòi hỏi quyền Input Monitoring TCC.

### Định dạng khung hình (Frame Layout)
- **Kích thước hiển thị:** 128 × 128 điểm ảnh.
- **Độ sâu màu:** 16-bit RGB565 (2 bytes cho mỗi pixel).
  - Định dạng Little-Endian: Byte thấp chứa 3 bit Blue và 3 bit Green thấp; Byte cao chứa 5 bit Red và 3 bit Green cao.
- **Tổng dung lượng 1 khung hình:**  
  $$128 \times 128 \times 2 = 32.768\text{ bytes}$$
- **Phân đoạn Chunk:**  
  32.768 bytes được chia chính xác thành **8 chunk liên tiếp**, mỗi chunk đúng **4096 bytes**:
  - Chunk 0: Bytes 0 – 4.095
  - Chunk 1: Bytes 4.096 – 8.191
  - ...
  - Chunk 7: Bytes 28.672 – 32.767

### Yêu cầu về nhịp độ truyền (Pacing)
- **Trạng thái:** **`[Đã đo kiểm trên phần cứng]`**
- Vi điều khiển (MCU) bàn phím không có bộ đệm lớn để lưu cả khung hình 32KB một lúc.
- Bắt buộc phải có khoảng nghỉ giữa các chunk (tối thiểu `2ms – 3ms`) để MCU kịp đẩy dữ liệu qua giao tiếp SPI nội bộ tới màn hình TFT trước khi nhận chunk tiếp theo.

---

## 2. Interface B: Kênh cấu hình & Điều khiển LED (Feature Reports)

- **Trạng thái:** **`[Đã đo kiểm trên phần cứng]`**
- **Đặc tính kết nối:**
  - **Usage Page:** `0xFFFF` (Vendor-defined)
  - Co-resident (chia sẻ cổng) với Consumer Control (`0x0C`) và Mouse (`0x01`).
  - Giao tiếp qua **Feature Reports kích thước 64 bytes**.
  - Trên macOS, yêu cầu mở ở chế độ chia sẻ `hid_darwin_set_open_exclusive(0)` để tránh chiếm quyền độc quyền bàn phím từ hệ điều hành.

### Cấu trúc gói tin Feature Report (64 Bytes)
- **Trạng thái:** **`[Suy luận từ capture]`** & **`[Đã đo kiểm trên phần cứng]`**
- Mọi gói tin gửi qua Feature Report bắt đầu bằng byte định danh lệnh (`0x04`):

```text
Byte 0:  0x04 (Command Magic / Prefix)
Byte 1:  <Command ID / Opcode>
Byte 2..63: Tham số lệnh, payload dữ liệu hoặc byte padding (0x00)
```

---

## 3. Bảng mã lệnh (Opcode Table & Whitelist)

Bảng dưới đây liệt kê các mã lệnh được xử lý trong `monkey-core` thông qua `SafetyRails`:

| Opcode (Hex) | Tên lệnh / Chức năng | Mức độ xác minh | Chi tiết payload |
| :---: | :--- | :---: | :--- |
| `0x02` | `ProbeDevice` | **`[Đã đo kiểm trên phần cứng]`** | Truy vấn thông tin capability tuple của thiết bị. |
| `0x18` | `SetLightingConfig` | **`[Đã đo kiểm trên phần cứng]`** | Byte 2: Chế độ LED (`0x01` static, `0x02` wave,...).<br>Byte 3: Tốc độ (`0–100`).<br>Byte 4: Độ sáng (`0–100`).<br>Byte 5–7: Màu RGB (`R, G, B`). |
| `0x13` | `ReadLightingStatus`| **`[Suy luận từ capture]`** | Đọc lại cấu hình LED hiện tại từ bộ nhớ bàn phím. |
| `0x20` | `KeyMapping` | **`[Suy luận từ capture]`** | Đọc/ghi ma trận phím và chức năng phím Fn. |
| `0xF0` | `EndTransaction` | **`[Đã đo kiểm trên phần cứng]`** | Gói tin giải phóng phiên làm việc, báo cho MCU trở lại trạng thái bình thường. |
| `0xF5` | `SaveSettings` | **`[Đã đo kiểm trên phần cứng]`** | Ra lệnh cho MCU lưu cấu hình từ RAM tạm vào chip nhớ Flash SPI NOR. |

---

## ⚠️ Vùng mã nguy hiểm (Dangerous / Bootloader Opcodes)

- **Trạng thái:** **`[Đã đo kiểm trên phần cứng]`**
- Chip Shenzhen HFD chia sẻ không gian địa chỉ USB giữa chế độ hoạt động bình thường và chế độ nạp firmware (ISP Bootloader).
- **Mã định danh ISP:** PID `0x7140` (HFD ISP Mode).
- **Quy tắc an toàn trong code:**
  - `SafetyRails` áp dụng chính sách **Default-Deny (Mặc định từ chối)**: Chỉ các opcode nằm trong danh sách whitelist ở trên mới được phép gửi đi.
  - Bất kỳ opcode lạ nào hoặc lệnh nạp bootloader đều bị chặn ở tầng `monkey-core` trước khi chạm vào driver USB của hệ điều hành.
