use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackgroundKind {
    Image,
    Video,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DurationMode {
    FitAudio,
    FitVideo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShortVideoMode {
    Loop,
    Freeze,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PerformanceMode {
    Auto,
    CpuFast,
    CpuCompact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum VideoEncoder {
    Nvenc,
    Qsv,
    Amf,
    VideoToolbox,
    Libx264Fast,
    Libx264Compact,
}

impl VideoEncoder {
    pub fn id(self) -> &'static str {
        match self {
            Self::Nvenc => "h264_nvenc",
            Self::Qsv => "h264_qsv",
            Self::Amf => "h264_amf",
            Self::VideoToolbox => "h264_videotoolbox",
            Self::Libx264Fast => "libx264-fast",
            Self::Libx264Compact => "libx264-compact",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Nvenc => "GPU NVIDIA NVENC",
            Self::Qsv => "GPU Intel QuickSync",
            Self::Amf => "GPU AMD AMF",
            Self::VideoToolbox => "GPU Apple VideoToolbox",
            Self::Libx264Fast => "CPU siêu nhanh",
            Self::Libx264Compact => "CPU nén gọn",
        }
    }

    pub fn is_hardware(self) -> bool {
        matches!(
            self,
            Self::Nvenc | Self::Qsv | Self::Amf | Self::VideoToolbox
        )
    }

    fn pix_fmt(self) -> &'static str {
        match self {
            Self::Qsv => "nv12",
            _ => "yuv420p",
        }
    }

    fn ffmpeg_args(self) -> Vec<OsString> {
        let args: &[&str] = match self {
            Self::Nvenc => &[
                "-c:v", "h264_nvenc", "-preset", "p3", "-rc", "vbr", "-cq", "23", "-pix_fmt",
                "yuv420p",
            ],
            Self::Qsv => &[
                "-c:v", "h264_qsv", "-preset", "veryfast", "-global_quality", "23", "-pix_fmt",
                "nv12",
            ],
            Self::Amf => &[
                "-c:v", "h264_amf", "-usage", "transcoding", "-quality", "speed", "-rc", "cqp",
                "-qp_i", "21", "-qp_p", "22", "-pix_fmt", "yuv420p",
            ],
            Self::VideoToolbox => &["-c:v", "h264_videotoolbox", "-q:v", "65", "-pix_fmt", "yuv420p"],
            Self::Libx264Fast => &[
                "-c:v", "libx264", "-preset", "superfast", "-crf", "22", "-pix_fmt", "yuv420p",
            ],
            Self::Libx264Compact => &[
                "-c:v", "libx264", "-preset", "veryfast", "-crf", "21", "-pix_fmt", "yuv420p",
            ],
        };
        args.iter().map(OsString::from).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub duration: f64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub has_video: bool,
    pub has_audio: bool,
    pub sar_is_one: bool,
    pub has_rotation: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderCapabilities {
    pub hardware_encoder: Option<String>,
    pub hardware_label: Option<String>,
    pub recommended_label: String,
}

#[derive(Clone, Debug)]
pub struct RenderSpec<'a> {
    pub background_kind: BackgroundKind,
    pub background_path: &'a Path,
    pub audio_path: &'a Path,
    pub temp_output: &'a Path,
    pub style: &'a str,
    pub output_format: &'a str,
    pub wave_position: &'a str,
    pub wave_scale: u8,
    pub duration_mode: DurationMode,
    pub short_video_mode: ShortVideoMode,
    pub start_delay_sec: f64,
    pub end_delay_sec: f64,
    pub keep_video_audio: bool,
    pub video_info: Option<&'a MediaInfo>,
    pub audio_info: &'a MediaInfo,
}

#[derive(Clone, Debug)]
pub struct RenderPlan {
    pub target_duration: f64,
    pub encoder: VideoEncoder,
    pub args: Vec<OsString>,
}

static HW_ENCODER_CACHE: OnceLock<Mutex<Option<(PathBuf, Option<VideoEncoder>)>>> = OnceLock::new();

pub fn configure_ffmpeg_command(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Keep the UI responsive during heavy video renders and avoid console window flashes.
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(BELOW_NORMAL_PRIORITY_CLASS | CREATE_NO_WINDOW);
    }
}

pub fn parse_media_probe(diagnostics: &str) -> Result<MediaInfo, String> {
    let mut duration: Option<f64> = None;
    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    let mut fps: Option<f64> = None;
    let mut has_video = false;
    let mut has_audio = false;
    let mut sar_is_one = true;
    let mut has_rotation = false;
    let mut in_primary_video_stream = false;

    for line in diagnostics.lines() {
        if duration.is_none() {
            if let Some(start) = line.find("Duration: ") {
                let value = line[start + 10..].split(',').next().unwrap_or("").trim();
                let mut pieces = value.split(':');
                let hours = pieces.next().and_then(|part| part.parse::<f64>().ok());
                let minutes = pieces.next().and_then(|part| part.parse::<f64>().ok());
                let seconds = pieces.next().and_then(|part| part.parse::<f64>().ok());
                if let (Some(h), Some(m), Some(s)) = (hours, minutes, seconds) {
                    let total = h * 3600.0 + m * 60.0 + s;
                    if total.is_finite() && total > 0.0 {
                        duration = Some(total);
                    }
                }
            }
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with("Stream #") {
            in_primary_video_stream = false;
            if trimmed.contains("Video:") {
                if !trimmed.contains("attached pic") && !has_video {
                    has_video = true;
                    in_primary_video_stream = true;
                    if let Some((w, h)) = parse_dimensions(trimmed) {
                        width = Some(w);
                        height = Some(h);
                    }
                    sar_is_one = parse_sar_is_one(trimmed);
                    fps = parse_fps(trimmed);
                }
            } else if trimmed.contains("Audio:") {
                has_audio = true;
            }
        } else if in_primary_video_stream {
            if let Some(idx) = trimmed.find("displaymatrix: rotation of ") {
                let rest = &trimmed[idx + "displaymatrix: rotation of ".len()..];
                if let Some(angle_str) = rest.split_whitespace().next() {
                    if let Ok(angle) = angle_str.parse::<f64>() {
                        if angle.abs() > 0.01 {
                            has_rotation = true;
                        }
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("rotate") {
                let rest = rest.trim_start();
                if let Some(val_str) = rest.strip_prefix(':') {
                    if let Ok(angle) = val_str.trim().parse::<f64>() {
                        if angle.abs() > 0.01 {
                            has_rotation = true;
                        }
                    }
                }
            }
        }
    }

    let duration = duration.ok_or_else(|| {
        let detail = diagnostics
            .lines()
            .last()
            .unwrap_or("Không tìm thấy thời lượng.")
            .trim();
        format!("Không đọc được thời lượng file. {detail}")
    })?;

    Ok(MediaInfo {
        duration,
        width,
        height,
        fps,
        has_video,
        has_audio,
        sar_is_one,
        has_rotation,
    })
}

fn parse_dimensions(line: &str) -> Option<(u32, u32)> {
    for raw_token in line.split(|c: char| c == ',' || c.is_whitespace() || c == '[' || c == ']') {
        let token = raw_token.trim();
        let Some((w_str, h_str)) = token.split_once('x') else {
            continue;
        };
        if w_str.is_empty() || h_str.is_empty() {
            continue;
        }
        if !w_str.bytes().all(|b| b.is_ascii_digit()) || !h_str.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let (Ok(w), Ok(h)) = (w_str.parse::<u32>(), h_str.parse::<u32>()) {
            if w >= 16 && h >= 16 {
                return Some((w, h));
            }
        }
    }
    None
}

fn parse_sar_is_one(line: &str) -> bool {
    let Some(idx) = line.find("SAR ") else {
        return true;
    };
    let rest = &line[idx + 4..];
    let Some(ratio) = rest.split_whitespace().next() else {
        return true;
    };
    let Some((num, den)) = ratio.split_once(':') else {
        return true;
    };
    match (num.parse::<u32>(), den.parse::<u32>()) {
        (Ok(a), Ok(b)) if b > 0 => a == b,
        _ => true,
    }
}

fn parse_fps(line: &str) -> Option<f64> {
    let idx = line.find(" fps")?;
    let before = line[..idx].trim_end();
    let num_str = before.rsplit(|c: char| c == ',' || c.is_whitespace()).next()?;
    num_str.parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0)
}

pub fn probe_media(ffmpeg: &Path, path: &Path) -> Result<MediaInfo, String> {
    let mut cmd = Command::new(ffmpeg);
    configure_ffmpeg_command(&mut cmd);
    let output = cmd
        .args(["-hide_banner", "-nostdin", "-i"])
        .arg(path)
        .output()
        .map_err(|error| format!("Không đọc được file media: {error}"))?;
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    parse_media_probe(&diagnostics)
}

fn hardware_candidates() -> &'static [VideoEncoder] {
    #[cfg(target_os = "macos")]
    {
        &[VideoEncoder::VideoToolbox]
    }
    #[cfg(target_os = "windows")]
    {
        &[VideoEncoder::Nvenc, VideoEncoder::Qsv, VideoEncoder::Amf]
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        &[]
    }
}

pub fn detect_hardware_encoder(ffmpeg: &Path) -> Option<VideoEncoder> {
    let cache = HW_ENCODER_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some((cached_path, cached_encoder)) = guard.as_ref() {
            if cached_path == ffmpeg {
                return *cached_encoder;
            }
        }
    }

    let mut enc_cmd = Command::new(ffmpeg);
    configure_ffmpeg_command(&mut enc_cmd);
    let encoder_list = enc_cmd
        .args(["-hide_banner", "-encoders"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| {
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            )
        })
        .unwrap_or_default();

    let mut detected = None;
    for &candidate in hardware_candidates() {
        if !encoder_list.contains(candidate.id()) {
            continue;
        }
        let mut test_cmd = Command::new(ffmpeg);
        configure_ffmpeg_command(&mut test_cmd);
        let ok = test_cmd
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=256x256:d=0.1",
                "-frames:v",
                "1",
            ])
            .args(candidate.ffmpeg_args())
            .args(["-f", "null", "-"])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        if ok {
            detected = Some(candidate);
            break;
        }
    }

    if let Ok(mut guard) = cache.lock() {
        *guard = Some((ffmpeg.to_path_buf(), detected));
    }
    detected
}

pub fn resolve_encoder(ffmpeg: &Path, mode: PerformanceMode) -> VideoEncoder {
    match mode {
        PerformanceMode::Auto => detect_hardware_encoder(ffmpeg).unwrap_or(VideoEncoder::Libx264Fast),
        PerformanceMode::CpuFast => VideoEncoder::Libx264Fast,
        PerformanceMode::CpuCompact => VideoEncoder::Libx264Compact,
    }
}

pub fn get_render_capabilities(ffmpeg: &Path) -> RenderCapabilities {
    let hw = detect_hardware_encoder(ffmpeg);
    let recommended_label = hw
        .map(|enc| enc.label().to_string())
        .unwrap_or_else(|| VideoEncoder::Libx264Fast.label().to_string());
    RenderCapabilities {
        hardware_encoder: hw.map(|enc| enc.id().to_string()),
        hardware_label: hw.map(|enc| enc.label().to_string()),
        recommended_label,
    }
}

struct GridGeometry {
    grid_w: u32,
    pitch: u32,
    gap: u32,
}

fn chunky_bar_geometry(target_width: u32, cols: u32) -> GridGeometry {
    let pitch = (target_width / cols).max(4);
    let raw_gap = (pitch * 2 / 5).clamp(2, pitch.saturating_sub(2).max(2));
    let gap = if raw_gap >= 4 { raw_gap & !1 } else { raw_gap };
    GridGeometry {
        grid_w: pitch * cols,
        pitch,
        gap,
    }
}

/// Builds a lightweight, zero-halo minimalist waveform chain using built-in FFmpeg filters.
fn build_wave_filter(
    style: &str,
    width: u32,
    height: u32,
    wave_scale: u8,
) -> Result<String, String> {
    let scale_percent = u32::from(wave_scale);
    let target_width = (width * 8 / 9 * scale_percent / 100).max(24);
    let base_wave_height = match style {
        "studio" => 144_u32,
        "eq" | "blocks" => 160_u32,
        _ => return Err("Kiểu sóng không hợp lệ.".to_string()),
    };
    let raw_height = (base_wave_height * height / 1920 * scale_percent / 100).max(16);

    let chain = match style {
        "studio" => {
            let geom = chunky_bar_geometry(target_width, 32);
            let half_h = (raw_height / 2).max(8);
            let grid_h = half_h * 2;
            let x_off = -(geom.gap as i32 / 2);
            let h_off = grid_h * 2;
            format!(
                "[a_viz]aformat=channel_layouts=mono,aresample=10000,showfreqs=s=32x{half_h}:mode=bar:ascale=log:fscale=log:win_size=512:averaging=3:rate=30:colors=0xFFFFFF,split[wt][wb];[wb]vflip[wbf];[wt][wbf]vstack,scale={grid_w}:{grid_h}:flags=neighbor,drawgrid=x={x_off}:y={grid_h}:w={pitch}:h={h_off}:t={gap}:color=black@1,format=rgba,colorkey=0x000000:0.07:0.0[wave]",
                grid_w = geom.grid_w,
                pitch = geom.pitch,
                gap = geom.gap,
            )
        }
        "eq" => {
            let geom = chunky_bar_geometry(target_width, 28);
            let grid_h = raw_height;
            let x_off = -(geom.gap as i32 / 2);
            let h_off = grid_h * 2;
            format!(
                "[a_viz]aformat=channel_layouts=mono,aresample=10000,showfreqs=s=28x{grid_h}:mode=bar:ascale=log:fscale=log:win_size=512:averaging=4:rate=30:colors=0xFF8FAB,scale={grid_w}:{grid_h}:flags=neighbor,drawgrid=x={x_off}:y={grid_h}:w={pitch}:h={h_off}:t={gap}:color=black@1,format=rgba,colorkey=0x000000:0.07:0.0[wave]",
                grid_w = geom.grid_w,
                pitch = geom.pitch,
                gap = geom.gap,
            )
        }
        "blocks" => {
            let geom = chunky_bar_geometry(target_width, 24);
            let rows = (raw_height / 18).clamp(4, 8);
            let row_pitch = (raw_height / rows).max(6);
            let grid_h = row_pitch * rows;
            let raw_brick_gap = (row_pitch * 2 / 5)
                .clamp(2, (geom.pitch.saturating_sub(2)).min(row_pitch.saturating_sub(2)).max(2));
            let brick_gap = if raw_brick_gap >= 4 { raw_brick_gap & !1 } else { raw_brick_gap };
            let x_off = -(brick_gap as i32 / 2);
            format!(
                "[a_viz]aformat=channel_layouts=mono,aresample=10000,showfreqs=s=24x{rows}:mode=bar:ascale=log:fscale=log:win_size=512:averaging=3:rate=30:colors=0x75E8E1,scale={grid_w}:{grid_h}:flags=neighbor,drawgrid=x={x_off}:y=0:w={pitch}:h={row_pitch}:t={brick_gap}:color=black@1,format=rgba,colorkey=0x000000:0.07:0.0[wave]",
                grid_w = geom.grid_w,
                pitch = geom.pitch,
            )
        }
        _ => unreachable!("Style was validated above"),
    };

    Ok(chain)
}

pub fn build_render_plan(spec: &RenderSpec<'_>, encoder: VideoEncoder) -> Result<RenderPlan, String> {
    let (width, height) = match spec.output_format {
        "portrait" => (1080_u32, 1920_u32),
        "landscape" => (1920_u32, 1080_u32),
        _ => return Err("Khổ video không hợp lệ.".to_string()),
    };
    if !(25..=100).contains(&spec.wave_scale) {
        return Err("Kích thước sóng phải nằm trong khoảng 25–100%".to_string());
    }
    if !spec.start_delay_sec.is_finite()
        || !spec.end_delay_sec.is_finite()
        || !(0.0..=300.0).contains(&spec.start_delay_sec)
        || !(0.0..=300.0).contains(&spec.end_delay_sec)
    {
        return Err("Khoảng delay đầu/cuối phải nằm trong khoảng 0 đến 300 giây.".to_string());
    }
    if !spec.audio_info.has_audio || !spec.audio_info.duration.is_finite() || spec.audio_info.duration <= 0.0 {
        return Err("File âm thanh không hợp lệ hoặc không có track âm thanh.".to_string());
    }

    let speech_span = spec.start_delay_sec + spec.audio_info.duration + spec.end_delay_sec;
    let mut use_stream_loop = false;
    let mut stretch_ratio: Option<f64> = None;
    let mut freeze_extra: Option<f64> = None;

    let target_duration = match spec.background_kind {
        BackgroundKind::Image => speech_span,
        BackgroundKind::Video => {
            let video_info = spec
                .video_info
                .ok_or_else(|| "Thiếu thông tin file video nền.".to_string())?;
            if !video_info.has_video || !video_info.duration.is_finite() || video_info.duration <= 0.0 {
                return Err("File video nền không có track hình ảnh hợp lệ.".to_string());
            }
            match spec.duration_mode {
                DurationMode::FitAudio => {
                    if video_info.duration + 0.02 < speech_span {
                        match spec.short_video_mode {
                            ShortVideoMode::Loop => use_stream_loop = true,
                            ShortVideoMode::Freeze => {
                                freeze_extra = Some((speech_span - video_info.duration + 0.5).max(0.5));
                            }
                            ShortVideoMode::Stretch => {
                                stretch_ratio = Some(speech_span / video_info.duration);
                            }
                        }
                    }
                    speech_span
                }
                DurationMode::FitVideo => {
                    if spec.start_delay_sec >= video_info.duration {
                        return Err(format!(
                            "Delay đầu ({:.1}s) phải nhỏ hơn độ dài video nền ({:.1}s).",
                            spec.start_delay_sec, video_info.duration
                        ));
                    }
                    video_info.duration
                }
            }
        }
    };

    let wave_filter = build_wave_filter(spec.style, width, height, spec.wave_scale)?;

    let (overlay_x, overlay_y) = match spec.wave_position {
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

    let bg_chain = match spec.background_kind {
        BackgroundKind::Image => format!(
            "[0:v]scale={width}:{height}:force_original_aspect_ratio=increase:flags=bilinear,crop={width}:{height},setsar=1,format=yuv420p[bg]"
        ),
        BackgroundKind::Video => {
            let video_info = spec.video_info.expect("validated above");
            let can_skip_scale = video_info.width == Some(width)
                && video_info.height == Some(height)
                && video_info.sar_is_one
                && !video_info.has_rotation;
            let mut steps = Vec::new();
            if let Some(ratio) = stretch_ratio {
                steps.push(format!("setpts=PTS*{ratio:.6}"));
            }
            steps.push("fps=30:round=near".to_string());
            if can_skip_scale {
                steps.push("setsar=1".to_string());
            } else {
                steps.push(format!(
                    "scale={width}:{height}:force_original_aspect_ratio=increase:flags=bilinear"
                ));
                steps.push(format!("crop={width}:{height}"));
                steps.push("setsar=1".to_string());
            }
            steps.push("format=yuv420p".to_string());
            if let Some(extra) = freeze_extra {
                steps.push(format!("tpad=stop_mode=clone:stop_duration={extra:.3}"));
            }
            format!("[0:v]{}[bg]", steps.join(","))
        }
    };

    let start_delay_ms = (spec.start_delay_sec * 1000.0).round() as u64;
    let mut audio_steps = vec!["aresample=async=1:first_pts=0".to_string()];
    if start_delay_ms > 0 {
        audio_steps.push(format!("adelay={start_delay_ms}:all=1"));
    }
    audio_steps.push(format!("apad=whole_dur={target_duration:.3}"));
    audio_steps.push(format!("atrim=0:{target_duration:.3}"));

    let mix_bg_audio = spec.background_kind == BackgroundKind::Video
        && spec.keep_video_audio
        && spec.video_info.is_some_and(|info| info.has_audio)
        && stretch_ratio.is_none();

    let audio_chain = if mix_bg_audio {
        format!(
            "[1:a]{},asplit=2[a_viz][a_spk];[0:a]aresample=async=1:first_pts=0,volume=0.30[a_bg];[a_spk][a_bg]amix=inputs=2:duration=first:normalize=0,atrim=0:{target_duration:.3}[a_out]",
            audio_steps.join(",")
        )
    } else {
        format!("[1:a]{},asplit=2[a_viz][a_out]", audio_steps.join(","))
    };

    let out_pix_fmt = encoder.pix_fmt();
    let graph = format!(
        "{bg_chain};{audio_chain};{wave_filter};[bg][wave]overlay=x={overlay_x}:y={overlay_y}:shortest=1:eval=init:format=auto,format={out_pix_fmt}[v]"
    );

    let duration_text = format!("{target_duration:.3}");
    let mut args: Vec<OsString> = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
    ];

    match spec.background_kind {
        BackgroundKind::Image => {
            args.extend([
                "-loop".into(),
                "1".into(),
                "-framerate".into(),
                "30".into(),
                "-i".into(),
                spec.background_path.as_os_str().to_owned(),
            ]);
        }
        BackgroundKind::Video => {
            if use_stream_loop {
                args.extend(["-stream_loop".into(), "-1".into()]);
            }
            args.extend(["-i".into(), spec.background_path.as_os_str().to_owned()]);
        }
    }

    args.extend([
        "-vn".into(),
        "-i".into(),
        spec.audio_path.as_os_str().to_owned(),
        "-filter_complex".into(),
        graph.into(),
        "-map".into(),
        "[v]".into(),
        "-map".into(),
        "[a_out]".into(),
    ]);
    args.extend(encoder.ffmpeg_args());
    args.extend([
        "-r".into(),
        "30".into(),
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "192k".into(),
        "-t".into(),
        duration_text.into(),
        "-shortest".into(),
        "-movflags".into(),
        "+faststart".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-nostats".into(),
        "-stats_period".into(),
        "0.5".into(),
        spec.temp_output.as_os_str().to_owned(),
    ]);

    Ok(RenderPlan {
        target_duration,
        encoder,
        args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_audio(duration: f64) -> MediaInfo {
        MediaInfo {
            duration,
            width: None,
            height: None,
            fps: None,
            has_video: false,
            has_audio: true,
            sar_is_one: true,
            has_rotation: false,
        }
    }

    fn sample_video(duration: f64, width: u32, height: u32, has_audio: bool) -> MediaInfo {
        MediaInfo {
            duration,
            width: Some(width),
            height: Some(height),
            fps: Some(60.0),
            has_video: true,
            has_audio,
            sar_is_one: true,
            has_rotation: false,
        }
    }

    fn arg_strings(plan: &RenderPlan) -> Vec<String> {
        plan.args
            .iter()
            .map(|a| a.to_string_lossy().to_string())
            .collect()
    }

    fn filter_graph(plan: &RenderPlan) -> String {
        let args = arg_strings(plan);
        let idx = args.iter().position(|a| a == "-filter_complex").unwrap();
        args[idx + 1].clone()
    }

    #[test]
    fn parses_video_with_audio_and_fps() {
        let stderr = "\
Input #0, mov,mp4,m4a, from 'clip.mp4':
  Duration: 00:01:12.50, start: 0.000000, bitrate: 8000 kb/s
  Stream #0:0[0x1](und): Video: h264 (High), yuv420p(progressive), 1920x1080 [SAR 1:1 DAR 16:9], 7800 kb/s, 59.94 fps, 59.94 tbr
  Stream #0:1[0x2](und): Audio: aac (LC), 48000 Hz, stereo, fltp, 192 kb/s
";
        let info = parse_media_probe(stderr).unwrap();
        assert!((info.duration - 72.5).abs() < 1e-6);
        assert_eq!(info.width, Some(1920));
        assert_eq!(info.height, Some(1080));
        assert!(info.fps.unwrap() > 59.9);
        assert!(info.has_video);
        assert!(info.has_audio);
        assert!(info.sar_is_one);
        assert!(!info.has_rotation);
    }

    #[test]
    fn parses_rotated_video_and_ignores_audio_attached_pic() {
        let rotated = "\
Input #0, mov,mp4, from 'phone.mov':
  Duration: 00:00:08.00, start: 0.000000
  Stream #0:0: Video: hevc, yuv420p10le, 1920x1080 [SAR 1:1 DAR 16:9], 30 fps
    Side data:
      displaymatrix: rotation of -90.00 degrees
";
        let info = parse_media_probe(rotated).unwrap();
        assert!(info.has_rotation);
        assert!(!info.has_audio);

        let mp3_cover = "\
Input #0, mp3, from 'song.mp3':
  Duration: 00:03:00.00, start: 0.000000
  Stream #0:0: Audio: mp3, 44100 Hz, stereo, fltp, 320 kb/s
  Stream #0:1: Video: mjpeg (Baseline), yuvj444p, 600x600 [SAR 1:1 DAR 1:1], 90k tbr (attached pic)
";
        let audio = parse_media_probe(mp3_cover).unwrap();
        assert!(audio.has_audio);
        assert!(!audio.has_video);
    }

    #[test]
    fn image_with_delays_builds_adelay_and_apad() {
        let audio = sample_audio(10.0);
        let plan = build_render_plan(
            &RenderSpec {
                background_kind: BackgroundKind::Image,
                background_path: Path::new("bg.png"),
                audio_path: Path::new("voice.wav"),
                temp_output: Path::new("out.mp4"),
                style: "studio",
                output_format: "portrait",
                wave_position: "bottom-center",
                wave_scale: 100,
                duration_mode: DurationMode::FitAudio,
                short_video_mode: ShortVideoMode::Loop,
                start_delay_sec: 1.5,
                end_delay_sec: 2.0,
                keep_video_audio: false,
                video_info: None,
                audio_info: &audio,
            },
            VideoEncoder::Libx264Fast,
        )
        .unwrap();

        assert!((plan.target_duration - 13.5).abs() < 1e-6);
        let graph = filter_graph(&plan);
        assert!(graph.contains("adelay=1500:all=1"));
        assert!(graph.contains("apad=whole_dur=13.500,atrim=0:13.500"));
        assert!(graph.contains("eval=init"));
        let args = arg_strings(&plan);
        assert!(args.contains(&"-vn".to_string()));
        assert!(args.contains(&"-loop".to_string()));
    }

    #[test]
    fn video_matching_dimensions_skips_scale_and_forces_fps_first() {
        let audio = sample_audio(12.0);
        let video = sample_video(5.0, 1080, 1920, true);
        let plan = build_render_plan(
            &RenderSpec {
                background_kind: BackgroundKind::Video,
                background_path: Path::new("bg.mp4"),
                audio_path: Path::new("voice.wav"),
                temp_output: Path::new("out.mp4"),
                style: "eq",
                output_format: "portrait",
                wave_position: "center",
                wave_scale: 80,
                duration_mode: DurationMode::FitAudio,
                short_video_mode: ShortVideoMode::Loop,
                start_delay_sec: 0.0,
                end_delay_sec: 0.0,
                keep_video_audio: true,
                video_info: Some(&video),
                audio_info: &audio,
            },
            VideoEncoder::Amf,
        )
        .unwrap();

        let args = arg_strings(&plan);
        assert!(args.contains(&"-stream_loop".to_string()));
        let graph = filter_graph(&plan);
        assert!(graph.contains("[0:v]fps=30:round=near,setsar=1,format=yuv420p[bg]"));
        assert!(!graph.contains("scale=1080:1920"));
        assert!(graph.contains("amix=inputs=2:duration=first:normalize=0"));
    }

    #[test]
    fn video_freeze_and_stretch_and_fit_video_validation() {
        let audio = sample_audio(8.0);
        let video = sample_video(4.0, 3840, 2160, true);

        let freeze = build_render_plan(
            &RenderSpec {
                background_kind: BackgroundKind::Video,
                background_path: Path::new("4k.mp4"),
                audio_path: Path::new("voice.wav"),
                temp_output: Path::new("out.mp4"),
                style: "blocks",
                output_format: "landscape",
                wave_position: "bottom-center",
                wave_scale: 100,
                duration_mode: DurationMode::FitAudio,
                short_video_mode: ShortVideoMode::Freeze,
                start_delay_sec: 1.0,
                end_delay_sec: 1.0,
                keep_video_audio: false,
                video_info: Some(&video),
                audio_info: &audio,
            },
            VideoEncoder::Qsv,
        )
        .unwrap();
        let freeze_graph = filter_graph(&freeze);
        assert!(freeze_graph.contains("flags=bilinear"));
        assert!(freeze_graph.contains("tpad=stop_mode=clone:stop_duration=6.500"));
        assert!(freeze_graph.contains("format=nv12[v]"));

        let stretch = build_render_plan(
            &RenderSpec {
                background_kind: BackgroundKind::Video,
                background_path: Path::new("4k.mp4"),
                audio_path: Path::new("voice.wav"),
                temp_output: Path::new("out.mp4"),
                style: "studio",
                output_format: "landscape",
                wave_position: "bottom-center",
                wave_scale: 100,
                duration_mode: DurationMode::FitAudio,
                short_video_mode: ShortVideoMode::Stretch,
                start_delay_sec: 0.0,
                end_delay_sec: 0.0,
                keep_video_audio: true,
                video_info: Some(&video),
                audio_info: &audio,
            },
            VideoEncoder::Libx264Compact,
        )
        .unwrap();
        let stretch_graph = filter_graph(&stretch);
        assert!(stretch_graph.contains("[0:v]setpts=PTS*2.000000,fps=30:round=near"));
        assert!(!stretch_graph.contains("amix="));

        let bad_fit_video = build_render_plan(
            &RenderSpec {
                background_kind: BackgroundKind::Video,
                background_path: Path::new("4k.mp4"),
                audio_path: Path::new("voice.wav"),
                temp_output: Path::new("out.mp4"),
                style: "studio",
                output_format: "landscape",
                wave_position: "bottom-center",
                wave_scale: 100,
                duration_mode: DurationMode::FitVideo,
                short_video_mode: ShortVideoMode::Loop,
                start_delay_sec: 5.0,
                end_delay_sec: 0.0,
                keep_video_audio: false,
                video_info: Some(&video),
                audio_info: &audio,
            },
            VideoEncoder::Libx264Fast,
        );
        assert!(bad_fit_video.is_err());
    }

    #[test]
    fn video_longer_than_audio_trims_without_loop_freeze_or_stretch() {
        let audio = sample_audio(8.0);
        let video = sample_video(30.0, 1920, 1080, false);
        let plan = build_render_plan(
            &RenderSpec {
                background_kind: BackgroundKind::Video,
                background_path: Path::new("long.mp4"),
                audio_path: Path::new("voice.wav"),
                temp_output: Path::new("out.mp4"),
                style: "studio",
                output_format: "landscape",
                wave_position: "bottom-center",
                wave_scale: 100,
                duration_mode: DurationMode::FitAudio,
                short_video_mode: ShortVideoMode::Loop,
                start_delay_sec: 1.0,
                end_delay_sec: 1.0,
                keep_video_audio: true,
                video_info: Some(&video),
                audio_info: &audio,
            },
            VideoEncoder::Libx264Fast,
        )
        .unwrap();

        assert!((plan.target_duration - 10.0).abs() < 1e-6);
        let args = arg_strings(&plan);
        assert!(!args.contains(&"-stream_loop".to_string()));
        let graph = filter_graph(&plan);
        assert!(!graph.contains("tpad="));
        assert!(!graph.contains("setpts="));
        assert!(!graph.contains("scale=1920:1080"));
        // Background video has no audio track, so amix must be omitted.
        assert!(!graph.contains("amix="));
    }

    #[test]
    fn rotated_or_anamorphic_video_forces_scale_and_crop() {
        let audio = sample_audio(5.0);
        let mut rotated_video = sample_video(10.0, 1080, 1920, false);
        rotated_video.has_rotation = true;
        let plan = build_render_plan(
            &RenderSpec {
                background_kind: BackgroundKind::Video,
                background_path: Path::new("rot.mp4"),
                audio_path: Path::new("voice.wav"),
                temp_output: Path::new("out.mp4"),
                style: "studio",
                output_format: "portrait",
                wave_position: "bottom-center",
                wave_scale: 100,
                duration_mode: DurationMode::FitAudio,
                short_video_mode: ShortVideoMode::Loop,
                start_delay_sec: 0.0,
                end_delay_sec: 0.0,
                keep_video_audio: false,
                video_info: Some(&rotated_video),
                audio_info: &audio,
            },
            VideoEncoder::Libx264Fast,
        )
        .unwrap();
        assert!(filter_graph(&plan).contains("scale=1080:1920"));

        let sar_stderr = "\
Input #0, matroska, from 'anamorphic.mkv':
  Duration: 00:00:10.00, start: 0.000000
  Stream #0:0: Video: h264, yuv420p, 1920x1080 [SAR 4:3 DAR 16:9], 30 fps
    Metadata:
      rotate          : 90
";
        let info = parse_media_probe(sar_stderr).unwrap();
        assert!(!info.sar_is_one);
        assert!(info.has_rotation);
    }

    #[test]
    fn live_ffmpeg_probe_and_render_roundtrip_when_engine_available() {
        let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
            return;
        };
        let ffmpeg = PathBuf::from(local_app_data)
            .join("com.tinycorn.wavely")
            .join("engine")
            .join("ffmpeg.exe");
        if !ffmpeg.is_file() {
            return;
        }

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("wavely-render-e2e-{nonce}"));
        std::fs::create_dir_all(&dir).unwrap();
        let bg_video = dir.join("input_bg.mp4");
        let speech_wav = dir.join("speech.wav");

        let mut make_vid = Command::new(&ffmpeg);
        configure_ffmpeg_command(&mut make_vid);
        assert!(make_vid
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=navy:s=1280x720:r=60:d=2",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=220:sample_rate=44100:duration=2",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-c:a",
                "aac",
            ])
            .arg(&bg_video)
            .status()
            .unwrap()
            .success());

        let mut make_wav = Command::new(&ffmpeg);
        configure_ffmpeg_command(&mut make_wav);
        assert!(make_wav
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=44100:duration=3",
                "-c:a",
                "pcm_s16le",
            ])
            .arg(&speech_wav)
            .status()
            .unwrap()
            .success());

        let video_info = probe_media(&ffmpeg, &bg_video).unwrap();
        let audio_info = probe_media(&ffmpeg, &speech_wav).unwrap();
        assert!(video_info.has_video && video_info.has_audio);
        assert_eq!(video_info.width, Some(1280));
        assert_eq!(video_info.height, Some(720));
        assert!((video_info.duration - 2.0).abs() < 0.2);
        assert!(audio_info.has_audio);
        assert!((audio_info.duration - 3.0).abs() < 0.2);

        let encoder = resolve_encoder(&ffmpeg, PerformanceMode::Auto);
        for style in ["studio", "eq", "blocks"] {
            let styled_out = dir.join(format!("final_{style}.mp4"));
            let plan = build_render_plan(
                &RenderSpec {
                    background_kind: BackgroundKind::Video,
                    background_path: &bg_video,
                    audio_path: &speech_wav,
                    temp_output: &styled_out,
                    style,
                    output_format: "portrait",
                    wave_position: "bottom-center",
                    wave_scale: 90,
                    duration_mode: DurationMode::FitAudio,
                    short_video_mode: ShortVideoMode::Loop,
                    start_delay_sec: 1.0,
                    end_delay_sec: 1.0,
                    keep_video_audio: true,
                    video_info: Some(&video_info),
                    audio_info: &audio_info,
                },
                encoder,
            )
            .unwrap();

            let mut run = Command::new(&ffmpeg);
            configure_ffmpeg_command(&mut run);
            let output = run.args(&plan.args).output().unwrap();
            assert!(
                output.status.success(),
                "FFmpeg render failed for {style}: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            let rendered = probe_media(&ffmpeg, &styled_out).unwrap();
            assert!(rendered.has_video && rendered.has_audio);
            assert_eq!(rendered.width, Some(1080));
            assert_eq!(rendered.height, Some(1920));
            assert!((rendered.fps.unwrap_or_default() - 30.0).abs() < 0.5);
            assert!((rendered.duration - 5.0).abs() < 0.25);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chunky_waveform_styles_and_validation() {
        let audio = sample_audio(6.0);
        for (style, format, scale) in [
            ("studio", "portrait", 100),
            ("eq", "landscape", 75),
            ("blocks", "portrait", 25),
        ] {
            let plan = build_render_plan(
                &RenderSpec {
                    background_kind: BackgroundKind::Image,
                    background_path: Path::new("bg.png"),
                    audio_path: Path::new("voice.wav"),
                    temp_output: Path::new("out.mp4"),
                    style,
                    output_format: format,
                    wave_position: "bottom-center",
                    wave_scale: scale,
                    duration_mode: DurationMode::FitAudio,
                    short_video_mode: ShortVideoMode::Loop,
                    start_delay_sec: 0.0,
                    end_delay_sec: 0.0,
                    keep_video_audio: false,
                    video_info: None,
                    audio_info: &audio,
                },
                VideoEncoder::Libx264Fast,
            )
            .unwrap();
            let graph = filter_graph(&plan);
            assert!(graph.contains("aresample=10000"));
            assert!(graph.contains("drawgrid="));
            assert!(graph.contains("flags=neighbor"));
            if style == "studio" {
                assert!(graph.contains("vflip"));
                assert!(graph.contains("vstack"));
            }
            if style == "blocks" {
                assert!(graph.contains("showfreqs=s=24x"));
            }
        }

        let invalid = build_render_plan(
            &RenderSpec {
                background_kind: BackgroundKind::Image,
                background_path: Path::new("bg.png"),
                audio_path: Path::new("voice.wav"),
                temp_output: Path::new("out.mp4"),
                style: "minimal",
                output_format: "portrait",
                wave_position: "bottom-center",
                wave_scale: 100,
                duration_mode: DurationMode::FitAudio,
                short_video_mode: ShortVideoMode::Loop,
                start_delay_sec: 0.0,
                end_delay_sec: 0.0,
                keep_video_audio: false,
                video_info: None,
                audio_info: &audio,
            },
            VideoEncoder::Libx264Fast,
        );
        assert!(invalid.is_err());
    }
}
