# Bảng tra cứu lệnh CLI (CLI Reference)

Tài liệu này cung cấp danh mục chi tiết tất cả các lệnh, cờ (flags) và tham số của công cụ `monkey`.

---

## 🌐 Các cờ toàn cục (Global Options)

Những cờ này có thể áp dụng cho bất kỳ lệnh con nào:

| Cờ | Ý nghĩa |
| :--- | :--- |
| `--json` | Định dạng kết quả đầu ra thành JSON có cấu trúc (dành cho scripting/tự động hóa). |
| `-v, --verbose` | Tăng mức độ log chi tiết (`-v` cho DEBUG, `-vv` cho TRACE). |
| `-h, --help` | Hiển thị hướng dẫn trợ giúp cho lệnh tương ứng. |
| `-V, --version` | Hiển thị phiên bản hiện tại của phần mềm. |

---

## 1. Nhóm lệnh chuẩn đoán & thông tin

### `monkey doctor`
Kiểm tra sức khỏe hệ thống, quyền truy cập USB và trạng thái kết nối tới 2 interface của bàn phím.

```bash
monkey doctor
monkey doctor --json
```

### `monkey info`
Truy vấn và hiển thị định danh phần cứng USB (VID, PID, số serial, mô tả giao diện) theo chế độ chỉ đọc (read-only).

```bash
monkey info
monkey info --json
```

### `monkey probe`
Gửi các truy vấn đọc không phá hủy để thăm dò phiên bản firmware và trạng thái truyền tải (wired vs wireless).

```bash
monkey probe
```

### `monkey bench`
Đo lường thông lượng truyền dữ liệu màn hình (bulk transfer) và độ trễ phản hồi gói tin cấu hình (feature report round-trip).

```bash
# Chạy benchmark giả lập trong bộ nhớ
monkey bench --mock

# Chạy benchmark trên phần cứng thật (cần cờ cho phép ghi)
monkey bench --allow-hardware-writes --iterations 10
```

Các tùy chọn:
- `--iterations <N>`: Số lượt lặp đo lường (mặc định: 5).
- `--inter-chunk-delay-ms <N>`: Độ trễ giữa các chunk (mặc định: 3ms).
- `--mock`: Chạy trên mock transport.
- `--allow-hardware-writes`: Cho phép ghi gói tin đo lường lên phần cứng thật.

---

## 2. Nhóm lệnh màn hình LCD (`monkey lcd`)

Giao tiếp với màn hình TFT 128×128 (chuẩn màu 16-bit RGB565) qua Vendor Bulk Pipe (`0xFF68`). Mỗi khung hình gồm 32.768 bytes, chia thành 8 chunk (mỗi chunk 4096 bytes).

### `monkey lcd image <PATH>`
Tải một ảnh tĩnh (PNG, JPEG, BMP), tự động crop tỷ lệ 1:1, resize về 128×128 và truyền vào màn hình.

```bash
monkey lcd image photo.png --allow-hardware-writes
monkey lcd image photo.png --dither --allow-hardware-writes
```

Các tùy chọn:
- `--dither`: Kích hoạt giải thuật khuếch tán lỗi Floyd-Steinberg, giảm hiện tượng gợn màu (color banding) trên màn hình 16-bit.
- `--big-endian`: Đổi thứ tự byte màu sang Big-Endian (mặc định là Little-Endian).
- `--inter-chunk-delay-ms <N>`: Khoảng nghỉ giữa các chunk 4096B (từ 0–8ms, mặc định: 3ms để tránh tràn bộ đệm MCU).
- `--mock`: Chạy giả lập trong bộ nhớ.
- `--allow-hardware-writes`: **Bắt buộc** khi ghi lên phần cứng thật.

### `monkey lcd anim <PATH>`
Giải mã và phát ảnh động GIF lên màn hình LCD ở foreground.

```bash
# Phát GIF theo tốc độ mặc định (12 FPS)
monkey lcd anim animation.gif --allow-hardware-writes

# Tùy chỉnh tốc độ khung hình và số vòng lặp
monkey lcd anim animation.gif --fps 15 --loop 3 --allow-hardware-writes
```

Hành vi:
- Hiển thị thanh tiến trình và FPS thực tế trên terminal.
- Để dừng phát, nhấn `Ctrl+C`. CLI sẽ bắt tín hiệu và ngắt truyền an toàn.

### `monkey lcd test-pattern <PATTERN>`
Hiển thị các mẫu kiểm tra hình học và màu sắc tích hợp sẵn để căn chỉnh hiển thị:

```bash
monkey lcd test-pattern rgb-bars --allow-hardware-writes
```

Các mẫu hợp lệ (`<PATTERN>`):
- `rgb-bars`: Dải màu Đỏ - Lục - Lam kiểm tra đúng kênh màu.
- `geometry`: Khung lưới kiểm tra viền và tỷ lệ hiển thị.
- `red`, `green`, `blue`, `white`, `black`: Màn hình đơn sắc kiểm tra điểm chết.
- `grayscale`: Bảng sắc độ xám kiểm tra độ tương phản.

---

## 3. Nhóm lệnh điều khiển LED RGB (`monkey rgb`)

Giao tiếp với chip điều khiển LED thông qua 64-byte Feature Reports trên Interface B (`0xFFFF`).

### `monkey rgb set <MODE>`
Thiết lập hiệu ứng, màu sắc, tốc độ và độ sáng.

```bash
# Đổi màu tĩnh sang Cyan (00FFFF)
monkey rgb set static --color 00FFFF --brightness 100 --allow-hardware-writes

# Hiệu ứng sóng lượn (Wave)
monkey rgb set wave --speed 60 --brightness 80 --direction left-to-right --allow-hardware-writes

# Tắt LED
monkey rgb set off --allow-hardware-writes

# Lưu vĩnh viễn cấu hình vào bộ nhớ Flash
monkey rgb set breathing --color FF5500 --commit --allow-hardware-writes
```

Các tham số và cờ:
- `<MODE>`: Chế độ ánh sáng (`static`, `breathing`, `wave`, `rainbow`, `ripple`, `reactive`, `off`).
- `-c, --color <HEX|NAME>`: Mã màu Hex (`#RRGGBB` hoặc `RRGGBB`) hoặc tên màu (`red`, `green`, `blue`, `cyan`, `magenta`, `yellow`, `white`).
- `-b, --brightness <0..=100>`: Mức độ sáng (0 đến 100, mặc định: 100).
- `-s, --speed <0..=100>`: Tốc độ hiệu ứng (0 đến 100, mặc định: 50).
- `-d, --direction <DIR>`: Hướng chuyển động (`left-to-right` hoặc `right-to-left`).
- `--commit`: Ghi cấu hình vĩnh viễn vào chip nhớ Flash SPI NOR (nếu không có cờ này, lệnh chỉ xem trước trên RAM tạm thời).
- `--force`: Bỏ qua cảnh báo chặn commit khi pin dưới 20% trên kết nối không dây.
- `--mock`: Chạy giả lập trong bộ nhớ.
- `--allow-hardware-writes`: **Bắt buộc** khi ghi lên phần cứng thật.

### `monkey rgb status`
Xem trạng thái cấu hình ánh sáng hiện tại (được lưu trong session).

```bash
monkey rgb status
monkey rgb status --json
```

### `monkey rgb save`
Xuất cấu hình ánh sáng hiện tại ra file JSON hoặc in trực tiếp ra stdout:

```bash
monkey rgb save --file ./my_rgb.json
monkey rgb save --stdout
```

### `monkey rgb restore`
Nạp cấu hình ánh sáng từ file JSON đã lưu vào bàn phím:

```bash
monkey rgb restore --file ./my_rgb.json --allow-hardware-writes
monkey rgb restore --file ./my_rgb.json --commit --allow-hardware-writes
```

---

## 4. Lệnh sinh Shell Completions (`monkey completions`)

Tạo script tự động gợi ý phím Tab cho shell của bạn:

```bash
# Dành cho Zsh (thêm vào ~/.zshrc)
monkey completions zsh > ~/.zfunc/_monkey

# Dành cho Bash
monkey completions bash > /etc/bash_completion.d/monkey

# Dành cho Fish
monkey completions fish > ~/.config/fish/completions/monkey.fish
```

Hỗ trợ các shell: `bash`, `zsh`, `fish`, `elvish`, `powershell`.
