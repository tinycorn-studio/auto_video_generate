# Wavely

Wavely ghép một ảnh với file ghi âm thành video MP4 có sóng âm đồng bộ. Có thể xuất video dọc hoặc ngang và chọn nhanh vị trí wave. Media được xử lý trực tiếp trên Windows hoặc macOS; không cần tài khoản, API AI hay tải file lên máy chủ.

## Yêu cầu hệ thống

- Windows 10/11 64-bit hoặc macOS 13 trở lên.
- FFmpeg 6 trở lên, có các filter `showwaves`, `showfreqs` và encoder H.264/AAC. Có thể dùng FFmpeg cài sẵn hoặc tải engine trong ứng dụng.
- Đầu vào: JPG/PNG/WebP/BMP và WAV/MP3/M4A/AAC/FLAC/OGG/Opus.
- Kết nối Internet chỉ cần khi tải engine lần đầu hoặc cập nhật dependency; các lần tạo video sau đó chạy offline.

Nếu chưa có FFmpeg, bấm **Tải engine FFmpeg** trong Wavely. Ứng dụng chỉ tải sau khi bạn chủ động bấm nút, xác minh SHA-256 của gói FFmpeg 9.0.2, rồi lưu riêng file engine trong thư mục dữ liệu ứng dụng. Archive tạm được xóa sau cài đặt. Lần tải đầu cần Internet; các lần xuất tiếp theo chạy offline và file ảnh/âm thanh không rời máy. FFmpeg cài sẵn trong `PATH` vẫn được hỗ trợ. Có thể cài thủ công bằng `winget install --id Gyan.FFmpeg -e` trên Windows hoặc `brew install ffmpeg` trên macOS. Nguồn build và thông tin license được ghi trong [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Build và chạy

### Công cụ cần cài

- Node.js 20 trở lên và npm.
- Rust stable với Cargo.
- Windows: Microsoft C++ Build Tools và WebView2 Runtime.
- macOS: Xcode Command Line Tools.
- FFmpeg 6 trở lên cho tính năng xuất video; người dùng cuối có thể cài một lần ngay trong Wavely.

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
- Chọn vị trí wave phía trên, giữa hoặc phía dưới khung hình; xem trước thay đổi theo lựa chọn.
- Chọn MP4 dọc 1080 × 1920 hoặc ngang 1920 × 1080 ở 30 fps, H.264/AAC, CRF mặc định 21.
- Theo dõi tiến độ, hủy render; video chỉ được đưa vào vị trí đích sau khi xuất thành công.
- Tự căn giữa và crop ảnh để lấp khung 9:16 hoặc 16:9, không kéo méo ảnh.

## Phím tắt

Chưa có phím tắt riêng trong phiên bản đầu.

## Kiến trúc

- `src/`: giao diện TypeScript/CSS chạy trên WebView hệ thống của Tauri.
- `src-tauri/`: lệnh Rust chọn cấu hình filtergraph, gọi FFmpeg bằng tham số tách rời, đọc tiến độ và quản lý hủy.
- `VERSION`: nguồn version duy nhất; `scripts/sync-version.mjs` đồng bộ metadata npm, Cargo và Tauri trước khi build.
- `PLAN.md`: mục tiêu, tiêu chí chất lượng và trạng thái các mốc triển khai.

FFmpeg là tiến trình native riêng, không phải thư viện nhúng vào Rust. Wavely dò engine đã tải trong app data, resource đóng gói, `PATH` và một số vị trí cài phổ biến. File ảnh, audio, video tạm và video cuối đều ở trên máy.

## Cấu hình và giới hạn

- Đầu ra có hai lựa chọn MP4 1080 × 1920 hoặc 1920 × 1080; tốc độ khung hình và codec hiện giữ ở 30 fps, H.264/AAC.
- Màu sắc và filtergraph được đặt theo preset; chưa có bảng chỉnh hàng loạt thông số.
- V1 không tạo phụ đề, không nhận diện thể loại, không thêm nhạc nền, reverb, blur, grain hoặc hiệu ứng nặng.
- Halo chưa được triển khai; hai kiểu sóng dùng filter FFmpeg native.
- Chất lượng/tốc độ phụ thuộc cấu hình FFmpeg và CPU; cần benchmark Windows/macOS trước khi công bố số liệu hiệu năng.
- Installer chưa được ký/notarize. macOS có thể yêu cầu mở lần đầu bằng chuột phải → **Open**; Windows có thể hiện cảnh báo SmartScreen.

## Lịch sử phiên bản

Phiên bản source đang chuẩn bị installer để review: **0.3.0**; bản phát hành ổn định gần nhất là **0.1.0**. Chi tiết ở [CHANGELOG.md](CHANGELOG.md).

## Tải về

Installer ổn định hiện có: [Wavely v0.1.0](https://github.com/tinycorn-studio/auto_video_generate/releases/tag/v0.1.0). Bộ cài v0.3.0 đã được CI tạo làm artifact để review; GitHub Release sẽ được tạo khi phát hành theo tag sau khi kiểm tra cài đặt hoàn tất.

- Windows x64: [bản cài .exe](https://github.com/tinycorn-studio/auto_video_generate/releases/download/v0.1.0/Wavely_0.1.0_x64-setup.exe) hoặc [bản cài .msi](https://github.com/tinycorn-studio/auto_video_generate/releases/download/v0.1.0/Wavely_0.1.0_x64_en-US.msi).
- macOS Intel: [DMG](https://github.com/tinycorn-studio/auto_video_generate/releases/download/v0.1.0/Wavely_0.1.0_x64.dmg).
- macOS Apple Silicon: [DMG](https://github.com/tinycorn-studio/auto_video_generate/releases/download/v0.1.0/Wavely_0.1.0_aarch64.dmg).

Bản v0.1.0 chưa ký số/notarize và cần cài FFmpeg riêng. Bản v0.3.0 có trình tải FFmpeg trong ứng dụng, khổ video dọc/ngang và chọn vị trí wave. CI đã build ba nền tảng; kiểm tra render và smoke test macOS còn chờ.
