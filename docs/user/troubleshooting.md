# Hướng dẫn khắc phục sự cố (Troubleshooting)

Tài liệu này tập hợp các lỗi phổ biến khi sử dụng MonKey và hướng dẫn từng bước xử lý.

---

## 🩺 1. Sử dụng công cụ chẩn đoán tự động (`monkey doctor`)

Bất cứ khi nào gặp sự cố về kết nối hoặc quyền hạn, hãy chạy lệnh đầu tiên:

```bash
monkey doctor
```

Lệnh sẽ kiểm tra 4 thành phần quan trọng:
1. **Platform Diagnostics:** Nhận diện hệ điều hành và phiên bản hạt nhân.
2. **Device Discovery:** Kiểm tra xem bàn phím có cắm vào máy không (tìm VID `0x05AC` và PID `0x024F`).
3. **Interface A Accessibility:** Kiểm tra khả năng mở vendor bulk pipe (màn hình LCD).
4. **Interface B Accessibility:** Kiểm tra khả năng truy cập kênh cấu hình RGB.

---

## 🔒 2. Sự cố quyền hạn trên macOS (Permission Denied)

### Nguyên nhân
Trên macOS, Interface B (`0xFFFF`) của Monka 3075 Pro được định nghĩa chung trong composite device với **Consumer Control** (`0x0C`) và **Mouse** (`0x01`). Do đó, cơ chế bảo vệ macOS TCC (`com.apple.TCC`) có thể yêu cầu cấp quyền **Input Monitoring** (`kTCCServiceListenEvent`) để ứng dụng giao tiếp với giao diện này.

### Cách khắc phục:
1. Mở **System Settings** (Cài đặt hệ thống) trên macOS.
2. Điều hướng tới **Privacy & Security** (Quyền riêng tư & Bảo mật) -> **Input Monitoring** (Theo dõi đầu vào).
3. Bật cấp quyền cho ứng dụng Terminal bạn đang sử dụng (ví dụ: `Terminal.app`, `iTerm2`, `Ghostty`, hoặc `VS Code`).
4. Khởi động lại ứng dụng Terminal và chạy lại `monkey doctor`.

---

## 🖥️ 3. Màn hình LCD bị treo hoặc hiển thị sai màu

### Trường hợp 1: Màn hình dừng lại ở một khung hình sau khi tắt terminal
- **Giải thích:** Khi bạn stream ảnh động GIF bằng lệnh `monkey lcd anim`, tiến trình CLI liên tục gửi từng khung hình. Nếu bạn tắt cửa sổ terminal đột ngột mà không nhấn `Ctrl+C`, bàn phím sẽ giữ nguyên khung hình cuối cùng được nạp vào bộ đệm RAM của màn hình.
- **Cách xử lý:** 
  - Gửi lại một ảnh tĩnh sạch:
    ```bash
    monkey lcd image path/to/image.png --allow-hardware-writes
    ```
  - Hoặc nạp mẫu kiểm tra màu để reset hiển thị:
    ```bash
    monkey lcd test-pattern rgb-bars --allow-hardware-writes
    ```

### Trường hợp 2: Màu sắc bị ngược hoặc hiển thị đốm hạt
- **Giải thích:** Bàn phím sử dụng chuẩn màu 16-bit RGB565 theo thứ tự byte Little-Endian. Nếu bạn thấy màu bị sai lệch nghiêm trọng, có thể cờ Endian đang bị ép sai.
- **Cách xử lý:** Thử bỏ cờ `--big-endian` nếu đang bật, và thêm cờ `--dither` để kích hoạt thuật toán khuếch tán điểm màu:
  ```bash
  monkey lcd image my_avatar.png --dither --allow-hardware-writes
  ```

---

## 🚫 4. Lỗi: "LCD commands write to hardware. Re-run with --allow-hardware-writes"

- **Giải thích:** Đây là tính năng an toàn có chủ đích của MonKey. Để ngăn ngừa người dùng vô tình chạy lệnh ghi đè phần cứng hoặc gửi dữ liệu sai, mọi lệnh thay đổi LCD hoặc LED RGB trên thiết bị thật đều yêu cầu cờ đồng ý rõ ràng.
- **Cách xử lý:** Thêm cờ `--allow-hardware-writes` vào cuối lệnh của bạn:
  ```bash
  monkey lcd image photo.png --allow-hardware-writes
  monkey rgb set static --color FF0000 --allow-hardware-writes
  ```

---

## 🔌 5. Không tìm thấy bàn phím (Device Not Found)

Nếu `monkey info` hoặc `monkey doctor` báo không tìm thấy thiết bị:
1. **Kiểm tra công tắc kết nối:** Đảm bảo gạt công tắc ở mặt đáy bàn phím về nấc **USB / Wired** (ở giữa), không để ở nấc 2.4G hoặc BT khi đang cắm dây.
2. **Tránh cắm qua Hub USB không có nguồn:** Màn hình LCD và dàn LED RGB tiêu thụ dòng điện khá lớn. Việc cắm qua các hub chia cổng USB rẻ tiền có thể gây thiếu dòng hoặc chập chờn đường truyền USB HID. Hãy cắm trực tiếp vào cổng máy tính hoặc dùng hub Type-C có nguồn riêng.
3. **Cáp USB:** Hãy đảm bảo bạn đang dùng cáp có hỗ trợ truyền dữ liệu (Data Cable), không phải cáp chỉ dùng để sạc pin (Charge-only).
