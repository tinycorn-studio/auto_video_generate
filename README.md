# Wavely

Wavely ghép một ảnh dọc với file ghi âm thành video MP4 có sóng âm đồng bộ. Media được xử lý trực tiếp trên Windows hoặc macOS; không cần tài khoản, API AI hay tải file lên máy chủ.

## Yêu cầu hệ thống

- Windows 10/11 64-bit hoặc macOS 13 trở lên.
- FFmpeg 6 trở lên, có các filter `showwaves`, `showfreqs` và encoder `libx264`, `aac`.
- Đầu vào: JPG/PNG/WebP/BMP và WAV/MP3/M4A/AAC/FLAC/OGG/Opus.
- Kết nối Internet chỉ cần khi cài công cụ phát triển hoặc cập nhật dependency; thao tác tạo video chạy offline.

Wavely không tải FFmpeg ngầm và không kèm một bản FFmpeg bên thứ ba trong installer. Trên Windows có thể cài bản đầy đủ bằng `winget install --id Gyan.FFmpeg -e`; trên macOS dùng `brew install ffmpeg` nếu đã cài Homebrew. Sau khi cài, bảo đảm lệnh `ffmpeg` có trong `PATH` rồi mở lại Wavely. Có thể tải thủ công từ [ffmpeg.org](https://ffmpeg.org/download.html).

## Build và chạy

### Công cụ cần cài

- Node.js 20 trở lên và npm.
- Rust stable với Cargo.
- Windows: Microsoft C++ Build Tools và WebView2 Runtime.
- macOS: Xcode Command Line Tools.
- FFmpeg 6 trở lên cho tính năng xuất video.

### Chạy giao diện desktop khi phát triển

```bash
npm ci
npm run tauri:dev
```

### Tạo installer trên máy hiện tại

```bash
npm run tauri:build
```

Tauri chỉ tạo bundle cho hệ điều hành đang build. Installer Windows/macOS cho bản phát hành được GitHub Actions tạo theo từng runner sau khi đẩy tag `vX.Y.Z`; xem [RELEASE.md](RELEASE.md).

## Tính năng

- Chọn một ảnh và một file âm thanh bằng hộp thoại native.
- Xem trước ảnh, nghe thử audio và xem thời lượng.
- Chọn nhanh đường sóng Tối giản hoặc cột phổ Spectrum.
- Xuất MP4 dọc 1080 × 1920 ở 30 fps, H.264/AAC, CRF mặc định 21.
- Theo dõi tiến độ, hủy render; video chỉ được đưa vào vị trí đích sau khi xuất thành công.
- Tự căn giữa và crop ảnh để lấp khung 9:16, không kéo méo ảnh.

## Phím tắt

Chưa có phím tắt riêng trong phiên bản đầu.

## Kiến trúc

- `src/`: giao diện TypeScript/CSS chạy trên WebView hệ thống của Tauri.
- `src-tauri/`: lệnh Rust chọn cấu hình filtergraph, gọi FFmpeg bằng tham số tách rời, đọc tiến độ và quản lý hủy.
- `VERSION`: nguồn version duy nhất; `scripts/sync-version.mjs` đồng bộ metadata npm, Cargo và Tauri trước khi build.
- `PLAN.md`: mục tiêu, tiêu chí chất lượng và trạng thái các mốc triển khai.

FFmpeg là tiến trình native riêng, không phải thư viện nhúng vào Rust. V1 dò FFmpeg trong `PATH` và các vị trí cài phổ biến; file ảnh, audio, video tạm và video cuối đều ở trên máy.

## Cấu hình và giới hạn

- Đầu ra hiện cố định ở MP4 1080 × 1920, 30 fps, H.264/AAC.
- Màu sắc và filtergraph được đặt theo preset; chưa có bảng chỉnh hàng loạt thông số.
- V1 không tạo phụ đề, không nhận diện thể loại, không thêm nhạc nền, reverb, blur, grain hoặc hiệu ứng nặng.
- Halo chưa được triển khai; hai kiểu sóng dùng filter FFmpeg native.
- Chất lượng/tốc độ phụ thuộc cấu hình FFmpeg và CPU; cần benchmark Windows/macOS trước khi công bố số liệu hiệu năng.
- Installer chưa được ký/notarize. macOS có thể yêu cầu mở lần đầu bằng chuột phải → **Open**; Windows có thể hiện cảnh báo SmartScreen.

## Lịch sử phiên bản

Phiên bản hiện tại: **0.1.0**. Chi tiết ở [CHANGELOG.md](CHANGELOG.md).

## Tải về

Installer được phát hành trong mục [GitHub Releases](https://github.com/tinycorn-studio/auto_video_generate/releases). Khi có tag SemVer mới, GitHub Actions sẽ build installer Windows x64 và DMG cho macOS Intel/Apple Silicon, rồi đính kèm vào release tương ứng. Bản hiện tại chưa có installer cho tới khi CI build và smoke test hoàn tất.
