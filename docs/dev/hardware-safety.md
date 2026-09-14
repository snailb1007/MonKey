# Cơ chế an toàn phần cứng (Hardware Safety Rails)

Tài liệu này giải thích chi tiết các rào chắn kỹ thuật được triển khai trong `monkey-core` nhằm bảo vệ bàn phím khỏi nguy cơ hư hỏng firmware (brick) hoặc mòn chip nhớ Flash SPI NOR.

---

## 🎯 Triết lý an toàn: "Defense-in-Depth"

Khác với các ứng dụng phần mềm thuần túy, thao tác sai lệch trên phần cứng nhúng (MCU/Flash) có thể dẫn tới hậu quả không thể đảo ngược:
1. **Ghi đè hoặc kích hoạt bootloader sai:** Làm MCU rơi vào trạng thái ISP vĩnh viễn, bàn phím không nhận phím.
2. **Ghi Flash quá nhiều lần (Flash wear-out):** Chip nhớ SPI NOR flash có tuổi thọ giới hạn (khoảng 10.000 đến 100.000 chu kỳ ghi/xóa cho mỗi sector). Nếu mỗi lần kéo thanh trượt đổi màu hoặc stream frame lại ghi trực tiếp vào Flash, chip sẽ hỏng trong vài ngày.
3. **Mất nguồn giữa chừng (Brown-out):** Khi bàn phím đang chạy pin không dây, nếu pin tụt đột ngột trong lúc đang commit Flash, sector nhớ bị ghi dở dang sẽ làm hỏng dữ liệu khởi động.

Để ngăn chặn các rủi ro trên, MonKey triển khai một hệ thống phòng thủ nhiều lớp:

---

## 1. Lọc mã lệnh theo danh sách trắng (`SafetyRails` Opcode Whitelist)

Tất cả các gói tin trước khi gửi qua `TransactionManager` đều phải đi qua bộ kiểm duyệt `SafetyRails`:

- **Chính sách Default-Deny:** Bất kỳ opcode nào không nằm trong danh sách lệnh đã đo kiểm (`0x02`, `0x18`, `0x13`, `0x20`, `0xF0`, `0xF5`) đều bị từ chối với lỗi `MonkeyError::SafetyViolation`.
- **Cách ly Bootloader:** Firmware của Shenzhen HFD sử dụng PID `0x7140` khi ở chế độ nạp flash (ISP mode). Tầng discovery chủ động cách ly và không gửi lệnh cấu hình thông thường tới thiết bị khi đang ở PID này.

---

## 2. Quản lý trạng thái hai tầng (RAM Preview vs. Flash Commit)

MonKey phân tách rạch ròi giữa việc "hiển thị hiệu ứng tức thì" và việc "lưu giữ cài đặt lâu dài":

```
Người dùng chạy lệnh:
  monkey rgb set wave
        │
        ▼
  ┌───────────────────────────────────┐
  │         RAM Preview Mode          │
  │  - Gửi cấu hình vào RAM tạm       │
  │  - Zero Flash writes (Không mòn)  │
  │  - Hiển thị ngay lập tức          │
  └───────────────────────────────────┘

Người dùng thêm cờ --commit:
  monkey rgb set wave --commit
        │
        ▼
  ┌───────────────────────────────────┐
  │        Flash Commit Mode          │
  │  - Kiểm tra điều kiện an toàn pin │
  │  - Kiểm tra khoảng cách 500ms     │
  │  - Gửi lệnh 04 F5 ghi vào Flash   │
  └───────────────────────────────────┘
```

### Giới hạn tần suất Flash Commit (Throttling)
- Trong phạm vi phiên chạy (`SafetyRails` session), hệ thống ghi lại mốc thời gian của lần ghi Flash gần nhất (`Instant`).
- Nếu hai lệnh commit diễn ra cách nhau dưới **500ms**, lệnh thứ hai sẽ bị chặn lại để tránh ghi dồn dập vào cùng một sector Flash.
- *Ghi chú kỹ thuật minh bạch:* Do CLI chạy theo từng tiến trình độc lập, cơ chế giới hạn này hiện áp dụng trong phiên làm việc của tiến trình đó hoặc khi tích hợp vào daemon/GUI tương lai.

---

## 3. Rào chắn bảo vệ nguồn điện pin yếu (Battery Gate)

- Khi thiết bị hoạt động qua kết nối không dây (2.4GHz Dongle):
  - Lệnh nạp Flash (`--commit`) kiểm tra mức pin hiện tại.
  - Nếu mức pin báo cáo **dưới 20%**, thao tác ghi Flash bị khóa ngay lập tức và trả về cảnh báo an toàn.
  - Người dùng có thể cắm cáp Type-C để tiếp tục, hoặc cố ý vượt qua rào chắn bằng cờ `--force` (tự chấp nhận rủi ro sụt nguồn).

---

## 4. Cờ cấp quyền ghi phần cứng rõ ràng (`--allow-hardware-writes`)

- Để ngăn chặn việc vô tình gửi dữ liệu sai làm biến đổi phần cứng, mọi lệnh `monkey lcd` và `monkey rgb set` trên phần cứng thật bắt buộc phải có cờ `--allow-hardware-writes`.
- Nếu thiếu cờ này, CLI sẽ dừng lại ngay trước khi mở cổng HID và yêu cầu người dùng xác nhận lại hoặc chuyển sang dùng cờ `--mock`.

---

## 5. Xử lý tín hiệu ngắt an toàn (Clean Signal Handling)

- Khi đang stream ảnh động GIF lên LCD bằng `monkey lcd anim`, người dùng có thể nhấn `Ctrl+C` bất cứ lúc nào.
- Trình phát sử dụng cờ nguyên tử `AtomicBool` để bắt tín hiệu `SIGINT`, kết thúc vòng lặp gửi chunk hiện tại một cách trọn vẹn trước khi thoát tiến trình, tránh tình trạng để lại một chunk dở dang trong bộ đệm của MCU.
- Trong luồng cấu hình RGB, nếu xảy ra lỗi giữa chừng, `TransactionManager` gửi tường minh gói tin `04 F0` (EndTransaction) để báo cho bàn phím thoát khỏi trạng thái chờ lệnh.

---

## ⚠️ Giới hạn & Tuyên bố trách nhiệm (Disclaimer)

Mặc dù `monkey-core` áp dụng tất cả các biện pháp phòng vệ tốt nhất có thể dựa trên mã nguồn đã đo kiểm, việc reverse engineering phần cứng độc quyền luôn tiềm ẩn những trường hợp ngoại lệ từ các biến thể firmware chưa rõ của nhà sản xuất. Người dùng và nhà phát triển được khuyến cáo:
- Luôn giữ bàn phím cắm cáp và đủ pin khi thực hiện các thao tác ghi Flash.
- Không tự ý bypass `SafetyRails` để gửi dữ liệu tùy tiện vào thiết bị.
