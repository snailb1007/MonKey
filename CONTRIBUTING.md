# Hướng dẫn đóng góp cho MonKey (Contributing Guide)

Cảm ơn bạn đã quan tâm đến việc phát triển và hoàn thiện **MonKey**. Dự án hoan nghênh mọi đóng góp từ sửa lỗi nhỏ, bổ sung tài liệu đến việc phân tích gói tin USB và mở rộng hỗ trợ cho các bàn phím cùng hệ chip.

---

## 🧭 Nguyên tắc đóng góp cốt lõi

1. **An toàn phần cứng là ưu tiên số một:**
   - Tuyệt đối không gửi opcode lạ hoặc suy đoán chưa qua kiểm chứng vào thiết bị thật.
   - Không commit code có nguy cơ kích hoạt chế độ ISP bootloader (`0x7140`).
   - Mọi thay đổi logic giao thức phải được bao bọc bởi `SafetyRails`.
2. **Minh bạch mức độ xác minh:**
   - Khi ghi chú về protocol hay hardware findings, luôn phân loại rõ:
     - `[Đã đo kiểm trên phần cứng]`
     - `[Suy luận từ capture]`
     - `[Chưa kiểm chứng]`
3. **Phát triển test-driven với MockTransport:**
   - Mọi tính năng giao thức hoặc CLI mới phải có unit/integration test chạy được trong môi trường CI không có phần cứng vật lý (dùng `MockTransport`).

---

## 🛠️ Thiết lập môi trường phát triển

### 1. Yêu cầu
- **Rust Toolchain:** Phiên bản 1.80 trở lên (`rustup default stable`).
- **Nền tảng:** macOS (được ưu tiên thử nghiệm với `IOKit`), Linux hoặc Windows.
- **Thư viện phụ trợ trên Linux:**
  ```bash
  sudo apt-get update
  sudo apt-get install -y libusb-1.0-0-dev libudev-dev pkg-config
  ```

### 2. Build Workspace
```bash
# Clone repository
git clone https://github.com/snailb1007/MonKey.git
cd MonKey

# Build toàn bộ workspace (monkey-core và monkey-cli)
cargo build

# Build ở chế độ release
cargo build --release
```

---

## 🧪 Kiểm thử và Tiêu chuẩn chất lượng (CI / Code Hygiene)

Trước khi gửi Pull Request, mã nguồn của bạn phải vượt qua toàn bộ các bài kiểm tra tự động sau:

### 1. Chạy bộ kiểm thử (Test Suite)
Dự án có sẵn hệ thống test mô phỏng phản hồi phần cứng mà không cần cắm bàn phím thật:

```bash
# Chạy tất cả unit tests và integration tests
cargo test
```

### 2. Kiểm tra tĩnh và Linter (`clippy`)
Dự án tuân thủ nghiêm ngặt các quy tắc linter:

```bash
cargo clippy --all-targets -- -D warnings
```

### 3. Kiểm tra định dạng (`rustfmt`)
```bash
cargo fmt --all -- --check
```

### 4. Kiểm tra bản quyền và phụ thuộc (`cargo-deny`)
```bash
cargo deny check
```

---

## 🔬 Đóng góp dữ liệu Reverse Engineering

Nếu bạn sở hữu bàn phím Monka 3075 Pro hoặc các dòng phím OEM Shenzhen HFD tương tự (`05AC:024F` / `RKGK890`), dữ liệu bắt gói USB (packet dumps) là đóng góp giá trị nhất:

1. Đặt các bản ghi capture thô, file `.pcapng` hoặc dump JSON vào thư mục `research/`.
2. Ghi rõ điều kiện kiểm thử:
   - Phiên bản firmware hiển thị trong app Windows OEM.
   - Chế độ kết nối (Cáp USB Type-C hay 2.4GHz Receiver).
   - Thao tác thực hiện khi bắt gói (ví dụ: "nhấn nút đổi sang chế độ LED Rainbow").
3. Không sửa đổi các ghi chú trong `docs/dev/` thành "đặc tả chuẩn" nếu chưa được kiểm chứng độc lập trên ít nhất một thiết bị thật.

---

## 🌿 Quy trình gửi Pull Request (PR)

1. Fork repository và tạo branch mới từ `dev`:
   ```bash
   git checkout -b feature/ten-tinh-nang
   ```
2. Thực hiện thay đổi, bổ sung test tương ứng.
3. Đảm bảo `cargo test` và `cargo clippy` đều sạch cảnh báo.
4. Viết commit message rõ ràng, tuân thủ Conventional Commits (ví dụ: `feat(lcd): add support for grayscale dithering`, `docs: update cli reference`).
5. Tạo Pull Request trỏ vào branch `dev` và mô tả rõ mục đích cũng như cách kiểm thử.
