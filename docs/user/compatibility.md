# Bảng tương thích thiết bị & Nền tảng (Compatibility)

Tài liệu này cung cấp thông tin chi tiết về các thiết bị phần cứng, firmware và hệ điều hành đã được đo kiểm hoặc nằm trong phạm vi hỗ trợ của MonKey.

---

## ⌨️ Phần cứng hỗ trợ chính thức

| Thiết bị | Thông số phần cứng | Mức độ xác minh | Ghi chú |
| :--- | :--- | :---: | :--- |
| **Monka 3075 Pro** (Bản có LCD) | - Layout: 75% (81 phím)<br>- Controller: Shenzhen HFD `RKGK890`<br>- USB VID: `0x05AC`<br>- USB PID: `0x024F`<br>- LCD: 128×128 RGB565 TFT | **[Đã kiểm chứng trên phần cứng]** | Thiết bị mục tiêu chính của dự án. Đã xác minh Interface A (`0xFF68`) và Interface B (`0xFFFF`). |

### Các dòng bàn phím tương đồng (OEM Siblings)
Bộ điều khiển `RKGK890` (Shenzhen HFD Technology) với cặp VID/PID `05AC:024F` xuất hiện trên nhiều bàn phím cơ OEM từ các thương hiệu khác nhau (như một số lô GMK67, Akko, hoặc Ziyoulang). 
- **Khả năng tương thích:** Về mặt lý thuyết, các lệnh cấu hình RGB (`04 xx`) có thể hoạt động tương tự.
- **Cảnh báo an toàn:** Các biến thể không có màn hình LCD hoặc sử dụng độ phân giải màn hình khác tuyệt đối không nên chạy lệnh `monkey lcd` để tránh lỗi tràn bộ đệm màn hình.

---

## 💻 Hệ điều hành hỗ trợ

| Hệ điều hành | Trạng thái hỗ trợ | Chi tiết triển khai | Yêu cầu quyền hạn |
| :--- | :---: | :--- | :--- |
| **macOS** (12 Monterey – 15 Sequoia) | **Chính thức**<br>*(Môi trường phát triển chính)* | Sử dụng `hidapi` với cờ `macos-shared-device` (`hid_darwin_set_open_exclusive(0)`). Giao tiếp trực tiếp qua Apple IOKit. | - Interface A (LCD): Không cần cấp quyền đặc biệt.<br>- Interface B (RGB): Có thể cần quyền **Input Monitoring** nếu mở đọc dữ liệu. |
| **Linux** (Kernel 5.4+) | **Hỗ trợ** | Giao tiếp qua `hidraw` trong userspace. | Cần tạo file `udev rules` để cấp quyền đọc/ghi cho user không phải root (xem bên dưới). |
| **Windows** (10 / 11) | **Khả dụng** | Giao tiếp qua Win32 HID subsystem trong `hidapi`. | Hoạt động bình thường ở chế độ quyền người dùng thông thường. |

---

## 🐧 Hướng dẫn thiết lập udev rules trên Linux

Trên Linux, theo mặc định hệ thống chỉ cho phép quyền `root` truy cập vào các cổng raw HID. Để tài khoản người dùng bình thường có thể chạy `monkey`, bạn cần thêm rule udev:

1. Tạo file `/etc/udev/rules.d/99-monka-keyboard.rules`:
```udev
# Monka 3075 Pro (Shenzhen HFD Technology RKGK890)
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="05ac", ATTRS{idProduct}=="024f", MODE="0666", GROUP="plugdev"
```

2. Tải lại udev rules và cắm lại cáp USB:
```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

---

## 📶 Phương thức kết nối: Cáp có dây vs. Không dây 2.4GHz

Monka 3075 Pro hỗ trợ 3 chế độ (Có dây USB, Không dây 2.4GHz qua USB Dongle, và Bluetooth):

1. **Kết nối có dây (Wired USB Type-C) — Khuyến nghị:**
   - Cung cấp băng thông tối đa và ổn định cho việc stream ảnh động LCD (đạt tốc độ 10–15 FPS không rớt khung hình).
   - An toàn tuyệt đối khi thực hiện ghi vào bộ nhớ Flash (`--commit`).

2. **Kết nối không dây 2.4GHz (USB Receiver):**
   - Vẫn nhận diện được các endpoint HID nhưng băng thông không dây có thể gây giật khung hình khi stream LCD liên tục.
   - **Cơ chế an toàn:** Nếu bàn phím báo pin dưới 20%, lệnh `--commit` sẽ bị khóa tự động để tránh mất nguồn làm hỏng dữ liệu Flash (phải cắm cáp hoặc thêm `--force`).

3. **Kết nối Bluetooth:**
   - **Chưa hỗ trợ** cho các lệnh ghi nâng cao (LCD & bulk streaming). Hệ điều hành quản lý Bluetooth HID theo cấu hình bàn phím chuẩn và không mở vendor bulk pipe.
