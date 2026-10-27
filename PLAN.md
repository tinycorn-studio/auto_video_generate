# Kế hoạch AutoVideoGenerator

**Cập nhật lần cuối:** 2026-10-27
**Trạng thái hiện tại:** Wavely v0.5.0 đang chờ merge PR #3; CI build và publishing tests đã qua trên ba mục tiêu, người dùng đã chạy thử app trên macOS; release công khai gần nhất vẫn là v0.3.0 cho đến khi publish tag v0.5.0
**Mục tiêu:** Tạo ứng dụng desktop chạy trên Windows và macOS, ghép một file ghi âm với một ảnh thành video có sóng âm đồng bộ.

## 1. Yêu cầu sản phẩm

### Đầu vào

- Một file âm thanh do người dùng chọn.
- Một ảnh; ứng dụng crop ảnh theo khổ video đã chọn mà không kéo méo.
- Đọc thời lượng và thông tin file tự động; báo lỗi dễ hiểu khi file không đọc được.

### Đầu ra mặc định

- Mặc định MP4 dọc 1080 × 1920, 30 fps, H.264 và âm thanh AAC; có lựa chọn ngang 1920 × 1080.
- Wave có thể đặt ở 9 vùng của khung hình và điều chỉnh kích thước từ 25% đến 100%.
- Độ dài video khớp với bản ghi âm.
- Giữ nguyên nội dung âm thanh; không tự giảm tốc, thêm reverb, nhạc nền hoặc phụ đề.
- Ảnh không méo. Nếu ảnh khác tỷ lệ khung hình đã chọn, tự căn giữa và cắt vừa khung; không phóng đại hiệu ứng.
- Tệp đầu ra tương thích với trình phát phổ biến và sẵn sàng đăng lên nền tảng video.

### Trải nghiệm sử dụng

- Quy trình gọn: chọn ảnh, âm thanh, preset, khổ và vị trí wave; xem trước rồi xuất video.
- Xử lý trên máy người dùng; v1 không gửi media lên dịch vụ bên ngoài.
- Không cần API AI. Nhận diện thể loại tự động, tạo phụ đề và xử lý giọng nói nằm ngoài phạm vi v1.
- Chỉ cung cấp một số preset chọn nhanh, không mở một bảng thông số kỹ thuật dài.

## 2. Đề xuất giao diện và preset

| Preset | Hình ảnh sóng | Dùng khi | Ưu tiên hiệu năng |
|---|---|---|---|
| **Tối giản** | Đường sóng mảnh | Giọng nói, podcast, bản thu nhẹ | Rất cao; đề xuất làm mặc định ban đầu |
| **Cân xứng** | Các vạch biên độ vươn đối xứng trên/dưới trục giữa | Podcast, lo-fi, video cần dải sóng cân đối | Rất cao; dùng `showwaves` native |
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
- **Cân xứng:** dùng showwaves `mode=cline` để vẽ từng mẫu đối xứng quanh tâm vùng wave; giữ cùng filter native, FPS và kích thước vùng để không thêm bước phân tích AI hay tiến trình phụ.
- **Spectrum:** dùng showfreqs ở chế độ bar để hiển thị các dải tần. Giới hạn kích thước vùng sóng và số cột; không dựng phân tích phổ cho toàn khung hình nếu chỉ cần một overlay nhỏ.
- **Halo:** FFmpeg không cung cấp cùng kiểu vòng tròn radial trực tiếp như hai kiểu trên. Chỉ thêm khi có thể dựng overlay nhỏ, đo được và không làm chậm đáng kể; nếu không, để ở phiên bản sau.
- Dùng -progress pipe:1 hoặc cơ chế tương đương để hiển thị tiến độ. Gọi tiến trình bằng danh sách tham số, không nối lệnh từ chuỗi đầu vào của người dùng.
- Render vào file tạm, chỉ đổi tên thành file đích sau khi FFmpeg hoàn tất thành công. Không ghi đè file có sẵn nếu người dùng chưa chọn.

### Chất lượng và hiệu năng

- Mặc định tắt film grain, blur, shake và glow nặng. Các hiệu ứng này làm tăng chi phí render; chúng không cần thiết cho yêu cầu ảnh + audio + sóng âm.
- Mặc định dùng 1080 × 1920, 30 fps, H.264, CRF khoảng 20–22 và preset mã hóa nhanh. Đo tốc độ/file size trước khi khóa chính xác thông số.
- Nếu file âm thanh đầu vào đã tương thích với MP4, cân nhắc copy stream để tránh mã hóa lại; nếu không, chuyển sang AAC chất lượng cao.
- Giữ phần sóng ở vùng nhỏ; phần tốn công chính vẫn là dựng và mã hóa khung Full HD 1080 × 1920 hoặc 1920 × 1080.
- Tạo preview ngắn, độ phân giải thấp hơn trước khi xuất toàn bộ video để người dùng kiểm tra bố cục với thời gian chờ ngắn.
- Đo thời gian render, kích thước file và mức dùng RAM trên máy tham chiếu. Mục tiêu ban đầu là đạt ít nhất tốc độ thời gian thực ở chế độ mặc định; ghi rõ cấu hình máy trong kết quả để so sánh công bằng.

### FFmpeg và cách phân phối

- Wavely ưu tiên FFmpeg đã cài trong app data, rồi kiểm tra resource đóng gói, PATH và các vị trí phổ biến; xác thực filter `showwaves`, `showfreqs`, encoder H.264/AAC và phiên bản 6+.
- Nếu chưa có engine, người dùng bấm nút cài trong app. Tải đúng archive cho Windows x64, macOS Intel hoặc Apple Silicon, xác minh SHA-256 ghim trong source, chỉ giữ executable cần thiết trong app data và xóa archive tạm. Không tải khi mở app hoặc khi chưa có thao tác người dùng.
- Gói FFmpeg 9.0.2 được lấy từ Gyan (Windows) và Martin Riedl (macOS); nguồn, hash và license được ghi trong `THIRD_PARTY_NOTICES.md`. Build có `libx264`, nên trước mỗi lần đổi build phải xem lại license/source của chính binary đó.
- FFmpeg tải lần đầu cần Internet; sau cài, render local/offline. Dung lượng engine là dữ liệu riêng, không nằm trong installer Wavely hoặc Cargo cache.

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
| **M1 · Chốt yêu cầu / nền tảng** | Đang triển khai · đã chốt cài engine theo yêu cầu, native build còn chờ | Tauri 2; preset Tối giản mặc định, Spectrum tùy chọn; engine tải một lần, kiểm tra hash, cần build thử trên Windows/macOS |
| **M2 · Prototype FFmpeg** | Đang triển khai · filtergraph đã có, chưa chạy | Ảnh + audio → MP4; đo line và Spectrum, giữ audio đúng thời lượng |
| **M3 · Vỏ ứng dụng** | Đang triển khai · đã thêm nút tải/cài FFmpeg và tiến độ, Rust chưa compile | Màn hình chọn file, preview, cài engine, xuất, tiến độ và hủy tác vụ |
| **M4 · Tối ưu và chất lượng** | Chưa bắt đầu | Đo thời gian/RAM/file size, kiểm tra đồng bộ âm thanh, màu sắc, crop và phát lại |
| **M5 · Windows/macOS** | Đang triển khai · CI build đạt trên ba mục tiêu; người dùng xác nhận render thành công; smoke test macOS còn chờ | Smoke test macOS Intel/Apple Silicon |
| **M6 · Bản dùng thử** | **Hoàn thành · v0.1.0 và v0.3.0 đã phát hành** | Installer và changelog đã đính kèm; ghi nhận phản hồi sau khi chạy thực tế |
| **M7 · Khổ và bố cục xuất** | Đã phát hành trong v0.3.0; người dùng xác nhận render thành công; chưa kiểm tra toàn bộ tổ hợp khổ/vị trí/preset | Chọn khổ dọc/ngang; preview và FFmpeg đồng bộ bố cục |
| **M8 · Sóng cân xứng và điều chỉnh bố cục** | **Hoàn thành · v0.4.0 đã merge; người dùng xác nhận xuất video thành công trên Windows; CI xanh cả ba mục tiêu** | Kiểu sóng cân xứng `cline`, lưới 3 × 3 vị trí và scale 25–100% |
| **M9 · Nội dung đăng bài từ audio** | **Hoàn thành về tính năng · PR #3 chờ merge; CI xanh trên ba mục tiêu, app đã được người dùng chạy thử trên macOS** | Google Gemini/XPiki, chế độ Audio mặc định hoặc Transcript, nội dung tiếng Anh mặc định và file `.txt` |

## 5. Tiêu chí nghiệm thu

### Chức năng

- Chọn được một ảnh và một file âm thanh; hiển thị tên, thời lượng và trạng thái đầu vào.
- Xuất được MP4 theo preset đã chọn và mở được trong trình phát phổ biến.
- Chọn được MP4 dọc 1080 × 1920 hoặc ngang 1920 × 1080; chọn được một trong 9 vị trí và scale wave 25–100%.
- Kiểu sóng Cân xứng thể hiện biên độ đều lên trên và xuống dưới đường giữa.
- Thời lượng khớp với âm thanh; không tăng/giảm tốc hoặc làm mất tiếng ở đầu/cuối.
- Tiến độ cập nhật; có thể hủy mà không để lại file đầu ra hỏng như kết quả hoàn chỉnh.
- Lỗi FFmpeg, file lỗi, thiếu dung lượng và đường dẫn có dấu/ký tự đặc biệt được xử lý rõ ràng.

### Chất lượng hình/tiếng

- Kích thước mặc định đúng 1080 × 1920; lựa chọn ngang đúng 1920 × 1080; cả hai ở 30 fps, H.264/AAC.
- Ảnh không bị kéo méo; wave ở đúng vị trí đã chọn và không bị cắt khỏi khung hình.
- Sóng có nhịp phản ứng ổn định theo audio, không nhấp nháy bất thường.
- Âm thanh đầu ra nghe được từ đầu đến cuối và giữ nguyên nội dung bản thu.
- Preview và video xuất ra dùng cùng khổ, vị trí, scale wave và preset.

### Hiệu năng và hệ điều hành

- Benchmark tối thiểu một clip 1 phút và một clip 5 phút ở cả hai khổ Full HD/30 fps.
- Ghi lại thời gian render, tỷ lệ thời gian render/thời lượng clip, kích thước video và mức RAM.
- Chạy được trên Windows và macOS; xác nhận riêng bộ giải mã, WebView, đường dẫn file, tên file Unicode và codec đầu ra.
- Không yêu cầu GPU rời; đường đi mặc định dựa trên CPU và FFmpeg.

## 6. Rủi ro và cách xử lý

| Rủi ro | Cách xử lý |
|---|---|
| Tốc độ mã hóa khác nhau giữa máy | Benchmark trên phần cứng được ghi rõ; cho phép hạ FPS hoặc dùng chế độ draft nếu cần |
| FFmpeg chưa cài hoặc build thiếu encoder | Kiểm tra khi khởi động, cho phép cài engine một lần, xác minh filter/encoder và cung cấp hướng dẫn thủ công khi tải lỗi |
| Khác biệt WebView trên Windows/macOS | Dùng CSS/HTML phổ thông, kiểm tra cả hai nền tảng ở mốc M5 |
| Cargo cache chiếm dung lượng máy phát triển | Đưa target vào .gitignore; dùng CARGO_TARGET_DIR nếu cần ổ riêng; chỉ clean khi cần và không đóng gói Cargo cache |
| Tải engine FFmpeg có nghĩa vụ license/build provenance | Ghim build, ghi source/hash/license; rà soát đúng binary và source tương ứng trước release |
| Nguồn tải engine thay đổi hoặc gói bị lỗi | Ghim URL/hash theo nền tảng, giới hạn kích thước, xác minh binary trước khi cài, xóa archive tạm và cho phép hủy |
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

#### 2026-10-23 · Release v0.1.0

- Đã tạo annotated tag `v0.1.0`; [release workflow](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/39890956368) thành công trên Windows, macOS Intel và Apple Silicon.
- [GitHub Release v0.1.0](https://github.com/tinycorn-studio/auto_video_generate/releases/tag/v0.1.0) có đủ bốn installer: Windows `.exe`/`.msi`, macOS Intel `.dmg`, macOS Apple Silicon `.dmg`; dung lượng từng file khoảng 2–3 MB.
- Release notes được tạo tự động từ mục `[0.1.0]` trong `CHANGELOG.md`; installer chưa ký số/notarize và cần cài FFmpeg 6+ riêng.
- Chưa chạy GUI hoặc render media trên máy thật; cần smoke test Windows/macOS và benchmark theo tiêu chí M4.
- **Tiếp theo:** cài đúng gói theo hệ điều hành, cài FFmpeg, tạo một video mẫu rồi ghi thời gian render/chất lượng/khả năng phát lại.

#### 2026-10-23 · Wavely v0.2.0 build mới

- Chuyển mục FFmpeg cài trong ứng dụng từ `[Unreleased]` sang changelog `0.2.0`; đồng bộ version npm/Cargo/Tauri.
- CI pull request chuyển từ compile-only sang đóng gói Windows `.exe`/`.msi` và macOS `.dmg`, tải làm artifacts để review và cài thử.
- Máy Windows hiện không có MSVC Build Tools; dùng GitHub Actions để build native mà không thêm toolchain nhiều GB vào máy.
- CI run [#4](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/39895920429) thành công trên Windows x64, macOS Intel và Apple Silicon; cả ba installer artifacts đã được tạo.
- Bản Windows tải từ artifact có SHA-256 khớp GitHub (`911a4c21afc20457ee1bef3d6ca2be836127e1f1f79c32d8a97b0059c793a08a`), cài đè bản cũ và mở thành công ở phiên bản `0.2.0`.
- Khi chụp trạng thái giao diện, tải FFmpeg 9.0.2 đang ở khoảng 7–8%; chưa xác nhận tải xong hoặc xuất video mẫu. Chưa phát hành tag/release `v0.2.0`.
- **Tiếp theo:** xác nhận engine cài hoàn tất, xuất video thử trên Windows, sau đó ghi nhận smoke test macOS và quyết định phát hành.

#### 2026-10-23 · Cài engine FFmpeg trong ứng dụng

- Đổi luồng thiếu FFmpeg từ hướng dẫn cài system package sang nút tải/cài một lần sau thao tác chủ động của người dùng.
- Thêm archive cố định FFmpeg 9.0.2 cho Windows x64 và macOS Intel/Apple Silicon; mã Rust ghim SHA-256, giới hạn kích thước, giải nén executable, xác thực filter/encoder và dọn archive tạm.
- Engine cài vào app-local-data; sau cài, app dò engine trước PATH và render vẫn chạy local/offline. Có tiến độ tải và hủy tải.
- Thêm `THIRD_PARTY_NOTICES.md` với nhà cung cấp, phiên bản, hash, nguồn và license/build source đã biết; cập nhật CHANGELOG/README/RELEASE.
- Chưa biên dịch native source mới, chưa chạy luồng tải thực tế hoặc render trên Windows/macOS. Cần CI trên ba target, kiểm tra binary/checksum và review license/build provenance trước khi phát hành.
- **Tiếp theo:** build CI cho source mới, chạy downloader và render mẫu trên Windows, macOS Intel và Apple Silicon; ghi kết quả/dung lượng engine và chốt release.

#### 2026-10-23 · Khổ xuất và vị trí wave

- Thêm lựa chọn khổ dọc 1080 × 1920 và ngang 1920 × 1080; preview đổi tỷ lệ, thông tin đầu ra và gợi ý crop ảnh đồng bộ.
- Thêm ba vị trí wave trên/giữa/dưới; lựa chọn được truyền sang filtergraph FFmpeg. Wave giữ tỷ lệ với chiều cao khung, căn giữa ngang; vị trí dưới mặc định giữ gần bố cục hiện tại.
- Cập nhật README và CHANGELOG; ghi thêm mốc M7 và tiêu chí nghiệm thu ở đây. Tính năng được đưa vào phiên bản 0.3.0.
- Chưa chạy build, test hoặc render mẫu theo yêu cầu hiện tại; chưa xác nhận giao diện hay đầu ra trên Windows/macOS.
- **Tiếp theo:** kiểm tra build và xuất mẫu ở cả hai tỷ lệ, ba vị trí và hai kiểu sóng trước khi phát hành bản mới.

#### 2026-10-23 · Chuẩn bị installer Wavely v0.3.0

- Đồng bộ `VERSION`, metadata npm, Cargo và Tauri lên `0.3.0` bằng `npm run version:set -- 0.3.0`.
- Chuyển ghi chú khổ xuất và vị trí wave vào `CHANGELOG.md` dưới mục `0.3.0`; cập nhật README về trạng thái bản cài.
- Commit `f0c1a6e` đã push lên `feat/ffmpeg-engine-setup`; [CI run #6](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/39902478541) thành công trên Windows x64, macOS Intel và Apple Silicon.
- Artifact Windows có SHA-256 `0209bb8ab8e424bf7ce413797c32dcbcfaf9ee0ca4389e03e0be25e531a4a12c`; bộ cài NSIS chạy thành công và `wavely.exe` báo ProductVersion/FileVersion `0.3.0` tại `%LOCALAPPDATA%\Wavely\wavely.exe`.
- Chưa tạo tag/GitHub Release. Chưa xuất video mẫu hoặc kiểm tra giao diện/renderer trên macOS.
- **Tiếp theo:** push cập nhật nhỏ phiên bản dự phòng trong giao diện, chờ CI cho HEAD mới; sau đó xuất video mẫu trên Windows và ghi nhận smoke test macOS trước release.

#### 2026-10-23 · Cài và xác nhận Wavely v0.3.0

- [CI run #7](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/39903024585) thành công trên Windows x64, macOS Intel và Apple Silicon cho source commit `906be3e`.
- Artifact Windows đã xác minh SHA-256 `abe74edfad7ba6efe685fd3f4ebe005626543c8da449c5d5934870feb0f958e5`; ứng dụng cài tại `%LOCALAPPDATA%\Wavely\wavely.exe` báo ProductVersion/FileVersion `0.3.0`, shortcut Start Menu trỏ đúng executable.
- Source và tiến độ đã được commit/push trên `feat/ffmpeg-engine-setup`; commit này chỉ cập nhật trạng thái tài liệu sau source commit CI nêu trên.
- Chưa tạo tag/GitHub Release. Chưa xuất video mẫu trên Windows hoặc chạy smoke test macOS; hai kiểm tra này vẫn cần trước khi phát hành.
- **Tiếp theo:** kiểm tra engine FFmpeg và render video mẫu trên Windows, sau đó smoke test trên macOS Intel/Apple Silicon.

#### 2026-10-23 · Chuẩn bị phát hành Wavely v0.3.0

- Người dùng xác nhận đã xuất được video bằng Wavely v0.3.0.
- [CI run #8](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/39903574933) xanh trên Windows x64, macOS Intel và Apple Silicon; job macOS Intel cần chạy lại do runner trước đó không tháo được disk image.
- Cập nhật README sang bộ cài v0.3.0 và sửa tiêu đề release notes cho phù hợp cả video dọc/ngang.
- Chưa smoke test ứng dụng trên máy macOS; bộ cài macOS được build trên runner CI. Chưa tạo tag/GitHub Release.
- **Tiếp theo:** hoàn tất CI sau cập nhật tài liệu, merge PR đã review vào `main`, đẩy tag `v0.3.0` và kiểm tra đủ installer trong GitHub Release.

#### 2026-10-23 · Phát hành Wavely v0.3.0

- Người dùng xác nhận đã xuất video thành công bằng Wavely v0.3.0.
- CI run [#9](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/39916035806) xanh trên Windows x64, macOS Intel và Apple Silicon; PR [#1](https://github.com/tinycorn-studio/auto_video_generate/pull/1) đã merge vào `main` tại commit `5eece92`.
- Tag annotated `v0.3.0` đã được đẩy. [Release workflow](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/39916739948) thành công; [GitHub Release v0.3.0](https://github.com/tinycorn-studio/auto_video_generate/releases/tag/v0.3.0) có Windows `.exe`/`.msi`, macOS Intel `.dmg` và macOS Apple Silicon `.dmg`.
- Smoke test trực tiếp trên macOS và benchmark thời gian/RAM chưa thực hiện; CI xác nhận build/package chứ không thay thế kiểm tra chạy app trên máy Mac.
- **Tiếp theo:** smoke test app trên macOS Intel/Apple Silicon; benchmark clip 1 và 5 phút ở hai khổ Full HD; kiểm tra các tổ hợp vị trí wave và preset.

#### 2026-10-23 · Sóng cân xứng và bố cục 9 vị trí

- Thêm preset Cân xứng, dùng FFmpeg `showwaves` ở `mode=cline` để các vạch biên độ phát triển đồng đều quanh trục giữa; preset Tối giản và Spectrum vẫn giữ nguyên.
- Thay lựa chọn ba vùng dọc bằng lưới 3 × 3 (gồm góc trái/phải và các vị trí giữa); mặc định giữ vị trí dưới chính giữa như trước.
- Thêm thanh scale 25–100%; kích thước vùng lọc thay đổi theo scale để tránh render toàn bộ wave rồi thu nhỏ, đồng thời giữ cùng tỷ lệ trong preview và video xuất.
- Cập nhật README, CHANGELOG v0.4.0, và metadata version đồng bộ. `npm run build` (TypeScript + Vite) và `git diff --check` thành công.
- Rustup 1.29.1 và Cargo/Rust stable x64 được cài bằng winget theo yêu cầu trước đó. `cargo check` local không truy cập được `index.crates.io`; native compile sau đó được CI xác nhận trên ba nền tảng và người dùng xác nhận đã xuất video bằng preview Windows. Trình duyệt preview tích hợp trước đó không truy cập được server `127.0.0.1:1420`.
- Kiểm tra hệ thống không tìm thấy Microsoft C++ Build Tools, prerequisite của Tauri trên Windows.
- Trước khi lấy artifact preview, Wavely đang cài tại `%LOCALAPPDATA%\Wavely\wavely.exe` và báo phiên bản 0.3.0.
- PR [#2](https://github.com/tinycorn-studio/auto_video_generate/pull/2) trên nhánh `feature/wave-layout-v0.4.0-preview`; commit `bf8dcc3` chứa mã nguồn đã build.
- GitHub Actions run [#12](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/39920781588) thành công trên Windows x64, macOS Intel và Apple Silicon. Artifact Windows ZIP có SHA-256 `22b8c85e4a9590b23e62934bd039b93b62740dc5ce49bc8927cc735b900111b`.
- Installer NSIS `Wavely_0.4.0_x64-setup.exe` cài thành công (exit code 0) vào `%LOCALAPPDATA%\Wavely`; file `wavely.exe` báo ProductVersion/FileVersion 0.4.0 và app đã được mở.
- FFmpeg runtime còn nguyên tại `%LOCALAPPDATA%\com.tinycorn.wavely\engine\ffmpeg.exe`; chạy `-version` xác nhận FFmpeg 9.0.2, không cần tải lại sau khi cập nhật app.
- Người dùng xác nhận đã kiểm tra và xuất video thành công với Wavely v0.4.0 trên Windows.
- PR [#2](https://github.com/tinycorn-studio/auto_video_generate/pull/2) đã squash-merge vào `main` tại commit `e610a4f`; source, metadata version và CHANGELOG v0.4.0 đã được đẩy lên remote.
- Không tạo tag/GitHub Release theo yêu cầu hiện tại; v0.3.0 vẫn là release công khai mới nhất. Rust/C++ toolchain trên máy không tham gia build installer.
- **Tiếp theo:** tạo annotated tag `v0.4.0` để build và phát hành installer Windows/macOS khi người dùng yêu cầu phát hành.

#### 2026-10-27 · Tối ưu prompt nội dung XPiki

- Bổ sung hướng viết riêng cho YouTube/TikTok/Facebook/Instagram; tách nguồn người dùng thành JSON và giữ quy tắc chỉ viết thông tin có trong transcript/tóm tắt.
- Đặt độ dài gợi ý theo nền tảng, tránh clickbait, hashtag xu hướng không có căn cứ và lời kêu gọi tương tác máy móc; vẫn dùng một yêu cầu XPiki cho các nền tảng đã chọn.
- Cho phép 0–5 hashtag và 0–10 từ khóa, chuẩn hóa/loại trùng và kiểm tra độ dài caption gồm hashtag. Giao diện thể hiện rõ kết quả không có đề xuất.
- Bổ sung kiểm tra hồi quy cho metadata tùy chọn, loại trùng và giới hạn caption Unicode. `npm run build`, định dạng Rust và `git diff --check` đạt; test Rust local bị chặn vì chưa cache `keyring`/thiếu C++ Build Tools.
- CI [run 40442693218](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/40442693218) của commit `115f9c2` build thành công và đạt cả 7 kiểm tra publishing trên Windows/macOS Intel/Apple Silicon. Artifact Windows được xác minh SHA-256 rồi giải nén và mở tại `previews/windows-115f9c2/app/PFiles/Wavely/wavely.exe` để thử prompt mới; chưa gọi API thật để đánh giá chất lượng bài sinh.

#### 2026-10-27 · Một form Gemini key cho Google và XPiki

- Thêm nhận diện key Google (`AIza…`, `AQ.…`) và XPiki (`sk-…`), kiểm tra model tại đúng endpoint; không thử chéo key hoặc đổi dịch vụ khi API lỗi.
- Lưu loại dịch vụ cùng key trong một credential bảo mật; đọc tương thích key XPiki cũ, chỉ thay bản ghi sau khi kiểm tra model thành công. Frontend chỉ nhận trạng thái và loại dịch vụ.
- Thêm request/response Google Generate Content, chuẩn hóa đầu ra về bộ kiểm tra bài đăng hiện tại. Giữ nguyên form văn bản, prompt và đầu ra `.txt`.
- Đổi nhãn chính sang Gemini API key, không thêm lựa chọn dịch vụ; chú thích văn bản hiển thị đúng nơi nhận dữ liệu.
- Bổ sung test nhận diện, tương thích credential cũ, ràng buộc host/header, metadata model, phản hồi Google và bảo vệ key trong trạng thái frontend. `npm run build`, định dạng Rust và `git diff --check` đạt.
- CI [run 40446321928](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/40446321928) tại commit `d6bcc65` build thành công và đạt 14 kiểm tra publishing trên Windows/macOS Intel/Apple Silicon. Artifact Windows được xác minh SHA-256, giải nén và mở tại `previews/windows-d6bcc65/app/PFiles/Wavely/wavely.exe`.
- Smoke test Windows xác nhận nhãn Gemini API key hiển thị, nhận được key XPiki đã lưu và không có lỗi khởi tạo. Chưa gọi Google bằng key thật; người dùng có thể nhập key trực tiếp trong app để thử dịch vụ/quota tài khoản của mình. Chưa smoke test trực tiếp app trên macOS.

#### 2026-10-27 · Nguồn audio cho nội dung đăng bài (v0.5.0)

- Thêm chế độ gửi audio đã chọn (mặc định) hoặc nhập transcript; lưu lựa chọn nguồn cùng ngôn ngữ, nền tảng và trạng thái tạo nội dung.
- Google Gemini nhận các định dạng audio của ứng dụng; dùng Files API cho file trên 14 MiB và yêu cầu xóa upload sau khi sinh. Với XPiki, chỉ gửi khi model công bố hỗ trợ audio và giới hạn WAV/MP3 tối đa 14 MiB; model hiện chỉ hỗ trợ văn bản vẫn dùng được ở chế độ transcript.
- Cập nhật prompt, thông báo dữ liệu được gửi, UI xem trước, README và CHANGELOG v0.5.0; không lưu lịch sử bài đăng và không kiểm tra trùng lặp.
- Người dùng xác nhận đã chạy thử app local trên macOS. `cargo test --manifest-path src-tauri/Cargo.toml --lib publishing::` thành công với 14/14 test; CI run [#40540574258](https://github.com/tinycorn-studio/auto_video_generate/actions/runs/40540574258) build installer, chạy publishing regressions và upload artifacts thành công trên Windows x64, macOS Intel và Apple Silicon.
- PR [#3](https://github.com/tinycorn-studio/auto_video_generate/pull/3) chờ merge và phát hành; chưa gắn tag hoặc publish v0.5.0.

## 8. Tài liệu tham khảo

- [FFmpeg showfreqs](https://ffmpeg.org/ffmpeg-filters.html#showfreqs) — chế độ bar/line/dot và các cài đặt phổ tần.
- [FFmpeg showwaves](https://ffmpeg.org/ffmpeg-filters.html#showwaves) — chế độ hiển thị mẫu sóng âm.
- [FFmpeg 6.1 showwaves source](https://ffmpeg.org/doxygen/6.1/avf__showwaves_8c_source.html) — định nghĩa `cline` là chế độ vẽ đường lấy mẫu quanh tâm.
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

