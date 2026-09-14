# Hướng dẫn bắt đầu nhanh (Quick Start)

Tài liệu này giúp bạn làm quen với các thao tác cơ bản của `monkey` trong vòng 3 phút: từ kiểm tra kết nối, thử nghiệm an toàn không cần phím thật đến việc nạp ảnh lên LCD và đổi màu LED RGB.

---

## 📋 Bước 1: Kiểm tra kết nối và quyền hạn (`monkey doctor`)

Trước tiên, hãy cắm bàn phím Monka 3075 Pro vào máy tính qua cáp USB Type-C. Sau đó chạy:

```bash
monkey doctor
```

Lệnh này sẽ rà soát:
- Quyền truy cập thiết bị HID của hệ điều hành.
- Kết nối tới **Interface A (`0xFF68`)** — dùng cho màn hình LCD.
- Kết nối tới **Interface B (`0xFFFF`)** — dùng cho cấu hình phím & LED RGB.

Nếu mọi thứ hiển thị dấu tick xanh `[OK]`, hệ thống đã sẵn sàng. Nếu có cảnh báo về quyền trên macOS, xem thêm [Khắc phục sự cố](troubleshooting.md).

---

## 🔍 Bước 2: Xem thông tin bàn phím (`monkey info`)

Để kiểm tra xem hệ thống nhận diện đúng thông tin chip điều khiển không, chạy:

```bash
monkey info
```

Kết quả hiển thị các thông số định danh như:
- **VID / PID:** `0x05ac:0x024f`
- **Tên sản phẩm:** `RKGK890`
- **Interfaces:** Interface A (OUT 4096 bytes) và Interface B (Feature 64 bytes).

*(Thao tác này là **read-only**, an toàn tuyệt đối 100% và không làm thay đổi trạng thái bàn phím).*

---

## 🧪 Bước 3: Chạy thử nghiệm giả lập (Chế độ `--mock`)

Nếu bạn chưa muốn can thiệp vào bàn phím thật hoặc muốn kiểm tra trước kết quả xử lý ảnh/màu sắc:

```bash
# Thử nghiệm pipeline load ảnh, crop và nạp giả lập lên LCD
monkey lcd image assets/sample.png --mock

# Thử nghiệm pipeline giải mã và tạo frame ảnh động GIF
monkey lcd anim assets/cat.gif --mock

# Thử nghiệm cấu hình màu sắc RGB
monkey rgb set wave --speed 50 --brightness 80 --mock
```

---

## 🖥️ Bước 4: Nạp ảnh và GIF lên màn hình LCD thật

> ⚠️ **Lưu ý:** Thao tác trên thiết bị thật **bắt buộc phải có cờ `--allow-hardware-writes`**.

### 1. Nạp ảnh tĩnh
Chuẩn bị một ảnh PNG, JPEG hoặc BMP bất kỳ (hệ thống sẽ tự động crop và resize về kích thước 128x128):

```bash
monkey lcd image my_avatar.png --allow-hardware-writes
```

*Mẹo:* Nếu ảnh có nhiều dải màu chuyển tiếp (gradient), hãy thêm cờ `--dither` để kích hoạt thuật toán khuếch tán lỗi Floyd-Steinberg, giúp ảnh mượt hơn trên chuẩn màu 16-bit (RGB565):
```bash
monkey lcd image my_avatar.png --dither --allow-hardware-writes
```

### 2. Phát ảnh động GIF
```bash
monkey lcd anim my_animation.gif --allow-hardware-writes
```
- Quá trình này sẽ chạy trực tiếp ở terminal với thanh tiến trình hiển thị FPS thực tế.
- Để dừng phát, nhấn `Ctrl+C`. Trình phát sẽ ngắt an toàn.

---

## 🌈 Bước 5: Điều khiển hiệu ứng LED RGB

### 1. Đổi màu tạm thời (RAM Preview)
Mặc định, lệnh `monkey rgb set` chỉ cập nhật vào bộ nhớ RAM của bàn phím:
```bash
# Đổi toàn bộ phím sang màu tĩnh (Cyan)
monkey rgb set static --color 00FFFF --brightness 100 --allow-hardware-writes

# Hiệu ứng sóng lượn (Wave)
monkey rgb set wave --speed 70 --brightness 100 --allow-hardware-writes
```
*Đặc tính:* Màu sắc sẽ hiển thị ngay lập tức, nhưng nếu bạn rút cáp hoặc tắt phím, cài đặt này sẽ không lưu lại. Điều này giúp bảo vệ tối đa tuổi thọ chip nhớ Flash SPI NOR của bàn phím.

### 2. Lưu cấu hình vĩnh viễn vào Flash
Khi đã ưng ý với màu sắc, thêm cờ `--commit` để lưu lâu dài:
```bash
monkey rgb set wave --speed 70 --brightness 100 --commit --allow-hardware-writes
```

---

## 📖 Đọc tiếp theo
- Xem danh sách đầy đủ tất cả các lệnh và cờ trong [Bảng tra cứu lệnh CLI](cli-reference.md).
- Tìm hiểu các vấn đề thường gặp tại [Khắc phục sự cố](troubleshooting.md).
