# MonKey

Open-source macOS and cross-platform driver and CLI tooling for the **Monka 3075 Pro** mechanical keyboard (and related Shenzhen HFD `RKGK890` OEM hardware).

> ⚠️ **Trạng thái dự án (Đang trong giai đoạn phát triển & thử nghiệm):**  
> Bộ mã nguồn CLI đã triển khai đầy đủ các nhóm lệnh (`info`, `probe`, `bench`, `lcd`, `rgb`, `doctor`, `completions`) và vượt qua kiểm thử tự động với Mock transport. Tuy nhiên, việc kiểm thử xác nhận (UAT) trên các lô phần cứng và phiên bản firmware thực tế vẫn đang được tiến hành. Mọi thao tác ghi lên thiết bị thật cần được thực hiện cẩn trọng.

---

## 🎯 Phạm vi phần cứng hỗ trợ

| Thông số | Giá trị | Ghi chú |
| :--- | :--- | :--- |
| **Bàn phím** | Monka 3075 Pro (Layout 75%, 81 phím) | Đã thử nghiệm trên biến thể có màn hình LCD |
| **Bộ điều khiển** | Shenzhen HFD Technology (`RKGK890`) | |
| **USB VID / PID** | `0x05AC` : `0x024F` | Apple VID được OEM giả lập trên firmware |
| **Màn hình tích hợp** | 128×128 TFT LCD | Chuẩn màu RGB565, truyền qua vendor bulk pipe |
| **Hệ điều hành** | macOS (ưu tiên thử nghiệm), Linux, Windows | Thông qua thư viện `hidapi` (`macos-shared-device`) |

---

## 🚀 Cài đặt

Hiện tại dự án đang được phân phối dưới dạng mã nguồn Rust:

### Yêu cầu môi trường
- [Rust toolchain](https://rustup.rs/) (khuyến nghị phiên bản 1.80+)
- Trên Linux: cần thư viện `libusb-1.0` và `libudev` (ví dụ: `sudo apt install libusb-1.0-0-dev libudev-dev`)

### Build từ source

```bash
# Clone repository
git clone https://github.com/snailb1007/MonKey.git
cd MonKey

# Cài đặt trực tiếp CLI binary vào $HOME/.cargo/bin
cargo install --path crates/monkey-cli

# Hoặc chỉ build bản release binary tại target/release/monkey
cargo build --release
```

*(Các kênh phân phối nhị phân dựng sẵn qua GitHub Releases và Homebrew Tap đang được chuẩn bị cho các mốc phát hành chính thức).*

---

## ⚡ Thao tác nhanh (Quick Start)

### 1. Kiểm tra môi trường và quyền truy cập USB
Chạy lệnh chẩn đoán hệ thống để kiểm tra xem hệ điều hành đã nhận diện bàn phím và quyền truy cập HID có khả dụng không:

```bash
monkey doctor
```

### 2. Xem thông tin thiết bị (Chế độ đọc an toàn - Read-only)
Liệt kê định danh phần cứng và các giao diện USB HID mà không làm thay đổi trạng thái bàn phím:

```bash
monkey info
```

### 3. Chạy thử nghiệm giả lập (Mock Mode)
Bạn có thể thử nghiệm cú pháp và pipeline xử lý ảnh/màu sắc mà không cần can thiệp vào phần cứng thật bằng cờ `--mock`:

```bash
# Giả lập quá trình nạp ảnh tĩnh lên LCD
monkey lcd image assets/sample.png --mock

# Giả lập thiết lập hiệu ứng RGB
monkey rgb set wave --mock
```

---

## 🎮 Điều khiển phần cứng (Hardware Writes)

> ⚠️ **Quy tắc an toàn:** Để tránh các thao tác ghi ngoài ý muốn lên bộ điều khiển, tất cả các lệnh ghi lên thiết bị thật **bắt buộc phải có cờ `--allow-hardware-writes`**.

### Điều khiển màn hình LCD (`monkey lcd`)

Màn hình LCD 128×128 giao tiếp qua đường truyền Vendor Bulk Pipe (`0xFF68`).

```bash
# Nạp 1 ảnh tĩnh (hỗ trợ PNG, JPEG, BMP - tự động căn chỉnh và xử lý màu)
monkey lcd image path/to/image.png --allow-hardware-writes

# Bật thuật toán dithering (Floyd-Steinberg) để dải màu mượt hơn
monkey lcd image path/to/image.png --dither --allow-hardware-writes

# Phát ảnh động GIF (chạy liên tục ở foreground, nhấn Ctrl+C để dừng an toàn)
monkey lcd anim path/to/animation.gif --allow-hardware-writes

# Hiển thị bảng màu kiểm tra căn chỉnh màn hình
monkey lcd test-pattern rgb-bars --allow-hardware-writes
```

### Điều khiển hiệu ứng LED RGB (`monkey rgb`)

Giao tiếp cấu hình LED sử dụng Feature Reports qua composite interface (`0xFFFF`).

```bash
# Cập nhật màu tĩnh (Mặc định: chỉ xem trước trên RAM, mất khi rút nguồn)
monkey rgb set static --color 00FFFF --brightness 100 --allow-hardware-writes

# Thiết lập hiệu ứng sóng lượn với tốc độ và độ sáng tùy chỉnh (thang 0-100)
monkey rgb set wave --speed 60 --brightness 80 --allow-hardware-writes

# Lưu cấu hình vĩnh viễn vào chip nhớ Flash SPI NOR (thêm cờ --commit)
monkey rgb set breathing --color FF0055 --commit --allow-hardware-writes

# Lưu cấu hình hiện tại ra file JSON để sao lưu
monkey rgb save --file my_profile.json

# Khôi phục cấu hình từ file profile JSON
monkey rgb restore --file my_profile.json --allow-hardware-writes
```

---

## 🛡️ Cơ chế an toàn phần cứng (Safety Rails)

Việc reverse engineering giao thức USB HID tiềm ẩn rủi ro nếu gửi sai mã điều khiển tới firmware. MonKey tích hợp sẵn các lớp kiểm soát trong `monkey-core`:

- **Opcode Whitelist (`SafetyRails`):** Toàn bộ các gói tin cấu hình đều được lọc qua bảng mã lệnh cho phép; chặn việc gửi các opcode lạ hoặc mã kích hoạt bootloader ISP (`0x7140`).
- **Phân tầng RAM Preview & Flash Commit:** Các thay đổi hiệu ứng thông thường chỉ gửi vào bộ nhớ tạm thời (RAM). Cờ `--commit` được giới hạn tần suất (tối thiểu 500ms giữa các lần commit trong cùng phiên chạy) nhằm hạn chế chu kỳ ghi/xóa của chip flash SPI NOR.
- **Bảo vệ khi pin yếu (Wireless):** Lệnh commit vào Flash sẽ bị chặn nếu thiết bị báo pin dưới 20% trên kết nối không dây để phòng ngừa sụt nguồn giữa chừng (có thể bỏ qua bằng `--force` nếu người dùng chấp nhận rủi ro).
- **Ngắt an toàn:** Trình phát ảnh LCD lắng nghe tín hiệu hủy (`SIGINT` / `Ctrl+C`) để dừng luồng truyền gói tin một cách có kiểm soát.

---

## 📚 Tài liệu chi tiết

Tài liệu dự án được phân chia theo nhu cầu sử dụng:

### Dành cho người dùng (User Guide)
- [Hướng dẫn cài đặt chi tiết](docs/user/installation.md): Thiết lập quyền truy cập USB trên macOS / Linux udev rules.
- [Bắt đầu nhanh trong 3 phút](docs/user/quickstart.md): Hướng dẫn từng bước cho người mới bắt đầu.
- [Bảng tra cứu lệnh CLI](docs/user/cli-reference.md): Toàn bộ tham số và cờ của các nhóm lệnh `lcd`, `rgb`, `bench`, `doctor`.
- [Bảng tương thích thiết bị](docs/user/compatibility.md): Danh sách các model và phiên bản firmware đã được thử nghiệm.
- [Khắc phục sự cố](docs/user/troubleshooting.md): Xử lý lỗi quyền truy cập và hướng dẫn dùng `monkey doctor`.

### Dành cho nhà phát triển & nghiên cứu (Developer & Architecture)
- [Kiến trúc hệ thống](docs/dev/architecture.md): Thiết kế phân tầng, luồng xử lý đồng bộ và `MockTransport`.
- [Đặc tả giao thức USB HID](docs/dev/protocol-spec.md): Phân tích Interface A (`0xFF68`), Interface B (`0xFFFF`), định dạng chunk 4096B và feature reports.
- [Cơ chế bảo vệ phần cứng](docs/dev/hardware-safety.md): Chi tiết implementation của `SafetyRails` và `TransactionManager`.
- [Hướng dẫn đóng góp](CONTRIBUTING.md): Quy trình build, viết unit test không cần phần cứng và tiêu chuẩn mã nguồn.
- [Nhật ký nghiên cứu](research/README.md): Dữ liệu thô, USB packet captures và các ghi chép reverse engineering ban đầu.

---

## 📜 License

Dự án được phát hành theo giấy phép [MIT](LICENSE).
