use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

mod provider;
use provider::{Connection, Provider};

const MODEL: &str = "gemini-3.8-flash";
const PLATFORMS: [&str; 4] = ["youtube", "tiktok", "facebook", "instagram"];

#[derive(Clone)]
pub struct Export {
    video: PathBuf,
    audio: PathBuf,
    text: Option<PathBuf>,
}

// Only the current export lives in memory. No posting history is stored.
#[derive(Default)]
pub struct PublishingState {
    export: Mutex<Option<Export>>,
    running: Arc<AtomicBool>,
}

pub fn register_export(app: &AppHandle, video: &Path, audio: &Path) {
    if let Ok(mut export) = app.state::<PublishingState>().export.lock() {
        *export = Some(Export {
            video: video.to_path_buf(),
            audio: audio.to_path_buf(),
            text: None,
        });
    }
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum SourceMode {
    Audio,
    Transcript,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishingRequest {
    language: String,
    platforms: Vec<String>,
    #[serde(default = "default_source_mode")]
    source_mode: SourceMode,
    transcript: Option<String>,
}

fn default_source_mode() -> SourceMode {
    SourceMode::Transcript
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Post {
    pub platform: String,
    pub title: String,
    pub description: String,
    pub hashtags: Vec<String>,
    pub keywords: Vec<String>,
}

#[derive(Deserialize)]
struct Content {
    posts: Vec<Post>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishingResult {
    posts: Vec<Post>,
    text_path: String,
    text: String,
    warning: Option<String>,
}

#[derive(Serialize)]
pub struct KeyStatus {
    saved: bool,
    provider: Option<Provider>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSupport {
    supported: bool,
    provider: Option<Provider>,
}

impl KeyStatus {
    fn from_connection(connection: Option<&Connection>) -> Self {
        Self {
            saved: connection.is_some(),
            provider: connection.map(|c| c.provider),
        }
    }
}

fn credential() -> Result<keyring::Entry, String> {
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Err("Lưu API key an toàn hiện chỉ hỗ trợ macOS và Windows.".into());
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    // Retain the historical entry name to read existing verified XPiki keys.
    keyring::Entry::new("com.tinycorn.wavely", "xpiki-api-key")
        .map_err(|_| "Không truy cập được kho API key của hệ điều hành.".into())
}

fn read_connection() -> Result<Option<Connection>, String> {
    match credential()?.get_password() {
        Ok(stored) => Connection::decode(&stored).map(Some),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(
            "Không đọc được API key. Hãy cho phép Wavely truy cập Keychain/Credential Manager."
                .into(),
        ),
    }
}

fn client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Không khởi tạo được kết nối API.".into())
}

fn network_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "Dịch vụ AI phản hồi quá lâu. Hãy thử lại; video vẫn được giữ nguyên.".into()
    } else {
        "Không kết nối được dịch vụ AI. Kiểm tra Internet rồi thử lại.".into()
    }
}

fn checked(response: Response) -> Result<Response, String> {
    if response.status().is_success() {
        return Ok(response);
    }
    // Do not expose service response bodies, headers, or keys to the UI/logs.
    Err(match response.status().as_u16() {
        400 => "Dịch vụ AI từ chối yêu cầu. Kiểm tra model và nội dung gửi lên.",
        401 | 403 => "API key không hợp lệ hoặc không có quyền truy cập model.",
        404 => "Không tìm thấy model hoặc endpoint được yêu cầu.",
        429 => "API đã hết hạn mức hoặc đang giới hạn lượt gọi. Kiểm tra quota rồi thử lại.",
        500..=599 => "Dịch vụ AI đang gặp lỗi máy chủ. Hãy thử lại sau.",
        _ => "Dịch vụ AI không xử lý được yêu cầu. Hãy thử lại sau.",
    }
    .into())
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| "Tác vụ tạo nội dung bị gián đoạn.".to_string())?
}

#[tauri::command]
pub async fn publishing_key_status() -> Result<KeyStatus, String> {
    blocking(|| Ok(KeyStatus::from_connection(read_connection()?.as_ref()))).await
}

#[tauri::command]
pub async fn publishing_audio_support() -> Result<AudioSupport, String> {
    blocking(|| {
        let Some(connection) = read_connection()? else {
            return Ok(AudioSupport {
                supported: false,
                provider: None,
            });
        };
        Ok(AudioSupport {
            supported: provider::audio_support(&client()?, &connection)?,
            provider: Some(connection.provider),
        })
    })
    .await
}

fn supports_publishing(model: &Value) -> bool {
    model["endpoint_families"]
        .as_array()
        .is_some_and(|families| {
            families
                .iter()
                .any(|item| item.as_str() == Some("chat_completions"))
        })
        && model["input_modalities"]
            .as_array()
            .is_some_and(|modalities| modalities.iter().any(|item| item.as_str() == Some("text")))
        && model["capabilities"]["supports_json_output"] == true
}

#[tauri::command]
pub async fn save_publishing_key(api_key: String) -> Result<KeyStatus, String> {
    blocking(move || {
        let connection = Connection::new(api_key.trim().into())?;
        provider::verify(&client()?, &connection)?;
        // Replace the stored key only after successful verification. The provider
        // and key are serialized together so they cannot drift apart.
        credential()?
            .set_password(&connection.encode()?)
            .map_err(|_| "Không lưu được API key vào kho bảo mật hệ điều hành.".to_string())?;
        Ok(KeyStatus::from_connection(Some(&connection)))
    })
    .await
}

#[tauri::command]
pub async fn delete_publishing_key() -> Result<(), String> {
    blocking(|| match credential()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("Không xóa được API key khỏi kho bảo mật hệ điều hành.".into()),
    })
    .await
}

impl PublishingRequest {
    fn validate(&self) -> Result<(), String> {
        if !["en", "vi"].contains(&self.language.as_str()) {
            return Err("Ngôn ngữ chưa được hỗ trợ.".into());
        }
        let unique: HashSet<_> = self.platforms.iter().collect();
        if self.platforms.is_empty()
            || self.platforms.len() > 4
            || unique.len() != self.platforms.len()
            || self
                .platforms
                .iter()
                .any(|p| !PLATFORMS.contains(&p.as_str()))
        {
            return Err("Hãy chọn ít nhất một nền tảng hợp lệ.".into());
        }
        if self.source_mode == SourceMode::Transcript {
            let transcript = self.transcript.as_deref().unwrap_or_default();
            if transcript.trim().is_empty() {
                return Err("Hãy dán transcript hoặc tóm tắt nội dung âm thanh.".into());
            }
            if transcript.chars().count() > 40000 {
                return Err("Transcript hoặc tóm tắt tối đa 40.000 ký tự.".into());
            }
        }
        Ok(())
    }
}

fn progress(app: &AppHandle, message: &str) {
    let _ = app.emit("publishing-progress", message);
}

struct Running(Arc<AtomicBool>);
impl Drop for Running {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn request_body(request: &PublishingRequest) -> Value {
    let language = if request.language == "en" {
        "English"
    } else {
        "Vietnamese"
    };
    json!({
        "model": MODEL,
        "messages":[
            {"role":"system","content":include_str!("publishing_prompt.txt")},
            {"role":"user","content":json!({
                "language": language,
                "platforms": request.platforms,
                "source_mode": match request.source_mode {
                    SourceMode::Audio => "audio",
                    SourceMode::Transcript => "transcript",
                },
                "source_text": request.transcript.as_deref().unwrap_or_default(),
            }).to_string()}
        ],
        "response_format":{"type":"json_object"},
        "max_tokens":8192,
        "stream":false
    })
}

fn parse_content(value: Value, request: &PublishingRequest) -> Result<Vec<Post>, String> {
    let candidate = value["choices"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or("Dịch vụ AI không trả về nội dung. Hãy thử lại sau.")?;
    if candidate["finish_reason"].as_str() != Some("stop") {
        return Err("Dịch vụ AI chưa tạo được nội dung hoàn chỉnh. Hãy thử lại.".into());
    }
    let content = &candidate["message"]["content"];
    let text = if let Some(text) = content.as_str() {
        text.to_string()
    } else if let Some(parts) = content.as_array() {
        parts
            .iter()
            .filter_map(|part| part["text"].as_str())
            .collect::<String>()
    } else {
        return Err("Dịch vụ AI trả về nội dung không hợp lệ.".into());
    };
    let mut content: Content = serde_json::from_str(&text)
        .map_err(|_| "Nội dung do dịch vụ AI tạo không đúng định dạng JSON. Hãy thử lại.")?;
    if content.posts.len() != request.platforms.len() {
        return Err("Dịch vụ AI chưa tạo đủ các nền tảng đã chọn.".into());
    }
    let mut ordered = Vec::new();
    for platform in &request.platforms {
        let index = content
            .posts
            .iter()
            .position(|p| &p.platform == platform)
            .ok_or("Dịch vụ AI trả về nền tảng không đúng yêu cầu.")?;
        let mut post = content.posts.remove(index);
        post.title = post.title.trim().to_string();
        post.description = post.description.trim().to_string();
        let title_limit = if platform == "youtube" { 100 } else { 150 };
        let description_limit = match platform.as_str() {
            "youtube" => 4500,
            "facebook" => 4000,
            _ => 1800,
        };
        if post.title.is_empty()
            || post.title.chars().count() > title_limit
            || post.description.is_empty()
            || post.description.chars().count() > description_limit
        {
            return Err(
                "Nội dung do dịch vụ AI tạo thiếu trường hoặc vượt giới hạn nền tảng. Hãy thử lại.".into(),
            );
        }
        let mut seen_tags = HashSet::new();
        for tag in &mut post.hashtags {
            *tag = format!(
                "#{}",
                tag.trim()
                    .trim_start_matches('#')
                    .split_whitespace()
                    .collect::<String>()
            );
            if tag == "#" || tag.chars().count() > 100 {
                return Err("Hashtag do dịch vụ AI tạo không hợp lệ.".into());
            }
        }
        post.hashtags
            .retain(|tag| seen_tags.insert(tag.to_lowercase()));
        let mut seen_keywords = HashSet::new();
        for keyword in &mut post.keywords {
            *keyword = keyword.split_whitespace().collect::<Vec<_>>().join(" ");
            if keyword.is_empty() || keyword.chars().count() > 200 {
                return Err("Từ khóa do dịch vụ AI tạo không hợp lệ.".into());
            }
        }
        post.keywords
            .retain(|keyword| seen_keywords.insert(keyword.to_lowercase()));
        if post.hashtags.len() > 5 || post.keywords.len() > 10 {
            return Err("Nội dung do dịch vụ AI tạo có quá nhiều hashtag hoặc từ khóa. Hãy thử lại.".into());
        }
        let caption_length = post.description.chars().count()
            + if post.hashtags.is_empty() {
                0
            } else {
                1 + post.hashtags.join(" ").chars().count()
            };
        if caption_length > description_limit {
            return Err("Mô tả kèm hashtag vượt giới hạn cấu hình. Hãy thử lại.".into());
        }
        ordered.push(post);
    }
    Ok(ordered)
}

fn format_text(posts: &[Post], language: &str) -> String {
    let mut text = format!(
        "WAVELY — PUBLISHING CONTENT\nLanguage: {}\n\n",
        if language == "en" {
            "English"
        } else {
            "Tiếng Việt"
        }
    );
    for post in posts {
        text.push_str(&format!(
            "{}\n{}\n\nTITLE\n{}\n\nDESCRIPTION\n{}\n\nHASHTAGS\n{}\n\nKEYWORDS\n{}\n\n",
            post.platform.to_uppercase(),
            "=".repeat(40),
            post.title,
            post.description,
            post.hashtags.join(" "),
            post.keywords.join(", ")
        ));
    }
    text
}

fn save_sidecar(video: &Path, text: &str) -> Result<PathBuf, String> {
    let stem = video
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Tên video không hợp lệ.")?;
    for index in 1..1000 {
        let suffix = if index == 1 {
            String::new()
        } else {
            format!("-{index}")
        };
        let path = video.with_file_name(format!("{stem}-noi-dung{suffix}.txt"));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if file
                    .write_all(text.as_bytes())
                    .and_then(|_| file.sync_all())
                    .is_err()
                {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(
                        "Không lưu được file nội dung. Kiểm tra quyền ghi và dung lượng ổ đĩa."
                            .into(),
                    );
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => {
                return Err(
                    "Không tạo được file nội dung cạnh video. Kiểm tra quyền ghi thư mục.".into(),
                )
            }
        }
    }
    Err("Có quá nhiều file nội dung cùng tên trong thư mục.".into())
}

fn generate(
    app: AppHandle,
    export: Export,
    request: PublishingRequest,
) -> Result<PublishingResult, String> {
    request.validate()?;
    let connection = read_connection()?.ok_or("Hãy lưu API key Google Gemini hoặc XPiki trước khi tạo nội dung.")?;
    if !export.video.is_file() {
        return Err("Không còn tìm thấy video vừa xuất.".into());
    }
    progress(&app, "Đang chuẩn bị nội dung đăng bài…");
    let client = client()?;
    let prepared_audio = if request.source_mode == SourceMode::Audio {
        if !export.audio.is_file() {
            return Err("Không còn tìm thấy file âm thanh vừa dùng để xuất video.".into());
        }
        progress(&app, "Đang chuẩn bị file âm thanh cho dịch vụ AI…");
        Some(provider::prepare_audio(&client, &connection, &export.audio)?)
    } else {
        None
    };
    progress(&app, "Đang tạo nội dung đăng bài…");
    let generated = (|| {
        let response = provider::generation_request(
            &client,
            &connection,
            &request_body(&request),
            prepared_audio.as_ref(),
        )
        .send()
        .map_err(network_error)?;
        if connection.provider == Provider::Xpiki
            && prepared_audio.is_some()
            && response.status().as_u16() == 400
        {
            return Err("XPiki báo model có audio nhưng endpoint từ chối định dạng audio. Hãy chuyển sang nhập transcript hoặc dùng model audio tương thích.".into());
        }
        checked(response)?
            .json()
            .map_err(|_| "Không đọc được phản hồi từ dịch vụ AI.".to_string())
    })();
    if let Some(audio) = prepared_audio.as_ref() {
        provider::cleanup_audio(&client, &connection, audio);
    }
    let value: Value = generated?;
    let posts = parse_content(
        provider::normalize_response(connection.provider, value)?,
        &request,
    )?;
    let text = format_text(&posts, &request.language);
    let text_path = save_sidecar(&export.video, &text)?;
    if let Ok(mut current) = app.state::<PublishingState>().export.lock() {
        if let Some(current) = current
            .as_mut()
            .filter(|current| current.video == export.video)
        {
            current.text = Some(text_path.clone());
        }
    }
    Ok(PublishingResult {
        posts,
        text_path: text_path.to_string_lossy().into_owned(),
        text,
        warning: None,
    })
}

#[tauri::command]
pub async fn generate_publishing_content(
    app: AppHandle,
    state: State<'_, PublishingState>,
    request: PublishingRequest,
) -> Result<PublishingResult, String> {
    request.validate()?;
    let export = state
        .export
        .lock()
        .map_err(|_| "Không đọc được video vừa xuất.")?
        .clone()
        .ok_or("Hãy xuất video trước khi tạo nội dung đăng bài.")?;
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("Đang có một tác vụ tạo nội dung khác.".into());
    }
    let guard = Running(state.running.clone());
    blocking(move || {
        let _guard = guard;
        generate(app, export, request)
    })
    .await
}

#[tauri::command]
pub async fn open_publishing_output(
    state: State<'_, PublishingState>,
    folder: bool,
) -> Result<(), String> {
    let export = state
        .export
        .lock()
        .map_err(|_| "Không đọc được file nội dung.")?
        .clone()
        .ok_or("Chưa có video vừa xuất.")?;
    let path = if folder {
        export.text.unwrap_or(export.video)
    } else {
        export.text.ok_or("Chưa có file nội dung.")?
    };
    if !path.is_file() {
        return Err("File đã được di chuyển hoặc xóa.".into());
    }
    blocking(move || {
        #[cfg(target_os = "macos")]
        let mut cmd = {
            let mut c = Command::new("open");
            if folder {
                c.arg("-R");
            }
            c.arg(&path);
            c
        };
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("explorer.exe");
            if folder {
                c.arg(format!("/select,{}", path.display()));
            } else {
                c.arg(&path);
            }
            c
        };
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let mut cmd = {
            let mut c = Command::new("xdg-open");
            c.arg(if folder {
                path.parent().unwrap_or(&path)
            } else {
                &path
            });
            c
        };
        cmd.spawn().map_err(|_| "Không mở được file/thư mục.")?;
        Ok(())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn request() -> PublishingRequest {
        PublishingRequest {
            language: "en".into(),
            platforms: vec!["youtube".into(), "tiktok".into()],
            source_mode: SourceMode::Transcript,
            transcript: Some("A quiet instrumental recording for relaxation.".into()),
        }
    }

    fn response(posts: Vec<Value>) -> Value {
        json!({"choices":[{"finish_reason":"stop","message":{"content":json!({"posts":posts}).to_string()}}]})
    }

    #[test]
    fn key_status_returns_provider_without_credential_material() {
        let connection = Connection::new("AIza-private-fixture".into()).unwrap();
        let status = serde_json::to_value(KeyStatus::from_connection(Some(&connection))).unwrap();
        assert_eq!(status, json!({"saved":true,"provider":"google"}));
        assert_eq!(
            serde_json::to_value(KeyStatus::from_connection(None)).unwrap(),
            json!({"saved":false,"provider":null})
        );
    }

    #[test]
    fn google_response_passes_the_shared_post_validation_and_export_format() {
        let text = json!({"posts":[valid_post("tiktok"),valid_post("youtube")]}).to_string();
        let google = json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"thought":true,"text":"not publishing copy"},{"text":text}]}}]});
        let normalized = provider::normalize_response(Provider::Google, google).unwrap();
        let posts = parse_content(normalized, &request()).unwrap();
        assert_eq!(posts[0].platform, "youtube");
        assert_eq!(posts[1].platform, "tiktok");
        let exported = format_text(&posts, "en");
        assert!(exported.contains("A moment to unwind"));
        assert!(!exported.contains("not publishing copy"));
    }

    #[test]
    fn validates_nested_xpiki_model_capabilities() {
        let model = json!({
            "endpoint_families": ["chat_completions"],
            "input_modalities": ["text"],
            "capabilities": {"supports_json_output": true}
        });
        assert!(supports_publishing(&model));
        let mut unsupported = model.clone();
        unsupported["capabilities"]["supports_json_output"] = json!(false);
        assert!(!supports_publishing(&unsupported));
        unsupported = model.clone();
        unsupported["endpoint_families"] = json!(["responses"]);
        assert!(!supports_publishing(&unsupported));
        unsupported = model.clone();
        unsupported["input_modalities"] = json!(["image"]);
        assert!(!supports_publishing(&unsupported));
        unsupported = model;
        unsupported.as_object_mut().unwrap().remove("capabilities");
        unsupported["supports_json_output"] = json!(true);
        assert!(!supports_publishing(&unsupported));
    }

    #[test]
    fn rejects_empty_or_unrecognized_platforms_before_request() {
        let mut r = request();
        r.platforms.clear();
        assert!(r.validate().is_err());
        r.platforms = vec!["unknown".into()];
        assert!(r.validate().is_err());
        r.platforms = vec!["youtube".into(), "youtube".into()];
        assert!(r.validate().is_err());
        r.platforms = vec!["youtube".into()];
        r.language = "invalid".into();
        assert!(r.validate().is_err());
    }

    #[test]
    fn orders_platforms_and_parses_chat_completion_json() {
        let posts = parse_content(
            response(vec![valid_post("tiktok"), valid_post("youtube")]),
            &request(),
        )
        .unwrap();
        assert_eq!(posts[0].platform, "youtube");
        assert_eq!(posts[1].hashtags[1], "#SlowLiving");
        let text = format_text(&posts, "en");
        assert!(text.contains("Language: English"));
        assert!(text.contains("HASHTAGS"));
    }

    #[test]
    fn accepts_optional_metadata_and_normalizes_duplicates() {
        let mut youtube = valid_post("youtube");
        youtube["hashtags"] = json!([]);
        youtube["keywords"] = json!([]);
        let mut tiktok = valid_post("tiktok");
        tiktok["hashtags"] = json!([" Quiet Moments ", "#quietmoments", "#Relax"]);
        tiktok["keywords"] = json!(["  quiet   moments ", "Quiet Moments", "relaxation"]);
        let posts = parse_content(response(vec![youtube, tiktok]), &request()).unwrap();
        assert!(posts[0].hashtags.is_empty());
        assert!(posts[0].keywords.is_empty());
        assert_eq!(posts[1].hashtags, vec!["#QuietMoments", "#Relax"]);
        assert_eq!(posts[1].keywords, vec!["quiet moments", "relaxation"]);
    }

    #[test]
    fn rejects_excess_metadata_and_combined_caption_overflow() {
        let mut post = valid_post("tiktok");
        post["hashtags"] = json!((0..6).map(|i| format!("#tag{i}")).collect::<Vec<_>>());
        assert!(parse_content(response(vec![valid_post("youtube"), post]), &request()).is_err());
        let mut post = valid_post("tiktok");
        post["keywords"] = json!((0..11).map(|i| format!("keyword {i}")).collect::<Vec<_>>());
        assert!(parse_content(response(vec![valid_post("youtube"), post]), &request()).is_err());
        let mut post = valid_post("tiktok");
        post["description"] = json!("á".repeat(1800));
        post["hashtags"] = json!([]);
        assert!(parse_content(
            response(vec![valid_post("youtube"), post.clone()]),
            &request()
        )
        .is_ok());
        post["hashtags"] = json!(["#Relax"]);
        assert!(parse_content(response(vec![valid_post("youtube"), post]), &request()).is_err());
    }

    #[test]
    fn rejects_partial_blocked_malformed_or_overlong_output() {
        assert!(parse_content(response(vec![valid_post("youtube")]), &request()).is_err());
        assert!(parse_content(
            response(vec![valid_post("youtube"), valid_post("youtube")]),
            &request()
        )
        .is_err());
        assert!(parse_content(json!({"error":{"message":"invalid request"}}), &request()).is_err());
        let mut partial = response(vec![valid_post("youtube"), valid_post("tiktok")]);
        partial["choices"][0]["finish_reason"] = json!("length");
        assert!(parse_content(partial, &request()).is_err());
        let mut long = valid_post("youtube");
        long["title"] = json!("x".repeat(101));
        assert!(parse_content(response(vec![long, valid_post("tiktok")]), &request()).is_err());
    }

    fn valid_post(platform: &str) -> Value {
        json!({"platform":platform,"title":"A moment to unwind","description":"Take a quiet moment with this recording.",
            "hashtags":["#QuietMoments","Slow Living","#Relax"],"keywords":["relaxation","quiet moments","instrumental","slow living","mindfulness"]})
    }

    #[test]
    fn sidecar_preserves_existing_files_and_unicode_names() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("wavely-sidecar-test-{nonce}"));
        fs::create_dir(&dir).unwrap();
        let video = dir.join("Bình yên.mp4");
        let existing = dir.join("Bình yên-noi-dung.txt");
        fs::write(&existing, "Keep this edited post").unwrap();
        let new_path = save_sidecar(&video, "New content ✓").unwrap();
        assert_eq!(new_path.file_name().unwrap(), "Bình yên-noi-dung-2.txt");
        assert_eq!(
            fs::read_to_string(&existing).unwrap(),
            "Keep this edited post"
        );
        assert_eq!(fs::read_to_string(new_path).unwrap(), "New content ✓");
        fs::remove_dir_all(dir).unwrap();
    }
}
