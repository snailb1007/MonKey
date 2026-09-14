# Hướng dẫn cài đặt MonKey

Tài liệu này cung cấp hướng dẫn chi tiết cách cài đặt CLI `monkey` trên macOS, Linux và Windows.

---

## 📦 Phương thức cài đặt hiện tại: Build từ mã nguồn (Source)

Ở giai đoạn hiện tại, MonKey được phân phối dưới dạng mã nguồn Rust. Các gói cài đặt nhị phân dựng sẵn (pre-built binaries) và package managers (Homebrew, AUR) đang được hoàn thiện cho các bản phát hành chính thức tiếp theo.

### 1. Cài đặt Rust Toolchain

Nếu máy tính của bạn chưa có Rust, hãy cài đặt qua [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Khuyến nghị sử dụng phiên bản Rust `1.80.0` trở lên:
```bash
rustup update stable
```

---

### 2. Cài đặt các gói phụ thuộc hệ thống (System Dependencies)

#### Trên macOS
- Cần có Xcode Command Line Tools:
  ```bash
  xcode-select --install
  ```
- MonKey sử dụng trực tiếp framework `IOKit` và `CoreFoundation` có sẵn của macOS thông qua `hidapi`, không cần cài thêm thư viện ngoài.

#### Trên Linux (Ubuntu / Debian)
- Cần cài đặt thư viện phát triển USB và udev:
  ```bash
  sudo apt-get update
  sudo apt-get install -y build-essential pkg-config libusb-1.0-0-dev libudev-dev
  ```

#### Trên Fedora / RHEL
  ```bash
  sudo dnf install systemd-devel libusb1-devel
  ```

#### Trên Arch Linux
  ```bash
  sudo pacman -S systemd libusb
  ```

---

### 3. Biên dịch và Cài đặt

#### Cách 1: Cài đặt trực tiếp vào thư mục Cargo bin (Khuyên dùng)

Lệnh này sẽ biên dịch bản tối ưu (release) và tự động đặt file thực thi `monkey` vào thư mục `$HOME/.cargo/bin`:

```bash
# Clone mã nguồn
git clone https://github.com/snailb1007/MonKey.git
cd MonKey

# Cài đặt CLI
cargo install --path crates/monkey-cli
```

*Lưu ý:* Hãy đảm bảo `$HOME/.cargo/bin` đã được thêm vào biến môi trường `PATH` trong file cấu hình shell (`.zshrc` hoặc `.bashrc`):
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

#### Cách 2: Tự build file thực thi thủ công

Nếu bạn không muốn cài đặt toàn cục:

```bash
cargo build --release
```
File thực thi sau khi biên dịch xong sẽ nằm tại:
```text
./target/release/monkey
```
Bạn có thể copy file này vào `/usr/local/bin` hoặc bất kỳ thư mục nào trong `PATH`.

---

### 4. Kiểm tra cài đặt

Sau khi cài đặt thành công, kiểm tra bằng lệnh:

```bash
monkey --version
```
Và chạy lệnh tự kiểm tra môi trường:
```bash
monkey doctor
```

---

## 🗺️ Kế hoạch phân phối trong tương lai (Roadmap)

Trong các bản phát hành chính thức (Milestone v1.0+), MonKey sẽ hỗ trợ thêm các hình thức cài đặt tiện lợi:
- **GitHub Releases:** Tải trực tiếp binary dựng sẵn cho macOS (Apple Silicon `aarch64` & Intel `x86_64`), Linux (`x86_64`) và Windows (`x86_64`).
- **Homebrew Tap:** `brew install snailb1007/tap/monkey` dành cho người dùng macOS.
- **Shell Completions tự động:** Hỗ trợ sinh script gợi ý lệnh cho Zsh, Bash, Fish.
