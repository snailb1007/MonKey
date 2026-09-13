---
status: testing
phase: 03-high-performance-lcd-rendering-streaming-engine
source: [03-01-SUMMARY.md, 03-02-SUMMARY.md, 03-03-SUMMARY.md]
started: 2026-09-13T12:38:50.947Z
updated: 2026-09-13T12:38:50.947Z
---

## Current Test

number: 1
name: Chạy mẫu LCD bằng mock
expected: |
  Chạy `rtk cargo run -p monkey-cli -- --json lcd test-pattern rgb-bars --mock`.
  CLI trả JSON với operation=test-pattern, target=mock, frames=1,
  chunks_per_frame=8, bytes_sent=32768 và kết thúc thành công.
awaiting: user response

## Tests

### 1. Chạy mẫu LCD bằng mock
expected: CLI trả JSON với operation=test-pattern, target=mock, frames=1, chunks_per_frame=8, bytes_sent=32768 và kết thúc thành công.
result: [pending]

### 2. Chặn ghi thiết bị khi chưa cho phép
expected: Chạy lcd test-pattern rgb-bars không có --mock và không có --allow-hardware-writes trả lỗi yêu cầu cho phép ghi hoặc dùng mock; không thay đổi LCD.
result: [pending]

### 3. Nạp ảnh tĩnh và báo tiến trình
expected: lcd image với ảnh hợp lệ và --mock hoàn tất, báo 1 frame, 8 chunks/frame, 32768 bytes; chế độ thường có tiến trình, --json trả dữ liệu có cấu trúc; --dither được phản ánh trong báo cáo.
result: [pending]

### 4. Phát GIF và giới hạn FPS
expected: lcd anim với GIF nhiều frame và --mock phát hết rồi thoát, báo đúng số frame và 32768 bytes/frame; --fps nhận 10–15 và từ chối giá trị ngoài khoảng.
result: [pending]

### 5. Dừng vòng lặp GIF
expected: lcd anim với GIF, --mock và --loop chạy lặp; Ctrl-C dừng và trả quyền điều khiển terminal cùng báo cáo frame đã gửi.
result: [pending]

### 6. Quan sát mẫu màu và ảnh trên LCD thật
expected: Khi có thiết bị phù hợp và người dùng chủ động cho phép ghi, LCD hiển thị đúng các mẫu màu, grayscale và geometry; ảnh được crop giữa về 128x128, không méo hoặc lệch màu. Mock không xác nhận được kết quả này.
result: [pending]

### 7. Quan sát GIF trên LCD thật
expected: Khi có thiết bị phù hợp và người dùng chủ động cho phép ghi, GIF hiển thị chuyển động đúng thứ tự, không có frame hỏng; tốc độ mục tiêu 10–15 FPS cần đối chiếu quan sát hoặc đo trên thiết bị thật, không suy ra từ mock.
result: [pending]

## Summary

total: 7
passed: 0
issues: 0
pending: 7
skipped: 0
blocked: 0

## Notes

- Canonical verification passed at initialization; UAT has not passed.
- verify:pre API coverage gate passed.
- UI checkpoints: 0 auto-verified; hardware visual outcomes require manual review.
- SUMMARY files use legacy coverage; no test automatically marked passed.
- Current source uses --allow-hardware-writes and default 3ms chunk pacing (0–8ms), differing from the older 03-03 summary. Checkpoints use current source syntax.

## Gaps

<!-- No user-reported gaps yet. -->
