# Kế hoạch AutoVideoGenerator

**Cập nhật lần cuối:** 2026-10-23
**Trạng thái hiện tại:** Mã nguồn v0.1.0 và CI compile đã đạt trên Windows x64, macOS Intel và Apple Silicon; đóng gói installer, kiểm tra FFmpeg thực tế và benchmark còn chờ
**Mục tiêu:** Tạo ứng dụng desktop chạy trên Windows và macOS, ghép một file ghi âm với một ảnh dọc thành video có sóng âm đồng bộ.

## 1. Yêu cầu sản phẩm

### Đầu vào

- Một file âm thanh do người dùng chọn.
- Một ảnh dọc, ưu tiên ảnh Full HD 1080 × 1920.
- Đọc thời lượng và thông tin file tự động; báo lỗi dễ hiểu khi file không đọc được.

### Đầu ra mặc định

- MP4 dọc 1080 × 1920, 30 fps, H.264 và âm thanh AAC.
- Độ dài video khớp với bản ghi âm.
- Giữ nguyên nội dung âm thanh; không tự giảm tốc, thêm reverb, nhạc nền hoặc phụ đề.
- Ảnh không méo. Nếu ảnh khác tỷ lệ 9:16, tự căn giữa và cắt vừa khung; không phóng đại hiệu ứng.
- Tệp đầu ra tương thích với trình phát phổ biến và sẵn sàng đăng lên nền tảng video.

### Trải nghiệm sử dụng

- Quy trình gọn: chọn ảnh, chọn âm thanh, chọn một preset, xem trước, xuất video.
- Xử lý trên máy người dùng; v1 không gửi media lên dịch vụ bên ngoài.
- Không cần API AI. Nhận diện thể loại tự động, tạo phụ đề và xử lý giọng nói nằm ngoài phạm vi v1.
- Chỉ cung cấp một số preset chọn nhanh, không mở một bảng thông số kỹ thuật dài.

## 2. Đề xuất giao diện và preset

| Preset | Hình ảnh sóng | Dùng khi | Ưu tiên hiệu năng |
|---|---|---|---|
| **Tối giản** | Đường sóng mảnh | Giọng nói, podcast, bản thu nhẹ | Rất cao; đề xuất làm mặc định ban đầu |
| **Spectrum** | Cột phổ đối xứng | Nhạc, beat, EDM | Cao; chỉ vẽ một dải cột gọn |
| **Halo** | Vòng âm thanh | Acoustic, lo-fi | Thử nghiệm; đưa vào sau khi đo hiệu năng |

Tên preset là gợi ý giao diện, không phải nhận diện thể loại bằng AI. Trong v1, người dùng tự chọn preset; màu sắc đi theo preset và có thể có một lựa chọn đổi màu đơn giản. Chốt preset mặc định sau khi xác nhận mẫu trong mockup.

## 3. Hướng kỹ thuật

### Ứng dụng desktop

- **Đề xuất:** Tauri 2, giao diện HTML/CSS/TypeScript gọn và một backend Rust nhỏ để chọn file, quản lý tiến trình, đọc tiến độ, hủy tác vụ và gọi FFmpeg an toàn.
- Tauri dùng WebView có sẵn trên hệ điều hành nên phần vỏ ứng dụng nhỏ. Windows dùng WebView2; macOS dùng WKWebView. Vì vậy cần kiểm tra giao diện trên cả hai nền tảng, không chỉ dựa vào Windows preview.
- Dự án cần công cụ build theo nền tảng: Tauri yêu cầu Xcode Command Line Tools trên macOS và C++ Build Tools cùng WebView2 trên Windows. Mỗi gói phát hành sẽ được build và kiểm tra trên hệ tương ứng.
- Giao diện dùng component và CSS chuẩn, không thêm framework UI lớn nếu chưa cần.

### Bộ dựng video

- Dùng FFmpeg làm bộ máy media chung trên Windows/macOS. V1 đọc duration từ thông tin đầu vào FFmpeg để không cần thêm binary FFprobe.
- Dùng filtergraph của FFmpeg để giữ ảnh nền, tạo dải sóng trong suốt, ghép âm thanh và mã hóa MP4.
- **Tối giản:** dùng showwaves cho sóng thể hiện biên độ theo thời gian.
- **Spectrum:** dùng showfreqs ở chế độ bar để hiển thị các dải tần. Giới hạn kích thước vùng sóng và số cột; không dựng phân tích phổ cho toàn khung hình nếu chỉ cần một overlay nhỏ.
- **Halo:** FFmpeg không cung cấp cùng kiểu vòng tròn radial trực tiếp như hai kiểu trên. Chỉ thêm khi có thể dựng overlay nhỏ, đo được và không làm chậm đáng kể; nếu không, để ở phiên bản sau.
- Dùng -progress pipe:1 hoặc cơ chế tương đương để hiển thị tiến độ. Gọi tiến trình bằng danh sách tham số, không nối lệnh từ chuỗi đầu vào của người dùng.
- Render vào file tạm, chỉ đổi tên thành file đích sau khi FFmpeg hoàn tất thành công. Không ghi đè file có sẵn nếu người dùng chưa chọn.

### Chất lượng và hiệu năng

- Mặc định tắt film grain, blur, shake và glow nặng. Các hiệu ứng này làm tăng chi phí render; chúng không cần thiết cho yêu cầu ảnh + audio + sóng âm.
- Mặc định dùng 1080 × 1920, 30 fps, H.264, CRF khoảng 20–22 và preset mã hóa nhanh. Đo tốc độ/file size trước khi khóa chính xác thông số.
- Nếu file âm thanh đầu vào đã tương thích với MP4, cân nhắc copy stream để tránh mã hóa lại; nếu không, chuyển sang AAC chất lượng cao.
- Giữ phần sóng ở vùng nhỏ; phần tốn công chính vẫn là dựng và mã hóa khung hình 1080 × 1920.
- Tạo preview ngắn, độ phân giải thấp hơn trước khi xuất toàn bộ video để người dùng kiểm tra bố cục với thời gian chờ ngắn.
- Đo thời gian render, kích thước file và mức dùng RAM trên máy tham chiếu. Mục tiêu ban đầu là đạt ít nhất tốc độ thời gian thực ở chế độ mặc định; ghi rõ cấu hình máy trong kết quả để so sánh công bằng.

### FFmpeg và cách phân phối

- V1 dò FFmpeg trong PATH và một số vị trí cài phổ biến; kiểm tra filter `showwaves`, `showfreqs`, encoder H.264/AAC và báo cách cài nếu thiếu.
- V1 yêu cầu cài FFmpeg riêng để giữ installer Wavely nhỏ và tránh tự phân phối một build FFmpeg có codec/license chưa được chốt. Không tải binary ngầm khi người dùng mở ứng dụng.
- Có thể đánh giá lại bundle FFmpeg cho các release sau; cần ghi nguồn binary, phiên bản, license/source tương ứng, dung lượng cài đặt và kiểm tra đường chạy trên cả hai hệ điều hành trước khi thay đổi.

### Git, version và release

- Dùng SemVer. `VERSION` là nguồn version duy nhất; `scripts/sync-version.mjs` đồng bộ sang metadata bắt buộc của npm, Cargo và Tauri. `package-lock.json` được cập nhật khi đổi version.
- Dùng Keep a Changelog; README và CHANGELOG cập nhật cùng feature. Release notes tiếng Việt được tạo từ mục tương ứng trong CHANGELOG.
- Làm feature trên nhánh riêng, commit message một dòng tiếng Anh dạng mệnh lệnh; không commit installer/build output/media.
- GitHub Actions chạy khi đẩy tag `vX.Y.Z`, xác nhận tag/version/changelog, build installer Windows x64 và DMG macOS Intel/Apple Silicon, rồi đính kèm chúng vào GitHub Release.
- Repo GitHub `tinycorn-studio/auto_video_generate` đã kết nối với remote `origin`; CI build app trên Windows x64, macOS Intel và Apple Silicon khi push/PR vào `main`; release workflow tạo installer khi đẩy tag SemVer.

### Dung lượng cache Rust/Cargo

- Có thể tăng đáng kể trên máy phát triển. Cargo lưu sản phẩm build và incremental artifacts trong thư mục target của repo; nhiều profile và kiến trúc tạo thêm đầu ra. Cargo lưu mã nguồn/thư mục phụ thuộc tải về trong CARGO_HOME, còn rustup lưu toolchain trong RUSTUP_HOME.
- cargo clean chỉ xóa artifacts của target; không xóa crate cache trong CARGO_HOME. Build tiếp theo sẽ phải biên dịch lại nên chỉ clean khi cần giải phóng dung lượng hoặc dọn build profile cũ.
- Nếu ổ hệ thống nhỏ, chuyển CARGO_TARGET_DIR sang ổ khác; khi cài Rust có thể đặt CARGO_HOME và RUSTUP_HOME sang vị trí khác. Không ghi đường dẫn riêng của máy phát triển vào cấu hình chia sẻ của repo.
- Bộ cài ứng dụng chỉ lấy binary release và runtime assets cần thiết. Không đưa target/, Cargo registry, Rust toolchain hoặc thư mục cache vào bộ cài. Người dùng cuối không cần cài Rust/Cargo để chạy app đã build.
- FFmpeg binary, video đầu ra và file preview/tạm là các thành phần dung lượng riêng, cần đo và dọn theo chính sách của chúng; chúng không phải Rust cache.

### Desktop local so với web trên Vercel

- **Đề xuất cho v1: desktop Tauri + FFmpeg chạy trên máy người dùng.** Phù hợp hơn với file ghi âm dài và video 1080 × 1920 vì xử lý bằng FFmpeg native, không phải tải media lên mạng; không phụ thuộc giới hạn thời gian của serverless function và giữ file nguồn trên máy.
- **Web deploy trên Vercel tiện chia sẻ và cập nhật giao diện.** Vercel có thể host giao diện tĩnh, nhưng bản thân việc deploy không tạo ra bộ máy render video.
- Nếu render ngay trong trình duyệt bằng ffmpeg.wasm, không cần backend xử lý media nhưng tốc độ thường thấp hơn FFmpeg native và có thể dùng nhiều RAM/CPU; cần đo trên máy và trình duyệt mục tiêu.
- Nếu render phía server Vercel, phải tải file lên và xử lý trong function. Functions có giới hạn dung lượng request, thời gian chạy và tài nguyên; upload media lớn cần lưu trữ ngoài hoặc tải trực tiếp vào object storage, còn tác vụ dài có thể cần hàng đợi/worker riêng. Cách này tăng thành phần vận hành và chi phí tiềm năng.
- **Khuyến nghị:** làm bộ tạo video desktop local trước. Có thể deploy một trang demo/giới thiệu trên Vercel sau này; chỉ chuyển phần render lên web nếu nhu cầu chia sẻ từ xa quan trọng hơn thời gian render, quyền riêng tư và sự đơn giản của bản desktop.
- **Trạng thái quyết định:** đây là khuyến nghị kiến trúc cho v1; chưa có ứng dụng hay website nào được triển khai. M1 vẫn cần kiểm tra build Tauri trên Windows/macOS và thử gọi FFmpeg.

## 4. Kế hoạch theo giai đoạn

Các mốc được cập nhật tại đây cùng với thay đổi mã nguồn. Mỗi lần cập nhật ghi trạng thái, ngày, việc hoàn thành, bằng chứng kiểm tra và bước tiếp theo. Không cần ghi từng thao tác nhỏ; cập nhật khi đổi trạng thái hoặc hoàn tất một mốc.

| Mốc | Trạng thái | Kết quả cần có |
|---|---|---|
| **M0 · Repo và plan** | **Hoàn thành** | Git repository, remote GitHub, .gitignore và tài liệu tiến độ dùng chung |
| **M1 · Chốt yêu cầu / nền tảng** | Đang triển khai · quyết định đã ghi, native build còn chờ | Tauri 2; preset Tối giản mặc định, Spectrum tùy chọn; FFmpeg cài riêng ở v1; cần build thử trên Windows/macOS |
| **M2 · Prototype FFmpeg** | Đang triển khai · filtergraph đã có, chưa chạy | Ảnh + audio → MP4; đo line và Spectrum, giữ audio đúng thời lượng |
| **M3 · Vỏ ứng dụng** | Đang triển khai · source đã có, Rust chưa compile | Màn hình chọn file, preview, lỗi FFmpeg, xuất, tiến độ và hủy tác vụ |
| **M4 · Tối ưu và chất lượng** | Chưa bắt đầu | Đo thời gian/RAM/file size, kiểm tra đồng bộ âm thanh, màu sắc, crop và phát lại |
| **M5 · Windows/macOS** | Đang triển khai · CI compile đạt trên ba mục tiêu, installer chưa đóng gói | Tạo installer từ tag, sau đó ghi nhận checklist smoke test Windows/macOS Intel/Apple Silicon |
| **M6 · Bản dùng thử** | Đang triển khai · source đã lên GitHub, release workflow đang chờ tag | Tạo release đầu có installer, kiểm tra asset/changelog, tiếp tục smoke test và ghi nhận giới hạn |

## 5. Tiêu chí nghiệm thu

### Chức năng

- Chọn được một ảnh và một file âm thanh; hiển thị tên, thời lượng và trạng thái đầu vào.
- Xuất được MP4 theo preset đã chọn và mở được trong trình phát phổ biến.
- Thời lượng khớp với âm thanh; không tăng/giảm tốc hoặc làm mất tiếng ở đầu/cuối.
- Tiến độ cập nhật; có thể hủy mà không để lại file đầu ra hỏng như kết quả hoàn chỉnh.
- Lỗi FFmpeg, file lỗi, thiếu dung lượng và đường dẫn có dấu/ký tự đặc biệt được xử lý rõ ràng.

### Chất lượng hình/tiếng

- Kích thước mặc định đúng 1080 × 1920, tỷ lệ 9:16, 30 fps, H.264/AAC.
- Ảnh không bị kéo méo; chữ và sóng không bị cắt khỏi vùng an toàn.
- Sóng có nhịp phản ứng ổn định theo audio, không nhấp nháy bất thường.
- Âm thanh đầu ra nghe được từ đầu đến cuối và giữ nguyên nội dung bản thu.
- Preview và video xuất ra dùng cùng bố cục/preset.

### Hiệu năng và hệ điều hành

- Benchmark tối thiểu một clip 1 phút và một clip 5 phút ở 1080 × 1920/30 fps.
- Ghi lại thời gian render, tỷ lệ thời gian render/thời lượng clip, kích thước video và mức RAM.
- Chạy được trên Windows và macOS; xác nhận riêng bộ giải mã, WebView, đường dẫn file, tên file Unicode và codec đầu ra.
- Không yêu cầu GPU rời; đường đi mặc định dựa trên CPU và FFmpeg.

## 6. Rủi ro và cách xử lý

| Rủi ro | Cách xử lý |
|---|---|
| Tốc độ mã hóa khác nhau giữa máy | Benchmark trên phần cứng được ghi rõ; cho phép hạ FPS hoặc dùng chế độ draft nếu cần |
| FFmpeg chưa cài hoặc build thiếu encoder | Kiểm tra khi khởi động, báo lệnh cần thiết và phiên bản/encoder yêu cầu |
| Khác biệt WebView trên Windows/macOS | Dùng CSS/HTML phổ thông, kiểm tra cả hai nền tảng ở mốc M5 |
| Cargo cache chiếm dung lượng máy phát triển | Đưa target vào .gitignore; dùng CARGO_TARGET_DIR nếu cần ổ riêng; chỉ clean khi cần và không đóng gói Cargo cache |
| Đóng gói FFmpeg làm tăng dung lượng hoặc có nghĩa vụ license | Chốt build và mô hình phân phối trước bản release; lưu thông tin license của binary |
| Dạng vòng tròn khó dựng nhẹ bằng filtergraph có sẵn | Để Halo sau hai kiểu native FFmpeg nếu prototype không đạt mục tiêu hiệu năng |
| Ảnh ngang hoặc ảnh có orientation metadata | Đọc orientation trước khi scale/crop và preview rõ vùng ảnh bị cắt |

## 7. Theo dõi tiến độ

Khi có thay đổi, cập nhật cả bảng mốc ở trên và nhật ký sau đây trong cùng một lượt làm việc với mã nguồn:

- Trạng thái hiện tại và ngày cập nhật.
- Việc đã hoàn thành, kèm lệnh/chụp kết quả/đường dẫn kiểm tra phù hợp.
- Hạng mục còn vướng hoặc cần quyết định; không đánh dấu hoàn thành nếu chưa có bằng chứng.
- Bước tiếp theo cụ thể.

### Nhật ký

#### 2026-10-23 · M0 hoàn thành

- Khởi tạo Git repository trên nhánh main; repository chưa có commit.
- Tạo PLAN.md để ghi yêu cầu, thiết kế kỹ thuật, tiêu chí nghiệm thu và tiến độ các mốc.
- Tạo .gitignore cho build output, media đã render và file hệ điều hành.
- Đã chọn hướng đề xuất: Tauri 2 cho giao diện desktop nhẹ; Rust điều phối FFmpeg/FFprobe; filtergraph FFmpeg dựng video.
- Làm rõ lưu trữ: Cargo build cache nằm ở máy phát triển; bản app đã build không mang theo Cargo cache/toolchain. Ghi cách chuyển hoặc dọn cache vào plan.
- Chưa bắt đầu code ứng dụng hoặc benchmark trên máy thật.
- **Tiếp theo:** M1 — xác nhận preset mặc định và cách cài/đóng gói FFmpeg, sau đó làm prototype media trước giao diện đầy đủ.

#### 2026-10-23 · Đánh giá phương án triển khai

- So sánh desktop local với web deploy trên Vercel cho tác vụ render ảnh + audio thành MP4 dọc.
- Đề xuất desktop Tauri + FFmpeg native cho v1; Vercel phù hợp để host giao diện demo/giới thiệu hơn là chạy render dài.
- Ghi nhận các lựa chọn web: ffmpeg.wasm xử lý trong trình duyệt (đổi lại tốc độ/RAM) hoặc serverless (giới hạn request/thời gian và cần storage/worker khi file lớn).
- Chưa tạo ứng dụng desktop, chưa tạo website và chưa deploy.
- **Tiếp theo:** M1 — dựng thử vỏ Tauri trên Windows/macOS và xác minh quy trình chạy FFmpeg local.

#### 2026-10-23 · V0.1.0 source và release flow

- Dựng giao diện Wavely theo mẫu Tối giản đã được chọn; có chọn ảnh/audio, nghe thử, preview dọc, preset Tối giản/Spectrum và chỉ báo trạng thái FFmpeg.
- Viết lệnh Rust/Tauri gọi FFmpeg bằng đối số riêng, tạo output tạm, gửi tiến độ, hủy tiến trình và chỉ thay file đích sau khi render thành công.
- Chốt FFmpeg cài riêng cho v1; installer Wavely không tải hoặc phân phối binary FFmpeg.
- Thêm `VERSION`, script đồng bộ metadata, `CHANGELOG.md`, `RELEASE.md`, README, LICENSE và workflow tạo installer Windows/macOS theo tag SemVer.
- Đã chạy `npm install`, tạo `package-lock.json`; `npm run build` thành công. `npm run icons` sinh icon desktop từ SVG nguồn.
- Máy hiện tại không có Rust/Cargo hoặc FFmpeg; chưa compile Tauri, chưa render video mẫu, chưa benchmark và chưa kiểm tra native build trên macOS.
- Repo GitHub `tinycorn-studio/auto_video_generate` public và đang rỗng; CI build app trên Windows/macOS đã được thêm, release workflow tạo installer theo tag SemVer.
- Chưa có native build pass, render video thật, benchmark hay smoke test trên máy macOS. Chưa tạo release.
- **Tiếp theo:** đẩy source lên repo, theo dõi CI trên ba mục tiêu, sửa lỗi phát hiện được rồi mới tạo bản cài đặt thử.

#### 2026-10-23 · CI build đa nền tảng

- Đã push commit `fb5405b` lên `main` của [tinycorn-studio/auto_video_generate](https://github.com/tinycorn-studio/auto_video_generate).
- [GitHub Actions run #1](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/39890146126) thành công: Tauri compile đạt trên Windows x64, macOS Intel và macOS Apple Silicon.
- Đây mới là xác nhận compile app; chưa đóng gói installer, chưa chạy app GUI/FFmpeg, chưa xuất video mẫu và chưa benchmark.
- **Tiếp theo:** tạo tag `v0.1.0` để chạy release workflow, kiểm tra installers và release notes, sau đó thử thực tế FFmpeg trên Windows/macOS.

## 8. Tài liệu tham khảo

- [FFmpeg showfreqs](https://ffmpeg.org/ffmpeg-filters.html#showfreqs) — chế độ bar/line/dot và các cài đặt phổ tần.
- [FFmpeg showwaves](https://ffmpeg.org/ffmpeg-filters.html#showwaves) — chế độ hiển thị mẫu sóng âm.
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) — yêu cầu build trên Windows và macOS.
- [Tauri process model](https://v2.tauri.app/concept/process-model/) — WebView hệ thống và ảnh hưởng đến kích thước ứng dụng.
- [Cargo build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html) — vị trí và nội dung target/build artifacts.
- [cargo clean](https://doc.rust-lang.org/cargo/commands/cargo-clean.html) — xóa artifacts trong target.
- [Cargo home](https://doc.rust-lang.org/cargo/guide/cargo-home.html) và [rustup variables](https://rust-lang.github.io/rustup/environment-variables.html) — chuyển vị trí dependency cache và Rust toolchain.
- [ialakey/music-video-generator](https://github.com/ialakey/music-video-generator) — tham khảo preset và giới hạn hiệu năng của pipeline Python/Pillow + FFmpeg.
- [TinyCorn Studio · studio-conventions](https://github.com/tinycorn-studio/agent-skills/blob/main/skills/studio-conventions/SKILL.md) — cấu trúc repo, ngôn ngữ, commit, SemVer và yêu cầu cập nhật tài liệu.
- [TinyCorn Studio · release-flow](https://github.com/tinycorn-studio/agent-skills/blob/main/skills/release-flow/SKILL.md) — tag annotated, release notes tiếng Việt, installer trong GitHub Release và không commit file cài.
- [TinyCorn Studio · ui-preview-first](https://github.com/tinycorn-studio/agent-skills/blob/main/skills/ui-preview-first/SKILL.md) — UI dùng mẫu đã được bạn duyệt trước khi đưa vào app.
- [Tauri Windows Installer](https://v2.tauri.app/distribute/windows-installer/) — định dạng `.msi`/`.exe` và cấu hình installer.
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) — dependency build cho Windows/macOS.
- [Vercel Functions: giới hạn kích thước request](https://vercel.com/kb/guide/how-to-bypass-vercel-body-size-limit-serverless-functions) — hướng upload trực tiếp khi payload media vượt giới hạn function.
- [Vercel Functions: cấu hình thời gian tối đa](https://vercel.com/docs/functions/configuring-functions/duration) — thời gian chạy phụ thuộc cấu hình và loại compute; cần kiểm tra giới hạn hiện hành trước khi thiết kế job render.
- [Vercel Hobby plan](https://vercel.com/docs/plans/hobby) — giới hạn sử dụng của gói miễn phí.
- [ffmpeg.wasm FAQ](https://ffmpegwasm.netlify.app/docs/faq/) và [performance](https://ffmpegwasm.netlify.app/docs/performance/) — so sánh đặc điểm hiệu năng của FFmpeg WebAssembly và native FFmpeg.
