# Changelog

Mọi thay đổi đáng chú ý của Wavely được ghi tại đây. Định dạng theo [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/) và phiên bản theo [SemVer](https://semver.org/lang/vi/).

## [Unreleased]

### Fixed

- Đọc đúng `capabilities.supports_json_output` khi kiểm tra model XPiki, tránh từ chối key có quyền dùng chat văn bản và JSON.

### Added

- Tạo tiêu đề, mô tả, hashtag và từ khóa bằng Gemini 3.8 Flash qua XPiki sau khi xuất video; mặc định English, hỗ trợ tiếng Việt và YouTube/TikTok/Facebook/Instagram.
- Lưu API key XPiki bằng Keychain/Credential Manager, kiểm tra quyền truy cập model trước khi lưu, thay/xóa key mà không lưu vào localStorage hay file cấu hình.
- Tab xem nội dung theo nền tảng, sao chép từng phần/tất cả và mở file/thư mục đầu ra.
- Xuất file `-noi-dung.txt` cạnh MP4; tránh ghi đè file cũ bằng hậu tố số. Lỗi Gemini không làm mất video; có thể thử lại nội dung mà không render lại.

### Changed

- Bố cục hai tab xem trước, preset gọn và phần chỉnh vị trí/kích thước sóng có thể thu gọn theo demo đã duyệt.
- Chỉ gửi transcript/tóm tắt do người dùng nhập tới XPiki khi bật Gemini; không gửi âm thanh/video, không lưu lịch sử bài đăng và không kiểm tra trùng lặp.

## [0.4.0] - 2026-10-23

### Added

- Thêm kiểu sóng Cân xứng với các nhịp vươn đều lên trên và xuống dưới trục giữa.
- Chọn vị trí wave theo lưới 3 × 3 và điều chỉnh kích thước từ 25% đến 100%; preview và filtergraph FFmpeg dùng cùng cấu hình.

## [0.3.0] - 2026-10-23

### Added

- Chọn khổ xuất video dọc 1080 × 1920 hoặc ngang 1920 × 1080; khung xem trước và vùng crop ảnh cập nhật theo lựa chọn.
- Chọn vị trí wave trên, giữa hoặc dưới video; vị trí trong khung xem trước khớp với vị trí khi xuất.

## [0.2.0] - 2026-10-23

### Added

- Cài engine FFmpeg một lần ngay trong Wavely theo nút bấm; tải đúng bản Windows/macOS, xác minh SHA-256, lưu tại app data và xóa archive tạm.
- Hiển thị tiến độ tải/cài đặt, trạng thái engine và thao tác hủy; sau khi cài, xuất video chạy offline.

### Changed

- Không còn yêu cầu người dùng cuối tự cài FFmpeg bằng winget/Homebrew; vẫn nhận FFmpeg sẵn có trên máy.
- Ghi rõ nhà build, phiên bản, hash, nguồn và license của từng FFmpeg runtime trong `THIRD_PARTY_NOTICES.md`.

## [0.1.0] - 2026-10-23

### Added

- Ứng dụng desktop Tauri cho Windows và macOS, giao diện tiếng Việt theo mẫu Tối giản đã duyệt.
- Chọn ảnh nền và file ghi âm bằng hộp thoại hệ điều hành; xem trước ảnh, nghe thử và đọc thời lượng âm thanh.
- Hai preset sóng âm: đường sóng Tối giản và cột tần số Spectrum.
- Xuất MP4 dọc 1080 × 1920, 30 fps, H.264/AAC; có tiến độ, hủy tác vụ và dọn file tạm khi lỗi.
- Quy trình GitHub Actions đóng gói installer Windows và DMG macOS khi đẩy tag SemVer.

### Changed

- Chốt `VERSION` làm nguồn version duy nhất; script đồng bộ metadata Tauri, Cargo và npm.
- Ghi release notes tiếng Việt từ mục tương ứng trong CHANGELOG.
- CI xác nhận build native trên Windows x64, macOS Intel và Apple Silicon trước khi phát hành installer.
- Cần cài FFmpeg 6 trở lên với `showwaves`, `showfreqs`, `libx264` và AAC; Wavely không tải FFmpeg ngầm hoặc đóng gói binary FFmpeg ở bản hiện tại.
- Chưa có chữ ký số/notarization; file cài trên release đầu sẽ có cảnh báo bảo mật của hệ điều hành.
- Chưa benchmark trên máy macOS; tốc độ xuất cần được đo trên các cấu hình tham chiếu.
