# Wavely

Wavely ghép một ảnh với file ghi âm thành video MP4 có sóng âm đồng bộ. Có thể xuất video dọc hoặc ngang, chọn kiểu sóng, đặt wave ở 9 vùng trong khung và thu nhỏ wave từ 25% đến 100%. Video được xử lý trực tiếp trên Windows hoặc macOS. Tùy chọn Gemini tạo thêm tiêu đề, mô tả, hashtag và từ khóa để đăng bài.

## Yêu cầu hệ thống

- Windows 10/11 64-bit hoặc macOS 13 trở lên.
- FFmpeg 6 trở lên, có các filter `showwaves`, `showfreqs` và encoder H.264/AAC. Có thể dùng FFmpeg cài sẵn hoặc tải engine trong ứng dụng.
- Đầu vào: JPG/PNG/WebP/BMP và WAV/MP3/M4A/AAC/FLAC/OGG/Opus.
- Tạo video chạy offline sau khi cài engine. Tính năng nội dung Gemini qua XPiki cần Internet, API key có quyền dùng `gemini-3.8-flash` và quota phù hợp.

Nếu chưa có FFmpeg, bấm **Tải engine FFmpeg** trong Wavely. Ứng dụng chỉ tải sau khi bạn chủ động bấm nút, xác minh SHA-256 của gói FFmpeg 9.0.2, rồi lưu riêng file engine trong thư mục dữ liệu ứng dụng. Archive tạm được xóa sau cài đặt. Lần tải đầu cần Internet; nếu tắt Gemini, các lần xuất tiếp theo chạy offline và file ảnh/âm thanh không rời máy. FFmpeg cài sẵn trong `PATH` vẫn được hỗ trợ. Có thể cài thủ công bằng `winget install --id Gyan.FFmpeg -e` trên Windows hoặc `brew install ffmpeg` trên macOS. Nguồn build và thông tin license được ghi trong [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

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
- Chọn đường sóng Tối giản, sóng Cân xứng trên/dưới trục giữa hoặc cột phổ Spectrum.
- Đặt wave ở 9 vùng của khung hình và chỉnh kích thước từ 25% đến 100%; xem trước thay đổi đồng bộ với video xuất.
- Chọn MP4 dọc 1080 × 1920 hoặc ngang 1920 × 1080 ở 30 fps, H.264/AAC, CRF mặc định 21.
- Theo dõi tiến độ, hủy render; video chỉ được đưa vào vị trí đích sau khi xuất thành công.
- Tự căn giữa và crop ảnh để lấp khung 9:16 hoặc 16:9, không kéo méo ảnh.
- Tùy chọn tạo nội dung bằng Gemini 3.8 Flash qua XPiki sau khi render; mặc định ngôn ngữ English, hỗ trợ thêm tiếng Việt.
- Dán transcript hoặc tóm tắt nội dung âm thanh để AI viết chính xác; XPiki Gemini hiện chỉ nhận văn bản.
- Nội dung riêng cho YouTube, TikTok, Facebook và Instagram, xem theo nền tảng và sao chép từng phần hoặc tất cả.
- Lưu file `<tên-video>-noi-dung.txt` cạnh MP4; tên được thêm số nếu đã tồn tại, không ghi đè file nội dung cũ.
- Lưu/thay/xóa API key XPiki trong Keychain macOS hoặc Credential Manager Windows. Nếu API lỗi, video vẫn giữ nguyên và có thể thử lại riêng phần nội dung.

## Phím tắt

Chưa có phím tắt riêng trong phiên bản đầu.

## Kiến trúc

- `src/`: giao diện TypeScript/CSS chạy trên WebView hệ thống của Tauri.
- `src-tauri/`: lệnh Rust chọn cấu hình filtergraph, gọi FFmpeg bằng tham số tách rời, đọc tiến độ và quản lý hủy.
- `src/publishing.ts`, `src-tauri/src/publishing.rs`: giao diện nội dung, kho API key và yêu cầu Gemini qua XPiki; frontend không nhận lại API key đã lưu.
- `VERSION`: nguồn version duy nhất; `scripts/sync-version.mjs` đồng bộ metadata npm, Cargo và Tauri trước khi build.
- `PLAN.md`: mục tiêu, tiêu chí chất lượng và trạng thái các mốc triển khai.

FFmpeg là tiến trình native riêng, không phải thư viện nhúng vào Rust. Wavely dò engine đã tải trong app data, resource đóng gói, `PATH` và một số vị trí cài phổ biến. Render video không gửi media lên mạng. Khi bật Gemini, app gửi transcript hoặc tóm tắt bạn nhập tới XPiki qua Chat Completions; ảnh, âm thanh và video không được tải lên. Model `gemini-3.8-flash` trong tài khoản XPiki hiện chỉ nhận văn bản nên Wavely không tự nghe hay chép lời từ file âm thanh. API key nằm trong kho bảo mật của hệ điều hành. Chi tiết: [XPiki Chat Completions](https://docs.xpiki.com/chat-completions-42241316e0), [XPiki List Models](https://docs.xpiki.com/list-models-42241309e0).

### Thiết lập Gemini

1. Nhập API key XPiki tại **Tạo nội dung đăng bài**, bấm **Lưu**. App kiểm tra quyền truy cập model trước khi lưu vào kho bảo mật hệ điều hành.
2. Bật công tắc tạo nội dung. Chọn ngôn ngữ (mặc định **English**) và nền tảng.
3. Dán transcript hoặc tóm tắt chính xác nội dung âm thanh. Văn bản này được gửi tới XPiki; không gửi file âm thanh hay video.
4. Chọn ảnh/âm thanh rồi bấm **Tạo video & nội dung đăng bài**.
5. Xem kết quả trong tab **Nội dung đăng bài**, sao chép hoặc mở file `.txt` để chỉnh sửa trước khi đăng.

Chỉ ngôn ngữ, nền tảng và trạng thái công tắc được lưu tại localStorage. API key nằm riêng trong kho bảo mật; transcript và bài đã tạo không được lưu vào lịch sử. Không có cơ sở dữ liệu bài đăng, kiểm tra trùng lặp hay lời hứa mỗi lần sinh sẽ khác nhau. App chỉ giữ video vừa xuất trong bộ nhớ để thử lại/mở file; các file `.txt` do người dùng xuất là đầu ra được yêu cầu. Nếu đóng app rồi muốn tạo lại nội dung, cần xuất video lại.

Gemini là tính năng tùy chọn, tắt mặc định ở lần đầu. Quota và chi phí phụ thuộc cấu hình tài khoản XPiki và lượng văn bản gửi/nhận. Không cần cài AI hay database trên máy.

## Cấu hình và giới hạn

- Đầu ra có hai lựa chọn MP4 1080 × 1920 hoặc 1920 × 1080; tốc độ khung hình và codec hiện giữ ở 30 fps, H.264/AAC.
- Màu sắc và filtergraph được đặt theo preset; chưa có bảng chỉnh hàng loạt thông số.
- V1 không tạo phụ đề, không nhận diện thể loại, không thêm nhạc nền, reverb, blur, grain hoặc hiệu ứng nặng.
- Halo chưa được triển khai; hai kiểu sóng dùng filter FFmpeg native.
- Chất lượng/tốc độ phụ thuộc cấu hình FFmpeg và CPU; cần benchmark Windows/macOS trước khi công bố số liệu hiệu năng.
- Installer chưa được ký/notarize. macOS có thể yêu cầu mở lần đầu bằng chuột phải → **Open**; Windows có thể hiện cảnh báo SmartScreen.
- Yêu cầu XPiki có timeout và không tự thử lại để tránh phát sinh lượt gọi ngoài ý muốn.

## Lịch sử phiên bản

Phiên bản phát hành gần nhất: **0.3.0**. Chi tiết ở [CHANGELOG.md](CHANGELOG.md).

## Tải về

Bản phát hành mới nhất: [Wavely v0.3.0](https://github.com/tinycorn-studio/auto_video_generate/releases/tag/v0.3.0). Bản này có tải FFmpeg trong ứng dụng và xuất video dọc/ngang. Sóng Cân xứng, lưới 9 vị trí và chỉnh kích thước đang được chuẩn bị cho bản phát hành tiếp theo.

- Windows x64: [bản cài .exe](https://github.com/tinycorn-studio/auto_video_generate/releases/download/v0.3.0/Wavely_0.3.0_x64-setup.exe) hoặc [bản cài .msi](https://github.com/tinycorn-studio/auto_video_generate/releases/download/v0.3.0/Wavely_0.3.0_x64_en-US.msi).
- macOS Intel: [DMG](https://github.com/tinycorn-studio/auto_video_generate/releases/download/v0.3.0/Wavely_0.3.0_x64.dmg).
- macOS Apple Silicon: [DMG](https://github.com/tinycorn-studio/auto_video_generate/releases/download/v0.3.0/Wavely_0.3.0_aarch64.dmg).

Bộ cài v0.3.0 chưa ký số/notarize; Windows có thể hiện SmartScreen và macOS có thể yêu cầu xác nhận mở lần đầu. CI đã build trên macOS Intel và Apple Silicon; smoke test chạy trực tiếp trên macOS vẫn chưa thực hiện.
