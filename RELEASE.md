# Quy trình Git và phát hành

Wavely dùng SemVer (`MAJOR.MINOR.PATCH`) và [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/). `VERSION` là nguồn version duy nhất; script đồng bộ giá trị này sang metadata bắt buộc của npm, Cargo và Tauri trước khi build.

## Quy ước Git

- `main` giữ trạng thái có thể build và phát hành.
- Làm tính năng trên nhánh riêng: `feat/<ten-ngan>`; sửa lỗi dùng `fix/<ten-ngan>`.
- Commit message một dòng, tiếng Anh, thể mệnh lệnh, ví dụ `Add Spectrum waveform preset`.
- Cập nhật README và CHANGELOG cùng feature. Ghi mục chưa phát hành dưới `## [Unreleased]`.
- Không commit `node_modules/`, `dist/`, `src-tauri/target/`, installer, file media đầu vào/đầu ra hay token.
- Chỉ gắn tag lên commit đã có README/CHANGELOG hoàn chỉnh và đã review.

## Tạo release

1. Hoàn tất và commit feature trên nhánh của feature; merge `--ff-only` về `main` sau review.
2. Chọn mức SemVer phù hợp: patch cho sửa tương thích, minor cho tính năng tương thích, major cho thay đổi phá vỡ tương thích.
3. Chuyển ghi chú `[Unreleased]` sang mục có ngày, ví dụ `## [0.1.1] - 2026-11-02`, bổ sung nội dung tiếng Việt và để trống `[Unreleased]` cho các thay đổi tiếp theo.
4. Chạy `npm run version:set -- 0.1.1` để cập nhật nguồn và đồng bộ metadata; kiểm tra `VERSION`, `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
5. Commit release bằng `Release v0.1.1`.
6. Tạo annotated tag và đẩy lên remote:

   ```bash
   git tag -a v0.1.1 -m "v0.1.1"
   git push origin main --follow-tags
   ```

7. GitHub Actions xác nhận tag, version và mục changelog khớp; build Windows x64, macOS Intel và Apple Silicon; tải installer `.msi`, `.exe`, `.dmg` lên GitHub Release với release notes lấy từ CHANGELOG.
8. Kiểm tra trang Release có đúng tag, nội dung changelog và ba nhóm installer. Không commit installer vào Git.

## Cài bản chưa ký

- macOS: tải đúng kiến trúc, mở DMG và kéo Wavely vào Applications. Lần đầu, control-click app rồi chọn **Open** nếu Gatekeeper hỏi.
- Windows: chạy `.exe` hoặc `.msi` tải từ GitHub Release; có thể cần xác nhận SmartScreen khi app chưa được ký số.
- Cài FFmpeg 6 trở lên riêng theo README. Release hiện không gói FFmpeg vào installer.

## Release đầu tiên

Repo nguồn là [tinycorn-studio/auto_video_generate](https://github.com/tinycorn-studio/auto_video_generate). CI build app trên Windows x64, macOS Intel và Apple Silicon khi push/PR vào `main`; release workflow build installer theo tag sau khi CI compile thành công. Bản [v0.1.0](https://github.com/tinycorn-studio/auto_video_generate/releases/tag/v0.1.0) đã được tạo và có Windows `.exe`/`.msi`, macOS Intel `.dmg`, macOS Apple Silicon `.dmg`. Ghi nhận smoke test Windows/macOS và kết quả render vào `PLAN.md` sau khi cài đặt. Máy phát triển không cần có Rust nếu build chạy qua GitHub Actions.
