use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

mod publishing;

type JobMap = HashMap<String, Arc<Mutex<ManagedProcess>>>;

#[derive(Default)]
struct ActiveJobs(Arc<Mutex<JobMap>>);

struct ManagedProcess {
    child: Child,
    cancelled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineStatus {
    available: bool,
    version: Option<String>,
    message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderEvent {
    job_id: String,
    status: String,
    percent: f32,
    message: String,
    output_path: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineInstallEvent {
    status: String,
    percent: f32,
    message: String,
}

#[derive(Default)]
struct EngineInstallState {
    running: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
}

struct EnginePackage {
    url: &'static str,
    sha256: &'static str,
    archive: EngineArchive,
}

#[derive(Clone, Copy)]
enum EngineArchive {
    SevenZip,
    Zip,
}

fn engine_package() -> Result<EnginePackage, String> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Ok(EnginePackage {
            url: "https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-9.0.2-essentials_build.7z",
            sha256: "4705843ccaaf54257c16ad90f3e952ece33c17df964ecf7bfdbb0f49c7171077",
            archive: EngineArchive::SevenZip,
        });
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Ok(EnginePackage {
            url: "https://ffmpeg.martin-riedl.de/download/macos/amd64/1789931006_9.0.2/ffmpeg.zip",
            sha256: "7c6b4125b191cbf773832dc51f424cf2b6bb7da43007d1e066f95909e47cacd4",
            archive: EngineArchive::Zip,
        });
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok(EnginePackage {
            url: "https://ffmpeg.martin-riedl.de/download/macos/arm64/1789931890_9.0.2/ffmpeg.zip",
            sha256: "c8ed4c4e6978a03c485edbfe4e0a5dc2380f8a30bba5150531b31b094492d924",
            archive: EngineArchive::Zip,
        });
    }
    #[allow(unreachable_code)]
    Err("Bản cài engine hiện hỗ trợ Windows x64 và macOS Intel/Apple Silicon.".to_string())
}

fn emit_engine_progress(app: &AppHandle, status: &str, percent: f32, message: impl Into<String>) {
    let _ = app.emit(
        "ffmpeg-install-progress",
        EngineInstallEvent { status: status.to_string(), percent, message: message.into() },
    );
}

fn engine_executable_name() -> &'static str {
    if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }
}

fn engine_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|path| path.join("engine"))
        .map_err(|error| format!("Không truy cập được thư mục dữ liệu Wavely: {error}"))
}

fn download_engine_archive(
    app: &AppHandle,
    package: &EnginePackage,
    destination: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(300))
        .user_agent(concat!("Wavely/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("Không khởi tạo được kết nối tải FFmpeg: {error}"))?;
    let mut response = client
        .get(package.url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("Không tải được FFmpeg. Kiểm tra kết nối Internet rồi thử lại: {error}"))?;
    let total = response.content_length().unwrap_or(0);
    const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
    if total > MAX_ARCHIVE_BYTES {
        return Err("Kích thước gói FFmpeg không hợp lệ.".to_string());
    }

    let mut file = File::create(destination).map_err(|error| format!("Không tạo được file tải tạm: {error}"))?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    let mut last_emit = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("__cancelled__".to_string());
        }
        let count = response.read(&mut buffer).map_err(|error| format!("Lỗi khi tải FFmpeg: {error}"))?;
        if count == 0 { break; }
        downloaded = downloaded.saturating_add(count as u64);
        if (total > 0 && downloaded > total) || downloaded > MAX_ARCHIVE_BYTES {
            return Err("Gói tải xuống vượt quá kích thước cho phép.".to_string());
        }
        file.write_all(&buffer[..count]).map_err(|error| format!("Không ghi được gói FFmpeg: {error}"))?;
        hasher.update(&buffer[..count]);
        if last_emit.elapsed() >= Duration::from_millis(250) {
            let percent = if total > 0 {
                (downloaded as f64 / total as f64 * 85.0).clamp(0.0, 85.0) as f32
            } else {
                (downloaded as f64 / MAX_ARCHIVE_BYTES as f64 * 85.0).clamp(0.0, 85.0) as f32
            };
            let detail = if total > 0 {
                format!("Đang tải engine FFmpeg · {}%", (downloaded as f64 / total as f64 * 100.0).round() as u32)
            } else {
                format!("Đang tải engine FFmpeg · {:.1} MB", downloaded as f64 / 1_048_576.0)
            };
            emit_engine_progress(app, "downloading", percent, detail);
            last_emit = Instant::now();
        }
    }
    file.flush().map_err(|error| format!("Không hoàn tất được gói FFmpeg: {error}"))?;
    if total > 0 && downloaded != total {
        return Err("Gói FFmpeg tải xuống chưa hoàn chỉnh.".to_string());
    }
    let actual_hash = format!("{:x}", hasher.finalize());
    if actual_hash != package.sha256 {
        return Err("Không khớp SHA-256 của FFmpeg. Gói tải xuống đã bị từ chối.".to_string());
    }
    Ok(())
}

fn find_named_file(directory: &Path, filename: &str) -> Result<Option<PathBuf>, String> {
    let entries = fs::read_dir(directory).map_err(|error| format!("Không đọc được thư mục giải nén: {error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("Không đọc được file giải nén: {error}"))?;
        let kind = entry.file_type().map_err(|error| format!("Không xác định được file giải nén: {error}"))?;
        if kind.is_file() && entry.file_name().to_string_lossy().eq_ignore_ascii_case(filename) {
            return Ok(Some(entry.path()));
        }
        if kind.is_dir() {
            if let Some(found) = find_named_file(&entry.path(), filename)? {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

fn extract_engine_archive(archive_path: &Path, destination: &Path, format: EngineArchive) -> Result<PathBuf, String> {
    fs::create_dir_all(destination).map_err(|error| format!("Không tạo được thư mục giải nén: {error}"))?;
    match format {
        EngineArchive::SevenZip => {
            sevenz_rust2::decompress_file(archive_path, destination)
                .map_err(|error| format!("Không thể giải nén FFmpeg: {error}"))?;
            find_named_file(destination, "ffmpeg.exe")?
                .ok_or_else(|| "Trong gói không tìm thấy ffmpeg.exe.".to_string())
        }
        EngineArchive::Zip => {
            let file = File::open(archive_path).map_err(|error| format!("Không mở được gói FFmpeg: {error}"))?;
            let mut archive = zip::ZipArchive::new(file).map_err(|error| format!("Gói FFmpeg ZIP không hợp lệ: {error}"))?;
            let index = (0..archive.len()).find(|index| {
                archive.by_index(*index).map(|entry| {
                    !entry.is_dir() && entry.name().rsplit('/').next() == Some("ffmpeg")
                }).unwrap_or(false)
            }).ok_or_else(|| "Trong gói không tìm thấy file ffmpeg.".to_string())?;
            let mut source = archive.by_index(index).map_err(|error| format!("Không đọc được FFmpeg trong gói: {error}"))?;
            let extracted = destination.join("ffmpeg");
            let mut output = File::create(&extracted).map_err(|error| format!("Không tạo được file FFmpeg: {error}"))?;
            std::io::copy(&mut source, &mut output).map_err(|error| format!("Không giải nén được FFmpeg: {error}"))?;
            output.flush().map_err(|error| format!("Không hoàn tất được file FFmpeg: {error}"))?;
            Ok(extracted)
        }
    }
}

fn install_engine_files(app: &AppHandle, cancel: &AtomicBool, work_dir: &Path) -> Result<String, String> {
    let package = engine_package()?;
    fs::create_dir_all(work_dir).map_err(|error| format!("Không tạo được thư mục tạm: {error}"))?;
    let archive_path = work_dir.join(match package.archive { EngineArchive::SevenZip => "ffmpeg.7z", EngineArchive::Zip => "ffmpeg.zip" });
    emit_engine_progress(app, "downloading", 0.0, "Đang kết nối tới nguồn FFmpeg…");
    download_engine_archive(app, &package, &archive_path, cancel)?;
    if cancel.load(Ordering::Relaxed) { return Err("__cancelled__".to_string()); }

    emit_engine_progress(app, "extracting", 87.0, "Đã xác minh gói tải xuống · đang cài engine…");
    let extract_dir = work_dir.join("unpacked");
    let extracted = extract_engine_archive(&archive_path, &extract_dir, package.archive)?;
    if cancel.load(Ordering::Relaxed) { return Err("__cancelled__".to_string()); }

    let engine_dir = engine_data_dir(app)?;
    fs::create_dir_all(&engine_dir).map_err(|error| format!("Không tạo được thư mục engine: {error}"))?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let staged = engine_dir.join(format!(".ffmpeg-stage-{nonce}"));
    fs::copy(&extracted, &staged).map_err(|error| format!("Không cài được file FFmpeg: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&staged).map_err(|error| format!("Không đọc được quyền file FFmpeg: {error}"))?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&staged, permissions).map_err(|error| format!("Không cấp quyền chạy FFmpeg: {error}"))?;
    }
    match ffmpeg_problem(&staged) {
        Some(result) if result.starts_with("ready:") => {},
        Some(problem) => { let _ = fs::remove_file(&staged); return Err(format!("Engine tải về không đáp ứng yêu cầu: {problem}")); },
        None => { let _ = fs::remove_file(&staged); return Err("Không xác định được trạng thái engine FFmpeg.".to_string()); },
    }

    let target = engine_dir.join(engine_executable_name());
    let backup = engine_dir.join(format!(".ffmpeg-backup-{nonce}"));
    if target.exists() {
        fs::rename(&target, &backup).map_err(|error| format!("Không thay được engine hiện tại: {error}"))?;
    }
    if let Err(error) = fs::rename(&staged, &target) {
        if backup.exists() { let _ = fs::rename(&backup, &target); }
        let _ = fs::remove_file(&staged);
        return Err(format!("Không thể hoàn tất cài đặt FFmpeg: {error}"));
    }
    let _ = fs::remove_file(backup);
    let version = match ffmpeg_problem(&target) {
        Some(result) if result.starts_with("ready:") => result.trim_start_matches("ready:").to_string(),
        _ => return Err("FFmpeg đã cài nhưng không vượt qua kiểm tra sau cài đặt.".to_string()),
    };
    Ok(version)
}

fn install_engine_worker(app: AppHandle, cancel: Arc<AtomicBool>, running: Arc<AtomicBool>) {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let work_dir = app.path().app_local_data_dir().map(|path| path.join(format!(".ffmpeg-install-{nonce}")));
    let result = match &work_dir {
        Ok(path) => install_engine_files(&app, &cancel, path),
        Err(error) => Err(format!("Không truy cập được thư mục dữ liệu Wavely: {error}")),
    };
    if let Ok(path) = work_dir { let _ = fs::remove_dir_all(path); }
    running.store(false, Ordering::Release);
    match result {
        Ok(version) => emit_engine_progress(&app, "ready", 100.0, format!("Đã cài {version}. Engine được lưu trên máy.")),
        Err(error) if error == "__cancelled__" => emit_engine_progress(&app, "cancelled", 0.0, "Đã hủy tải FFmpeg. File tạm đã được dọn."),
        Err(error) => emit_engine_progress(&app, "error", 0.0, error),
    }
}

#[tauri::command]
fn install_ffmpeg(app: AppHandle, state: State<'_, EngineInstallState>) -> Result<(), String> {
    if state.running.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return Err("FFmpeg đang được tải hoặc cài đặt.".to_string());
    }
    state.cancel.store(false, Ordering::Release);
    let cancel = Arc::clone(&state.cancel);
    let running = Arc::clone(&state.running);
    thread::spawn(move || install_engine_worker(app, cancel, running));
    Ok(())
}

#[tauri::command]
fn cancel_ffmpeg_install(state: State<'_, EngineInstallState>) -> Result<(), String> {
    if !state.running.load(Ordering::Acquire) {
        return Err("Không có lượt cài FFmpeg nào đang chạy.".to_string());
    }
    state.cancel.store(true, Ordering::Release);
    Ok(())
}

fn locate_ffmpeg(app: &AppHandle) -> Option<PathBuf> {
    let filename = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    if let Ok(engine_dir) = engine_data_dir(app) {
        let managed = engine_dir.join(filename);
        if managed.is_file() {
            return Some(managed);
        }
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        let packaged = resource_dir.join("resources").join(filename);
        if packaged.is_file() {
            return Some(packaged);
        }
        let flat_packaged = resource_dir.join(filename);
        if flat_packaged.is_file() {
            return Some(flat_packaged);
        }
    }

    if let Some(path) = find_in_path(filename) {
        return Some(path);
    }

    #[cfg(target_os = "macos")]
    for path in ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"] {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    #[cfg(target_os = "windows")]
    if let Some(program_data) = std::env::var_os("ProgramData") {
        let candidate = PathBuf::from(program_data)
            .join("chocolatey")
            .join("bin")
            .join(filename);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

fn find_in_path(executable: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(executable);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let candidate = directory.join("ffmpeg.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn ffmpeg_problem(path: &Path) -> Option<String> {
    let version_result = Command::new(path).args(["-hide_banner", "-version"]).output();
    let version_output = match version_result {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).to_string(),
        Ok(output) => {
            let message = String::from_utf8_lossy(&output.stderr);
            return Some(format!("Không chạy được FFmpeg: {}", message.trim()));
        }
        Err(error) => return Some(format!("Không mở được FFmpeg: {error}")),
    };

    let filters = Command::new(path).args(["-hide_banner", "-filters"]).output();
    let filters = match filters {
        Ok(output) if output.status.success() => format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)),
        _ => return Some("FFmpeg không đọc được danh sách filter.".to_string()),
    };
    let encoders = Command::new(path).args(["-hide_banner", "-encoders"]).output();
    let encoders = match encoders {
        Ok(output) if output.status.success() => format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)),
        _ => return Some("FFmpeg không đọc được danh sách encoder.".to_string()),
    };

    if !filters.contains("showwaves") || !filters.contains("showfreqs") {
        return Some("FFmpeg này thiếu showwaves hoặc showfreqs. Hãy cài bản đầy đủ từ ffmpeg.org.".to_string());
    }
    if !encoders.contains("libx264") {
        return Some("FFmpeg này thiếu encoder libx264 cần cho video MP4.".to_string());
    }
    if !encoders.contains(" aac ") && !encoders.contains("aac") {
        return Some("FFmpeg này thiếu encoder AAC cần cho âm thanh MP4.".to_string());
    }

    let version = version_output.lines().next().unwrap_or("FFmpeg").trim().to_string();
    let major = version_output
        .split_once("version ")
        .and_then(|(_, value)| value.split_whitespace().next())
        .and_then(|value| value.split('.').next())
        .and_then(|value| value.parse::<u32>().ok());
    if major.is_some_and(|major| major < 6) {
        return Some("Cần FFmpeg phiên bản 6 trở lên để xuất video ổn định.".to_string());
    }
    Some(format!("ready:{version}"))
}

#[tauri::command]
fn ffmpeg_status(app: AppHandle) -> EngineStatus {
    let Some(path) = locate_ffmpeg(&app) else {
        return EngineStatus {
            available: false,
            version: None,
            message: "Chưa tìm thấy FFmpeg. Cài engine ngay trong Wavely hoặc tự cài FFmpeg 6 trở lên.".to_string(),
        };
    };
    match ffmpeg_problem(&path) {
        Some(result) if result.starts_with("ready:") => EngineStatus {
            available: true,
            version: Some(result.trim_start_matches("ready:").to_string()),
            message: path.display().to_string(),
        },
        Some(problem) => EngineStatus { available: false, version: None, message: problem },
        None => EngineStatus { available: false, version: None, message: "Không xác định được trạng thái FFmpeg.".to_string() },
    }
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn probe_duration(ffmpeg: &Path, audio: &Path) -> Result<f64, String> {
    let output = Command::new(ffmpeg)
        .args(["-hide_banner", "-nostdin", "-i"])
        .arg(audio)
        .output()
        .map_err(|error| format!("Không đọc được file âm thanh: {error}"))?;
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    for line in diagnostics.lines() {
        if let Some(start) = line.find("Duration: ") {
            let value = line[start + 10..].split(',').next().unwrap_or("").trim();
            let mut pieces = value.split(':');
            let hours = pieces.next().and_then(|part| part.parse::<f64>().ok());
            let minutes = pieces.next().and_then(|part| part.parse::<f64>().ok());
            let seconds = pieces.next().and_then(|part| part.parse::<f64>().ok());
            if let (Some(hours), Some(minutes), Some(seconds)) = (hours, minutes, seconds) {
                let duration = hours * 3600.0 + minutes * 60.0 + seconds;
                if duration.is_finite() && duration > 0.0 {
                    return Ok(duration);
                }
            }
        }
    }
    let detail = diagnostics.lines().last().unwrap_or("Không tìm thấy thời lượng.").trim();
    Err(format!("Không đọc được thời lượng âm thanh. {detail}"))
}

fn output_args(
    style: &str,
    output_format: &str,
    wave_position: &str,
    wave_scale: u8,
    duration: f64,
    image: &Path,
    audio: &Path,
    temp: &Path,
) -> Result<Vec<std::ffi::OsString>, String> {
    let (width, height) = match output_format {
        "portrait" => (1080_u32, 1920_u32),
        "landscape" => (1920_u32, 1080_u32),
        _ => return Err("Khổ video không hợp lệ.".to_string()),
    };
    if !(25..=100).contains(&wave_scale) {
        return Err("Kích thước sóng phải nằm trong khoảng 25–100%".to_string());
    }
    let scale_percent = u32::from(wave_scale);
    let wave_width = (width * 8 / 9 * scale_percent / 100).max(2);
    let base_wave_height = match style {
        "minimal" | "balanced" => 144_u32,
        "spectrum" => 176_u32,
        _ => return Err("Kiểu sóng không hợp lệ.".to_string()),
    };
    let wave_height = (base_wave_height * height / 1920 * scale_percent / 100).max(2);
    let wave_filter = match style {
        "minimal" => format!("[1:a]aformat=channel_layouts=mono,showwaves=s={wave_width}x{wave_height}:mode=line:rate=30:colors=0x75E8E1:scale=sqrt:draw=full,format=rgba,colorkey=0x000000:0.07:0.0[wave]"),
        "balanced" => format!("[1:a]aformat=channel_layouts=mono,showwaves=s={wave_width}x{wave_height}:mode=cline:rate=30:colors=0xFFFFFF:scale=sqrt:draw=full,format=rgba,colorkey=0x000000:0.07:0.0[wave]"),
        "spectrum" => format!("[1:a]aformat=channel_layouts=stereo,showfreqs=s={wave_width}x{wave_height}:mode=bar:ascale=log:fscale=log:win_size=2048:rate=30:colors=0xFF799A,format=rgba,colorkey=0x000000:0.07:0.0[wave]"),
        _ => unreachable!("Visualizer style was validated above"),
    };
    let (overlay_x, overlay_y) = match wave_position {
        "top-left" => ("W*0.0555", "H*0.05"),
        "top-center" => ("(W-w)/2", "H*0.05"),
        "top-right" => ("W-w-W*0.0555", "H*0.05"),
        "center-left" => ("W*0.0555", "(H-h)/2"),
        "center" => ("(W-w)/2", "(H-h)/2"),
        "center-right" => ("W-w-W*0.0555", "(H-h)/2"),
        "bottom-left" => ("W*0.0555", "H-h-H*0.05"),
        "bottom-center" => ("(W-w)/2", "H-h-H*0.05"),
        "bottom-right" => ("W-w-W*0.0555", "H-h-H*0.05"),
        _ => return Err("Vị trí sóng âm không hợp lệ.".to_string()),
    };
    let graph = format!(
        "[0:v]scale={width}:{height}:force_original_aspect_ratio=increase,crop={width}:{height},setsar=1[bg];{wave_filter};[bg][wave]overlay=x={overlay_x}:y={overlay_y}:shortest=1:format=auto,format=yuv420p[v]"
    );
    let duration_text = format!("{duration:.3}");
    let mut args = vec![
        "-hide_banner".into(), "-loglevel".into(), "error".into(), "-nostdin".into(), "-y".into(),
        "-loop".into(), "1".into(), "-framerate".into(), "30".into(), "-i".into(), image.as_os_str().to_owned(),
        "-i".into(), audio.as_os_str().to_owned(), "-filter_complex".into(), graph.into(),
        "-map".into(), "[v]".into(), "-map".into(), "1:a:0".into(),
        "-c:v".into(), "libx264".into(), "-preset".into(), "veryfast".into(), "-crf".into(), "21".into(),
        "-r".into(), "30".into(), "-pix_fmt".into(), "yuv420p".into(),
        "-af".into(), "aresample=async=1:first_pts=0".into(), "-c:a".into(), "aac".into(), "-b:a".into(), "192k".into(),
        "-t".into(), duration_text.into(), "-shortest".into(), "-movflags".into(), "+faststart".into(),
        "-progress".into(), "pipe:1".into(), "-nostats".into(), "-stats_period".into(), "0.5".into(),
    ];
    args.push(temp.as_os_str().to_owned());
    Ok(args)
}

fn emit(app: &AppHandle, event: RenderEvent) {
    let _ = app.emit("render-progress", event);
}

fn partial_path(output: &Path, job_id: &str, suffix: &str) -> PathBuf {
    let filename = output.file_name().unwrap_or_default().to_string_lossy();
    output.with_file_name(format!(".{filename}.{job_id}.{suffix}.mp4"))
}

fn replace_output(temp: &Path, output: &Path, job_id: &str) -> Result<(), String> {
    if !output.exists() {
        return fs::rename(temp, output).map_err(|error| format!("Không thể lưu video: {error}"));
    }
    let backup = partial_path(output, job_id, "previous");
    fs::rename(output, &backup).map_err(|error| format!("Không thể thay video đích đã tồn tại: {error}"))?;
    if let Err(error) = fs::rename(temp, output) {
        let _ = fs::rename(&backup, output);
        return Err(format!("Không thể hoàn tất video: {error}"));
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

#[tauri::command]
fn start_render(
    app: AppHandle,
    state: State<'_, ActiveJobs>,
    image_path: String,
    audio_path: String,
    output_path: String,
    style: String,
    output_format: String,
    wave_position: String,
    wave_scale: u8,
) -> Result<String, String> {
    {
        let jobs = state.0.lock().map_err(|_| "Không đọc được trạng thái tác vụ.".to_string())?;
        if !jobs.is_empty() {
            return Err("Đang có một video khác được xuất.".to_string());
        }
    }

    let image = PathBuf::from(image_path).canonicalize().map_err(|error| format!("Không tìm thấy ảnh: {error}"))?;
    let audio = PathBuf::from(audio_path).canonicalize().map_err(|error| format!("Không tìm thấy âm thanh: {error}"))?;
    if !image.is_file() || !audio.is_file() {
        return Err("Ảnh và âm thanh phải là file hợp lệ.".to_string());
    }
    let output = PathBuf::from(output_path);
    if output.extension().and_then(|extension| extension.to_str()).map(|extension| extension.to_lowercase()) != Some("mp4".to_string()) {
        return Err("Hãy chọn tên file đầu ra có đuôi .mp4.".to_string());
    }
    let parent = output.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
    if !parent.is_dir() {
        return Err("Thư mục lưu video không còn tồn tại.".to_string());
    }
    let output = if output.is_absolute() { output } else { std::env::current_dir().map_err(|error| error.to_string())?.join(output) };
    let ffmpeg = locate_ffmpeg(&app).ok_or_else(|| "Chưa tìm thấy FFmpeg. Cài FFmpeg 6 trở lên rồi mở lại ứng dụng.".to_string())?;
    if let Some(problem) = ffmpeg_problem(&ffmpeg) {
        if !problem.starts_with("ready:") {
            return Err(problem);
        }
    }
    let duration = probe_duration(&ffmpeg, &audio)?;
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let job_id = format!("{:x}", nanos);
    let temp = partial_path(&output, &job_id, "rendering");
    let args = output_args(&style, &output_format, &wave_position, wave_scale, duration, &image, &audio, &temp)?;

    let mut child = Command::new(&ffmpeg)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Không khởi động được FFmpeg: {error}"))?;
    let stdout = child.stdout.take().ok_or_else(|| "Không đọc được tiến độ FFmpeg.".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Không đọc được thông báo FFmpeg.".to_string())?;
    let process = Arc::new(Mutex::new(ManagedProcess { child, cancelled: false }));
    {
        let mut jobs = state.0.lock().map_err(|_| "Không lưu được tác vụ xuất video.".to_string())?;
        jobs.insert(job_id.clone(), process.clone());
    }

    let app_handle = app.clone();
    let jobs = state.0.clone();
    let job_key = job_id.clone();
    let output_string = output.to_string_lossy().to_string();
    thread::spawn(move || {
        let stderr_text = Arc::new(Mutex::new(String::new()));
        let stderr_capture = stderr_text.clone();
        let stderr_thread = thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut chunk = [0_u8; 4096];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => {
                        if let Ok(mut text) = stderr_capture.lock() {
                            text.push_str(&String::from_utf8_lossy(&chunk[..count]));
                            if text.len() > 24_000 {
                                let remove = text.len() - 24_000;
                                text.drain(..remove);
                            }
                        }
                    }
                }
            }
        });

        let duration_us = (duration * 1_000_000.0) as u64;
        let mut current_us = 0_u64;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(value) = line.strip_prefix("out_time_us=") {
                current_us = value.parse::<u64>().unwrap_or(current_us);
            }
            if line == "progress=continue" || line == "progress=end" {
                let percent = ((current_us as f64 / duration_us as f64) * 100.0).clamp(0.0, 99.0) as f32;
                emit(&app_handle, RenderEvent {
                    job_id: job_key.clone(), status: "running".to_string(), percent,
                    message: format!("Đang ghép video · {}%", percent.round() as u32), output_path: None,
                });
            }
        }

        let (status, cancelled) = match process.lock() {
            Ok(mut managed) => (managed.child.wait(), managed.cancelled),
            Err(_) => (Err(std::io::Error::other("process lock poisoned")), false),
        };
        let _ = stderr_thread.join();
        let result = match status {
            Ok(status) if status.success() && !cancelled => replace_output(&temp, &output, &job_key)
                .map(|_| ("completed", 100.0_f32, "Video đã xuất xong.".to_string())),
            Ok(_) if cancelled => {
                let _ = fs::remove_file(&temp);
                Ok(("cancelled", 0.0_f32, "Đã hủy xuất video.".to_string()))
            }
            Ok(_) => {
                let _ = fs::remove_file(&temp);
                let detail = stderr_text.lock().map(|text| text.trim().to_string()).unwrap_or_default();
                Err(if detail.is_empty() { "FFmpeg không thể xuất video. Hãy kiểm tra định dạng file đầu vào.".to_string() } else { detail })
            }
            Err(error) => {
                let _ = fs::remove_file(&temp);
                Err(format!("Không theo dõi được tiến trình FFmpeg: {error}"))
            }
        };

        if matches!(&result, Ok(("completed", _, _))) {
            publishing::register_export(&app_handle, &output);
        }
        // Release the job before notifying the UI so an immediate next render can start.
        if let Ok(mut active) = jobs.lock() {
            active.remove(&job_key);
        }
        match result {
            Ok((status, percent, message)) => emit(&app_handle, RenderEvent {
                job_id: job_key.clone(), status: status.to_string(), percent, message,
                output_path: if status == "completed" { Some(output_string) } else { None },
            }),
            Err(message) => emit(&app_handle, RenderEvent {
                job_id: job_key.clone(), status: "error".to_string(), percent: 0.0, message, output_path: None,
            }),
        }
    });

    Ok(job_id)
}

#[tauri::command]
fn cancel_render(state: State<'_, ActiveJobs>, job_id: String) -> Result<(), String> {
    let process = state
        .0
        .lock()
        .map_err(|_| "Không truy cập được tác vụ hiện tại.".to_string())?
        .get(&job_id)
        .cloned()
        .ok_or_else(|| "Tác vụ đã kết thúc hoặc không tồn tại.".to_string())?;
    let mut managed = process.lock().map_err(|_| "Không dừng được FFmpeg.".to_string())?;
    managed.cancelled = true;
    managed.child.kill().map_err(|error| format!("Không dừng được FFmpeg: {error}"))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(ActiveJobs::default())
        .manage(EngineInstallState::default())
        .manage(publishing::PublishingState::default())
        .invoke_handler(tauri::generate_handler![app_version, ffmpeg_status, install_ffmpeg, cancel_ffmpeg_install, start_render, cancel_render,
            publishing::xpiki_key_status, publishing::save_xpiki_key, publishing::delete_xpiki_key,
            publishing::generate_publishing_content, publishing::open_publishing_output])
        .run(tauri::generate_context!())
        .expect("Wavely could not start");
}
