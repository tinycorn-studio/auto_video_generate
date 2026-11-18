import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { capturePublishingRequest, generatePublishing, initPublishing, publishingEnabled, resetPublishingResult, setPublishingBusy } from "./publishing";
import type { PublishingRequest } from "./publishing";
import "./style.css";

type VisualizerStyle = "studio" | "eq" | "blocks";
type OutputFormat = "portrait" | "landscape";
type WavePosition = "top-left" | "top-center" | "top-right" | "center-left" | "center" | "center-right" | "bottom-left" | "bottom-center" | "bottom-right";
type BackgroundKind = "image" | "video";
type DurationMode = "fit-audio" | "fit-video";
type ShortVideoMode = "loop" | "freeze" | "stretch";
type PerformanceMode = "auto" | "cpu-fast" | "cpu-compact";

type MediaInfo = {
  duration: number;
  width: number | null;
  height: number | null;
  fps: number | null;
  hasVideo: boolean;
  hasAudio: boolean;
  sarIsOne: boolean;
  hasRotation: boolean;
};

type RenderCapabilities = {
  hardwareEncoder: string | null;
  hardwareLabel: string | null;
  recommendedLabel: string;
};

type EngineStatus = { available: boolean; version: string | null; message: string };
type EngineInstallEvent = { status: "downloading" | "extracting" | "ready" | "cancelled" | "error"; percent: number; message: string };
type RenderEvent = {
  jobId: string;
  status: "running" | "completed" | "cancelled" | "error";
  percent: number;
  message: string;
  outputPath?: string;
};

const PERF_STORAGE_KEY = "wavely.performanceMode";

const byId = <T extends HTMLElement>(id: string) => {
  const element = document.getElementById(id);
  if (!element) throw new Error(`Missing element #${id}`);
  return element as T;
};

const mediaPicker = byId<HTMLButtonElement>("mediaPicker");
const audioPicker = byId<HTMLButtonElement>("audioPicker");
const renderButton = byId<HTMLButtonElement>("renderButton");
const cancelButton = byId<HTMLButtonElement>("cancelButton");
const audioElement = byId<HTMLAudioElement>("audioElement");
const selectedImage = byId<HTMLImageElement>("selectedImage");
const selectedVideo = byId<HTMLVideoElement>("selectedVideo");
const engineStatus = byId<HTMLDivElement>("engineStatus");
const engineSetup = byId<HTMLElement>("engineSetup");
const engineSetupText = byId<HTMLParagraphElement>("engineSetupText");
const installEngineButton = byId<HTMLButtonElement>("installEngineButton");
const cancelEngineButton = byId<HTMLButtonElement>("cancelEngineButton");
const engineProgress = byId<HTMLDivElement>("engineProgress");
const renderProgress = byId<HTMLDivElement>("renderProgress");
const resultMessage = byId<HTMLParagraphElement>("resultMessage");
const renderLabel = byId<HTMLSpanElement>("renderLabel");
const previewPlaceholder = byId<HTMLDivElement>("previewPlaceholder");
const previewEmptyLabel = byId<HTMLDivElement>("previewEmptyLabel");
const videoFrame = byId<HTMLDivElement>("videoFrame");
const startDelayInput = byId<HTMLInputElement>("startDelay");
const endDelayInput = byId<HTMLInputElement>("endDelay");
const keepVideoAudioInput = byId<HTMLInputElement>("keepVideoAudio");

const state = {
  backgroundKind: "image" as BackgroundKind,
  imagePath: "",
  imageMeta: "",
  videoPath: "",
  videoInfo: null as MediaInfo | null,
  videoError: "",
  probingVideo: false,
  audioPath: "",
  audioDurationSec: 0,
  style: "studio" as VisualizerStyle,
  outputFormat: "portrait" as OutputFormat,
  wavePosition: "bottom-center" as WavePosition,
  waveScale: 100,
  durationMode: "fit-audio" as DurationMode,
  shortVideoMode: "loop" as ShortVideoMode,
  startDelaySec: 0,
  endDelaySec: 0,
  keepVideoAudio: false,
  performanceMode: "auto" as PerformanceMode,
  capabilities: null as RenderCapabilities | null,
  engine: null as EngineStatus | null,
  jobId: "",
  busy: false,
  installing: false,
};

let pendingPublishing: PublishingRequest | null = null;
let startingRender = false;
let earlyRenderEvent: RenderEvent | null = null;

function humanDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "—";
  const total = Math.floor(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const remainder = total % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}`
    : `${minutes}:${String(remainder).padStart(2, "0")}`;
}

function formatSec(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0s";
  if (seconds >= 60) return `${humanDuration(seconds)} (${seconds.toFixed(1)}s)`;
  return `${Number.isInteger(seconds) ? seconds.toFixed(0) : seconds.toFixed(1)}s`;
}

function stem(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? path;
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

function extUpper(path: string, fallback: string): string {
  return path.split(".").pop()?.toUpperCase() ?? fallback;
}

function activeBackgroundPath(): string {
  return state.backgroundKind === "video" ? state.videoPath : state.imagePath;
}

function hasInvalidFitVideoDelay(): boolean {
  return (
    state.backgroundKind === "video" &&
    state.durationMode === "fit-video" &&
    state.videoInfo !== null &&
    state.startDelaySec >= state.videoInfo.duration
  );
}

function isStretchingShortVideo(): boolean {
  if (state.backgroundKind !== "video" || state.durationMode !== "fit-audio" || state.shortVideoMode !== "stretch") {
    return false;
  }
  const speechSpan = state.startDelaySec + state.audioDurationSec + state.endDelaySec;
  return state.videoInfo !== null && state.audioDurationSec > 0 && state.videoInfo.duration + 0.02 < speechSpan;
}

function activeEncoderLabel(): string {
  if (state.performanceMode === "cpu-fast") return "CPU siêu nhanh";
  if (state.performanceMode === "cpu-compact") return "CPU nén gọn";
  return state.capabilities?.recommendedLabel ?? "Tự động";
}

function updateOutputSummaryText(): void {
  const landscape = state.outputFormat === "landscape";
  const dimensions = landscape ? "1920 × 1080" : "1080 × 1920";
  byId("outputDimensions").textContent = `MP4 · ${dimensions} · 30 fps · H.264 (${activeEncoderLabel()})`;
}

function updateKeepVideoAudioControl(): void {
  const row = byId("keepVideoAudioRow");
  const label = byId("keepVideoAudioLabel");
  row.hidden = state.backgroundKind !== "video";
  if (state.backgroundKind !== "video") return;

  const noAudioTrack = state.videoInfo !== null && !state.videoInfo.hasAudio;
  const stretched = isStretchingShortVideo();
  const disabled = state.busy || !state.videoPath || noAudioTrack || stretched;

  keepVideoAudioInput.disabled = disabled;
  row.classList.toggle("is-disabled", disabled && !state.busy);

  if (noAudioTrack) {
    keepVideoAudioInput.checked = false;
    state.keepVideoAudio = false;
    label.textContent = "Video nền không có track âm thanh gốc";
  } else if (stretched) {
    label.textContent = "Tắt tiếng gốc video khi đang dùng chế độ kéo chậm";
  } else {
    keepVideoAudioInput.checked = state.keepVideoAudio;
    label.textContent = "Giữ tiếng gốc của video nền (trộn nhẹ dưới giọng nói)";
  }
}

function updateTimelineSummary(): void {
  const summary = byId("timelineSummary");
  const audioDur = state.audioDurationSec;
  const start = state.startDelaySec;
  const end = state.endDelaySec;
  const speechSpan = start + audioDur + end;

  if (state.backgroundKind === "image") {
    if (!audioDur) {
      if (start > 0 || end > 0) {
        summary.hidden = false;
        summary.className = "timeline-summary";
        summary.textContent = `Delay đầu ${formatSec(start)} · Delay cuối ${formatSec(end)}. Chọn file âm thanh để xem tổng thời lượng.`;
      } else {
        summary.hidden = true;
      }
      return;
    }
    summary.hidden = false;
    summary.className = "timeline-summary is-ready";
    summary.textContent = `Tổng thời lượng xuất: ${formatSec(speechSpan)} (Chờ ${formatSec(start)} · Ghi âm ${formatSec(audioDur)} · Kết ${formatSec(end)}).`;
    return;
  }

  // Video background
  if (state.probingVideo) {
    summary.hidden = false;
    summary.className = "timeline-summary";
    summary.textContent = "Đang đọc thông số video nền…";
    return;
  }
  if (state.videoError) {
    summary.hidden = false;
    summary.className = "timeline-summary is-warning";
    summary.textContent = state.videoError;
    return;
  }
  if (!state.videoInfo && !audioDur) {
    summary.hidden = true;
    return;
  }
  if (!state.videoInfo) {
    summary.hidden = false;
    summary.className = "timeline-summary";
    summary.textContent = `Ghi âm + delay hiện cần ${formatSec(speechSpan)}. Hãy chọn video nền.`;
    return;
  }

  const videoDur = state.videoInfo.duration;
  if (!audioDur) {
    summary.hidden = false;
    summary.className = "timeline-summary";
    summary.textContent =
      state.durationMode === "fit-video"
        ? `Sẽ giữ nguyên độ dài video nền (${formatSec(videoDur)}). Chọn file ghi âm để kiểm tra khớp giờ.`
        : `Video nền dài ${formatSec(videoDur)}. Chọn file ghi âm để tính thời lượng ghép.`;
    return;
  }

  if (state.durationMode === "fit-audio") {
    summary.hidden = false;
    summary.className = "timeline-summary is-ready";
    let detail = "";
    if (Math.abs(videoDur - speechSpan) < 0.05) {
      detail = "khớp sẵn độ dài video";
    } else if (videoDur >= speechSpan - 0.02) {
      detail = `cắt video từ ${formatSec(videoDur)}`;
    } else if (state.shortVideoMode === "loop") {
      detail = `lặp lại video ${formatSec(videoDur)} cho đủ`;
    } else if (state.shortVideoMode === "freeze") {
      detail = `giữ khung cuối thêm ${formatSec(speechSpan - videoDur)}`;
    } else {
      const ratio = speechSpan / videoDur;
      detail = `kéo chậm video ${ratio.toFixed(2)}×`;
    }
    summary.textContent = `Tổng xuất: ${formatSec(speechSpan)} (${detail}; chờ ${formatSec(start)} · ghi âm ${formatSec(audioDur)} · kết ${formatSec(end)}).`;
    return;
  }

  // Fit-video mode
  summary.hidden = false;
  if (start >= videoDur) {
    summary.className = "timeline-summary is-warning";
    summary.textContent = `Delay đầu (${formatSec(start)}) đang lớn hơn hoặc bằng video (${formatSec(videoDur)}). Hãy giảm delay đầu.`;
  } else if (start + audioDur > videoDur) {
    summary.className = "timeline-summary is-warning";
    summary.textContent = `Tổng xuất: ${formatSec(videoDur)} (theo video). Ghi âm + delay đầu (${formatSec(start + audioDur)}) dài hơn video nên phần cuối bản thu sẽ bị cắt.`;
  } else if (speechSpan > videoDur + 0.05) {
    const actualTail = Math.max(0, videoDur - start - audioDur);
    summary.className = "timeline-summary";
    summary.textContent = `Tổng xuất: ${formatSec(videoDur)} (theo video). Giọng nói phát từ ${formatSec(start)} → ${formatSec(start + audioDur)}; khoảng lặng cuối thực tế còn ${formatSec(actualTail)}.`;
  } else {
    summary.className = "timeline-summary is-ready";
    summary.textContent = `Tổng xuất: ${formatSec(videoDur)} (theo video). Giọng nói phát từ ${formatSec(start)} → ${formatSec(start + audioDur)}, dư ${formatSec(videoDur - start - audioDur)} ở cuối.`;
  }
}

function updateBackgroundUI(): void {
  const isVideo = state.backgroundKind === "video";
  byId<HTMLInputElement>("bgKindImage").checked = !isVideo;
  byId<HTMLInputElement>("bgKindVideo").checked = isVideo;
  byId("videoTimingControls").hidden = !isVideo;
  byId("shortVideoGroup").hidden = !isVideo || state.durationMode !== "fit-audio";

  const aspect = state.outputFormat === "landscape" ? "16:9" : "9:16";
  byId("imageAspectHint").textContent = isVideo
    ? `Video tự căn giữa và cắt theo ${aspect}`
    : `Ảnh tự căn giữa và cắt theo ${aspect}`;
  byId("mediaIcon").innerHTML = isVideo
    ? `<svg viewBox="0 0 24 24" fill="none"><rect x="3" y="5.5" width="13.5" height="13" rx="2.3" stroke="currentColor" stroke-width="1.7"/><path d="m16.5 10 4.5-2.7v9.4L16.5 14" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>`
    : `<svg viewBox="0 0 24 24" fill="none"><rect x="3.5" y="3" width="17" height="18" rx="2.5" stroke="currentColor" stroke-width="1.7"/><circle cx="15.5" cy="8" r="1.6" fill="currentColor"/><path d="m5 17 4.7-4.7 3.1 3.1 2.3-2.3 4 4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

  if (isVideo) {
    byId("mediaAction").textContent = state.videoPath ? "Đổi video" : "Chọn video";
    if (state.videoPath) {
      byId("mediaName").textContent = stem(state.videoPath);
      if (state.probingVideo) {
        byId("mediaDetails").textContent = "Đang đọc thông số video…";
      } else if (state.videoError) {
        byId("mediaDetails").textContent = state.videoError;
      } else if (state.videoInfo) {
        const res = state.videoInfo.width && state.videoInfo.height ? `${state.videoInfo.width} × ${state.videoInfo.height}` : "Video";
        const fps = state.videoInfo.fps ? `${Math.round(state.videoInfo.fps)} fps` : "";
        const audioTag = state.videoInfo.hasAudio ? "có tiếng gốc" : "không tiếng";
        byId("mediaDetails").textContent = [res, formatSec(state.videoInfo.duration), fps, extUpper(state.videoPath, "MP4"), audioTag]
          .filter(Boolean)
          .join(" · ");
      } else {
        byId("mediaDetails").textContent = extUpper(state.videoPath, "VIDEO");
      }
    } else {
      byId("mediaName").textContent = "Chọn video nền";
      byId("mediaDetails").textContent = "MP4, MOV, MKV, WebM hoặc M4V";
    }

    selectedImage.hidden = true;
    if (state.videoPath) {
      selectedVideo.hidden = false;
      previewPlaceholder.hidden = true;
      byId("videoCaption").hidden = true;
      previewEmptyLabel.hidden = true;
      if (selectedVideo.dataset.loadedPath !== state.videoPath) {
        selectedVideo.dataset.loadedPath = state.videoPath;
        selectedVideo.src = convertFileSrc(state.videoPath);
        selectedVideo.load();
      }
      void selectedVideo.play().catch(() => {});
    } else {
      selectedVideo.pause();
      selectedVideo.hidden = true;
      previewPlaceholder.hidden = false;
      byId("videoCaption").hidden = false;
      previewEmptyLabel.hidden = false;
      previewEmptyLabel.textContent = "Ảnh hoặc video xem trước sẽ xuất hiện ở đây";
    }
  } else {
    selectedVideo.pause();
    selectedVideo.hidden = true;
    byId("mediaAction").textContent = state.imagePath ? "Đổi ảnh" : "Chọn ảnh";
    if (state.imagePath) {
      byId("mediaName").textContent = stem(state.imagePath);
      byId("mediaDetails").textContent = state.imageMeta || extUpper(state.imagePath, "Ảnh");
      selectedImage.hidden = false;
      previewPlaceholder.hidden = true;
      previewEmptyLabel.hidden = true;
      byId("videoCaption").hidden = true;
    } else {
      byId("mediaName").textContent = "Chọn ảnh nền";
      byId("mediaDetails").textContent = "JPG, PNG, WebP hoặc BMP";
      selectedImage.hidden = true;
      previewPlaceholder.hidden = false;
      previewEmptyLabel.hidden = false;
      previewEmptyLabel.textContent = "Ảnh hoặc video xem trước sẽ xuất hiện ở đây";
      byId("videoCaption").hidden = false;
    }
  }

  updateKeepVideoAudioControl();
  updateTimelineSummary();
  updateRenderButton();
}

function setEngine(status: EngineStatus): void {
  state.engine = status;
  engineStatus.classList.toggle("is-ready", status.available);
  engineStatus.classList.toggle("is-error", !status.available);
  const dot = document.createElement("span");
  dot.className = "status-dot";
  const label = document.createElement("span");
  label.textContent = status.available
    ? `FFmpeg sẵn sàng${status.version ? ` · ${status.version}` : ""}`
    : "Cần cài FFmpeg để xuất video";
  engineStatus.replaceChildren(dot, label);
  engineStatus.title = status.message;
  if (!status.available) {
    engineSetupText.textContent = status.message.includes("Chưa tìm thấy")
      ? "Tải FFmpeg một lần vào thư mục riêng của Wavely. Video được xử lý trên máy; tạo nội dung AI là tùy chọn và cần Internet."
      : `${status.message} Bạn có thể cài engine tương thích ngay trong Wavely.`;
    engineSetup.hidden = false;
    if (!state.installing && !resultMessage.classList.contains("is-error")) resultMessage.textContent = "";
  } else {
    engineSetup.hidden = true;
  }
  installEngineButton.disabled = state.installing;
  cancelEngineButton.hidden = !state.installing;
  engineProgress.hidden = !state.installing;
  updateRenderButton();
}

function updateRenderButton(): void {
  const backgroundReady =
    state.backgroundKind === "image"
      ? Boolean(state.imagePath)
      : Boolean(state.videoPath) && !state.probingVideo && state.videoInfo !== null && !state.videoError;
  renderButton.disabled =
    state.busy ||
    !backgroundReady ||
    !state.audioPath ||
    !state.engine?.available ||
    hasInvalidFitVideoDelay();

  mediaPicker.disabled = state.busy;
  audioPicker.disabled = state.busy;
  startDelayInput.disabled = state.busy;
  endDelayInput.disabled = state.busy;
  document
    .querySelectorAll<HTMLButtonElement>(".preset-card, .choice-card, .position-card")
    .forEach((button) => (button.disabled = state.busy));
  document
    .querySelectorAll<HTMLInputElement>(
      'input[name="backgroundKind"], input[name="shortVideoMode"], input[name="performanceMode"]'
    )
    .forEach((input) => (input.disabled = state.busy));
  byId<HTMLInputElement>("waveScale").disabled = state.busy;
  updateKeepVideoAudioControl();
  if (!state.busy) renderLabel.textContent = publishingEnabled() ? "Tạo video & nội dung đăng bài" : "Tạo video";
}

function showError(message: string): void {
  resultMessage.textContent = message.replace(/^Error: /, "");
  resultMessage.className = "result-message is-error";
}

async function refreshCapabilities(): Promise<void> {
  if (!state.engine?.available) {
    byId("encoderHint").textContent = "Chờ cài FFmpeg để dò GPU";
    updateOutputSummaryText();
    return;
  }
  try {
    const caps = await invoke<RenderCapabilities>("render_capabilities");
    state.capabilities = caps;
    byId("encoderHint").textContent = caps.hardwareLabel
      ? `Phát hiện ${caps.hardwareLabel}`
      : "Dùng CPU tối ưu đa luồng";
  } catch {
    state.capabilities = null;
    byId("encoderHint").textContent = "Tự động theo máy";
  }
  updateOutputSummaryText();
}

async function refreshEngineStatus(): Promise<void> {
  setEngine(await invoke<EngineStatus>("ffmpeg_status"));
  await refreshCapabilities();
  if (state.engine?.available && state.videoPath && !state.videoInfo && !state.probingVideo) {
    await probeSelectedVideo(state.videoPath);
  }
  if (state.engine?.available && state.audioPath && state.audioDurationSec <= 0) {
    await probeSelectedAudio(state.audioPath);
  }
}

async function startEngineInstall(): Promise<void> {
  if (state.installing || state.engine?.available) return;
  resultMessage.textContent = "";
  resultMessage.className = "result-message";
  state.installing = true;
  installEngineButton.disabled = true;
  cancelEngineButton.disabled = false;
  cancelEngineButton.hidden = false;
  engineProgress.hidden = false;
  byId("engineProgressMessage").textContent = "Đang kết nối tới nguồn FFmpeg…";
  byId("engineProgressPercent").textContent = "0%";
  byId("engineProgressFill").style.width = "0%";
  try {
    await invoke("install_ffmpeg");
  } catch (error) {
    state.installing = false;
    installEngineButton.disabled = false;
    cancelEngineButton.hidden = true;
    engineProgress.hidden = true;
    showError(String(error));
  }
}

async function cancelEngineInstall(): Promise<void> {
  if (!state.installing) return;
  cancelEngineButton.disabled = true;
  byId("engineProgressMessage").textContent = "Đang hủy tải và dọn file tạm…";
  try {
    await invoke("cancel_ffmpeg_install");
  } catch (error) {
    cancelEngineButton.disabled = false;
    showError(String(error));
  }
}

async function handleEngineInstallEvent(event: EngineInstallEvent): Promise<void> {
  const percent = Math.max(0, Math.min(100, Math.round(event.percent)));
  byId("engineProgressPercent").textContent = `${percent}%`;
  byId("engineProgressFill").style.width = `${percent}%`;
  byId("engineProgressMessage").textContent = event.message;
  if (event.status === "ready") {
    state.installing = false;
    cancelEngineButton.disabled = false;
    await refreshEngineStatus();
    resultMessage.textContent = event.message;
    resultMessage.className = "result-message is-success";
  } else if (event.status === "error" || event.status === "cancelled") {
    state.installing = false;
    cancelEngineButton.disabled = false;
    engineProgress.hidden = true;
    if (event.status === "error") showError(event.message);
    else {
      resultMessage.textContent = event.message;
      resultMessage.className = "result-message";
    }
    await refreshEngineStatus();
  }
}

function setBusy(busy: boolean): void {
  state.busy = busy;
  cancelButton.hidden = !busy;
  renderProgress.hidden = !busy;
  renderLabel.textContent = busy ? "Đang xuất video…" : "Tạo video";
  cancelButton.disabled = false;
  setPublishingBusy(busy);
  updateRenderButton();
}

async function probeSelectedVideo(selection: string): Promise<void> {
  state.videoInfo = null;
  state.videoError = "";
  state.probingVideo = true;
  updateBackgroundUI();

  if (!state.engine?.available) {
    state.probingVideo = false;
    state.videoError = "Cần cài FFmpeg để đọc thông số video.";
    updateBackgroundUI();
    return;
  }

  resultMessage.textContent = "";
  try {
    const info = await invoke<MediaInfo>("probe_media_info", { path: selection });
    if (state.videoPath === selection) {
      if (!info.hasVideo) {
        state.videoError = "File đã chọn không có hình ảnh video hợp lệ.";
        state.videoInfo = null;
      } else {
        state.videoInfo = info;
      }
    }
  } catch (error) {
    if (state.videoPath === selection) {
      state.videoError = String(error).replace(/^Error: /, "");
    }
  } finally {
    if (state.videoPath === selection) {
      state.probingVideo = false;
      updateBackgroundUI();
    }
  }
}

async function probeSelectedAudio(selection: string): Promise<void> {
  if (!state.engine?.available) return;
  try {
    const info = await invoke<MediaInfo>("probe_media_info", { path: selection });
    if (state.audioPath === selection && info.duration > 0) {
      state.audioDurationSec = info.duration;
      byId("audioDuration").textContent = `${humanDuration(info.duration)} · ${extUpper(selection, "Audio")}`;
      updateTimelineSummary();
      updateKeepVideoAudioControl();
      updateRenderButton();
    }
  } catch {
    // Fallback remains handled by audioElement metadata or start_render validation.
  }
}

async function pickMedia(): Promise<void> {
  if (state.busy) return;
  if (state.backgroundKind === "video") {
    const selection = await open({
      multiple: false,
      title: "Chọn video nền",
      filters: [{ name: "Video", extensions: ["mp4", "mov", "mkv", "webm", "m4v", "avi"] }],
    });
    if (typeof selection !== "string") return;
    state.videoPath = selection;
    await probeSelectedVideo(selection);
    return;
  }

  const selection = await open({
    multiple: false,
    title: "Chọn ảnh nền",
    filters: [{ name: "Ảnh", extensions: ["jpg", "jpeg", "png", "webp", "bmp"] }],
  });
  if (typeof selection !== "string") return;
  state.imagePath = selection;
  state.imageMeta = extUpper(selection, "Ảnh đã chọn");
  selectedImage.onload = () => {
    state.imageMeta = `${selectedImage.naturalWidth} × ${selectedImage.naturalHeight} · ${extUpper(selection, "Ảnh")}`;
    if (state.backgroundKind === "image" && state.imagePath === selection) {
      byId("mediaDetails").textContent = state.imageMeta;
    }
  };
  selectedImage.onerror = () => showError("Không thể xem trước ảnh này. Bạn vẫn có thể thử xuất video.");
  selectedImage.src = convertFileSrc(selection);
  if (state.engine?.available) resultMessage.textContent = "";
  updateBackgroundUI();
}

async function pickAudio(): Promise<void> {
  if (state.busy) return;
  const selection = await open({
    multiple: false,
    title: "Chọn file ghi âm",
    filters: [{ name: "Âm thanh", extensions: ["wav", "mp3", "m4a", "aac", "flac", "ogg", "opus"] }],
  });
  if (typeof selection !== "string") return;
  state.audioPath = selection;
  state.audioDurationSec = 0;
  resetPublishingResult();
  const name = stem(selection);
  byId("audioName").textContent = name;
  byId("publishingAudioFile").textContent = `${name}.${extUpper(selection, "AUDIO")} · Chỉ gửi nếu model hỗ trợ audio`;
  byId("audioTrackName").textContent = name;
  byId("audioAction").textContent = "Đổi file";
  byId("audioPreview").hidden = false;
  byId("audioDuration").textContent = "Đang đọc thời lượng…";

  audioElement.onloadedmetadata = () => {
    if (state.audioPath === selection && Number.isFinite(audioElement.duration) && audioElement.duration > 0) {
      if (state.audioDurationSec <= 0) state.audioDurationSec = audioElement.duration;
      byId("audioDuration").textContent = `${humanDuration(state.audioDurationSec)} · ${extUpper(selection, "Audio")}`;
      updateTimelineSummary();
      updateRenderButton();
    }
  };
  audioElement.onerror = () => {
    if (state.audioPath === selection && state.audioDurationSec <= 0) {
      byId("audioDuration").textContent = "Đã chọn · kiểm tra khi xuất";
      updateRenderButton();
    }
  };
  audioElement.src = convertFileSrc(selection);
  audioElement.load();

  if (state.engine?.available) {
    resultMessage.textContent = "";
    await probeSelectedAudio(selection);
  }
  updateTimelineSummary();
  updateRenderButton();
}

async function chooseOutput(): Promise<string | null> {
  const chosen = await save({
    title: "Lưu video MP4",
    defaultPath: `${stem(state.audioPath)}-wavely.mp4`,
    filters: [{ name: "Video MP4", extensions: ["mp4"] }],
  });
  if (typeof chosen !== "string") return null;
  return chosen.toLowerCase().endsWith(".mp4") ? chosen : `${chosen}.mp4`;
}

async function startRender(): Promise<void> {
  if (renderButton.disabled || state.busy) return;
  resultMessage.textContent = "";
  try {
    capturePublishingRequest();
    const outputPath = await chooseOutput();
    if (!outputPath) return;
    const publishing = capturePublishingRequest();
    pendingPublishing = publishing;
    resetPublishingResult();
    setBusy(true);
    byId("progressMessage").textContent = "Đang chuẩn bị video…";
    byId("progressPercent").textContent = "0%";
    byId("progressFill").style.width = "0%";
    startingRender = true;
    state.jobId = await invoke<string>("start_render", {
      request: {
        backgroundKind: state.backgroundKind,
        backgroundPath: activeBackgroundPath(),
        audioPath: state.audioPath,
        outputPath,
        style: state.style,
        outputFormat: state.outputFormat,
        wavePosition: state.wavePosition,
        waveScale: state.waveScale,
        durationMode: state.durationMode,
        shortVideoMode: state.shortVideoMode,
        startDelaySec: state.startDelaySec,
        endDelaySec: state.endDelaySec,
        keepVideoAudio: state.keepVideoAudio,
        performanceMode: state.performanceMode,
      },
    });
    resultMessage.textContent = "Đang xuất video. Bạn có thể hủy nếu cần.";
    resultMessage.className = "result-message";
    startingRender = false;
    if (earlyRenderEvent) {
      const event = earlyRenderEvent;
      earlyRenderEvent = null;
      handleRenderEvent(event);
    }
  } catch (error) {
    startingRender = false;
    earlyRenderEvent = null;
    pendingPublishing = null;
    setBusy(false);
    showError(String(error));
  }
}

async function cancelRender(): Promise<void> {
  if (!state.jobId) return;
  cancelButton.disabled = true;
  try {
    await invoke("cancel_render", { jobId: state.jobId });
    byId("progressMessage").textContent = "Đang dừng và dọn file tạm…";
  } catch (error) {
    showError(String(error));
    cancelButton.disabled = false;
  }
}

function updatePreset(style: VisualizerStyle): void {
  state.style = style;
  document.querySelectorAll<HTMLButtonElement>(".preset-card").forEach((button) => {
    const selected = button.dataset.style === style;
    button.classList.toggle("is-selected", selected);
    button.setAttribute("aria-checked", String(selected));
  });
  const wave = byId("videoWave");
  const notes: Record<VisualizerStyle, string> = {
    studio: "Cột đối xứng tách khe thoáng, rõ ràng trên mọi nền.",
    eq: "Cột tần số dày chuyển động mượt, hợp nhạc và thuyết minh.",
    blocks: "Khối gạch xếp tầng tối giản, sắc nét trên cả video và ảnh.",
  };
  byId("presetNote").textContent = notes[style];
  wave.dataset.style = style;
  updateWaveScale(state.waveScale);
  videoFrame.dataset.style = style;
}

function updateOutputFormat(format: OutputFormat): void {
  state.outputFormat = format;
  const landscape = format === "landscape";
  const dimensions = landscape ? "1920 × 1080" : "1080 × 1920";
  const aspect = landscape ? "16:9" : "9:16";
  document.querySelectorAll<HTMLButtonElement>(".choice-card[data-format]").forEach((button) => {
    const selected = button.dataset.format === format;
    button.classList.toggle("is-selected", selected);
    button.setAttribute("aria-checked", String(selected));
  });
  videoFrame.dataset.format = format;
  byId("previewFormat").textContent = `${dimensions} · MP4`;
  byId("outputAspect").textContent = aspect;
  updateOutputSummaryText();
  updateBackgroundUI();
}

function updateDurationMode(mode: DurationMode): void {
  state.durationMode = mode;
  document.querySelectorAll<HTMLButtonElement>(".choice-card[data-duration-mode]").forEach((button) => {
    const selected = button.dataset.durationMode === mode;
    button.classList.toggle("is-selected", selected);
    button.setAttribute("aria-checked", String(selected));
  });
  updateBackgroundUI();
}

function parseDelayInput(input: HTMLInputElement): number {
  const raw = Number(input.value);
  if (!Number.isFinite(raw) || raw < 0) return 0;
  return Math.min(300, Math.round(raw * 10) / 10);
}

function updateDelays(): void {
  state.startDelaySec = parseDelayInput(startDelayInput);
  state.endDelaySec = parseDelayInput(endDelayInput);
  updateKeepVideoAudioControl();
  updateTimelineSummary();
  updateRenderButton();
}

function updateWavePosition(position: WavePosition): void {
  state.wavePosition = position;
  document.querySelectorAll<HTMLButtonElement>(".position-card[data-position]").forEach((button) => {
    const selected = button.dataset.position === position;
    button.classList.toggle("is-selected", selected);
    button.setAttribute("aria-checked", String(selected));
  });
  const wave = byId("videoWave");
  const [vertical, horizontal = "center"] = position.split("-");
  wave.dataset.position = position;
  wave.style.left = horizontal === "left" ? "5.55%" : horizontal === "center" ? "50%" : "auto";
  wave.style.right = horizontal === "right" ? "5.55%" : "auto";
  wave.style.top = vertical === "top" ? "5%" : vertical === "center" ? "50%" : "auto";
  wave.style.bottom = vertical === "bottom" ? "5%" : "auto";
  wave.style.transform = `translate(${horizontal === "center" ? "-50%" : "0"}, ${vertical === "center" ? "-50%" : "0"})`;
  updateWaveLayoutSummary();
}

function updateWaveScale(scale: number): void {
  state.waveScale = Math.max(25, Math.min(100, Math.round(scale / 5) * 5));
  byId<HTMLOutputElement>("waveScaleValue").value = `${state.waveScale}%`;
  byId<HTMLInputElement>("waveScale").value = String(state.waveScale);
  const wave = byId("videoWave");
  wave.style.width = `${(8 / 9) * state.waveScale}%`;
  wave.style.height = `${((state.style === "studio" ? 7.5 : 8.33) * state.waveScale) / 100}%`;
  updateWaveLayoutSummary();
}

function updateWaveLayoutSummary(): void {
  const selected = document.querySelector<HTMLButtonElement>(`.position-card[data-position="${state.wavePosition}"]`);
  byId("waveLayoutSummary").textContent = `Vị trí: ${selected?.getAttribute("aria-label") ?? "Dưới giữa"} · Kích thước: ${state.waveScale}%`;
}

async function createPublishingContent(request: PublishingRequest): Promise<void> {
  setBusy(true);
  cancelButton.hidden = true;
  renderProgress.hidden = true;
  renderLabel.textContent = "Đang tạo nội dung đăng bài…";
  try {
    await generatePublishing(request);
    resultMessage.textContent = "Đã lưu video và file nội dung .txt cùng thư mục.";
    resultMessage.className = "result-message is-success";
  } catch {
    resultMessage.textContent = "Video đã được lưu. Dịch vụ AI chưa tạo được nội dung; xem chi tiết và thử lại trong tab Nội dung đăng bài.";
    resultMessage.className = "result-message";
  } finally {
    setBusy(false);
  }
}

async function retryPublishing(): Promise<void> {
  if (state.busy) return;
  try {
    const request = capturePublishingRequest();
    if (!request) throw new Error("Bật tạo nội dung đăng bài để thử lại.");
    await createPublishingContent(request);
  } catch (error) {
    showError(String(error));
  }
}

function handleRenderEvent(event: RenderEvent): void {
  if (startingRender && !state.jobId) {
    earlyRenderEvent = event;
    return;
  }
  if (event.jobId !== state.jobId) return;
  const percent = Math.max(0, Math.min(100, Math.round(event.percent)));
  byId("progressPercent").textContent = `${percent}%`;
  byId("progressFill").style.width = `${percent}%`;
  byId("progressMessage").textContent = event.message;

  if (event.status === "completed") {
    state.jobId = "";
    resultMessage.textContent = `Đã tạo video: ${event.outputPath ?? ""}`;
    resultMessage.className = "result-message is-success";
    byId("progressPercent").textContent = "100%";
    byId("progressFill").style.width = "100%";
    if (pendingPublishing) {
      const request = pendingPublishing;
      pendingPublishing = null;
      void createPublishingContent(request);
    } else {
      setBusy(false);
    }
  } else if (event.status === "cancelled") {
    pendingPublishing = null;
    setBusy(false);
    state.jobId = "";
    resultMessage.textContent = "Đã hủy xuất video. File tạm đã được dọn.";
    resultMessage.className = "result-message";
  } else if (event.status === "error") {
    pendingPublishing = null;
    setBusy(false);
    state.jobId = "";
    showError(event.message);
  }
}

function loadSavedPerformanceMode(): PerformanceMode {
  try {
    const stored = localStorage.getItem(PERF_STORAGE_KEY);
    if (stored === "cpu-fast" || stored === "cpu-compact" || stored === "auto") return stored;
  } catch {
    // Ignore storage failures.
  }
  return "auto";
}

async function init(): Promise<void> {
  state.performanceMode = loadSavedPerformanceMode();
  const perfRadio = document.querySelector<HTMLInputElement>(`input[name="performanceMode"][value="${state.performanceMode}"]`);
  if (perfRadio) perfRadio.checked = true;

  mediaPicker.addEventListener("click", () => void pickMedia().catch((error) => showError(String(error))));
  audioPicker.addEventListener("click", () => void pickAudio().catch((error) => showError(String(error))));
  renderButton.addEventListener("click", () => void startRender());
  cancelButton.addEventListener("click", () => void cancelRender());
  installEngineButton.addEventListener("click", () => void startEngineInstall());
  cancelEngineButton.addEventListener("click", () => void cancelEngineInstall());

  document.querySelectorAll<HTMLInputElement>('input[name="backgroundKind"]').forEach((input) => {
    input.addEventListener("change", () => {
      if (!input.checked) return;
      state.backgroundKind = input.value === "video" ? "video" : "image";
      updateBackgroundUI();
    });
  });

  document.querySelectorAll<HTMLInputElement>('input[name="shortVideoMode"]').forEach((input) => {
    input.addEventListener("change", () => {
      if (!input.checked) return;
      state.shortVideoMode = input.value as ShortVideoMode;
      updateKeepVideoAudioControl();
      updateTimelineSummary();
    });
  });

  document.querySelectorAll<HTMLInputElement>('input[name="performanceMode"]').forEach((input) => {
    input.addEventListener("change", () => {
      if (!input.checked) return;
      state.performanceMode = input.value as PerformanceMode;
      try {
        localStorage.setItem(PERF_STORAGE_KEY, state.performanceMode);
      } catch {
        // Ignore storage failures.
      }
      updateOutputSummaryText();
    });
  });

  keepVideoAudioInput.addEventListener("change", () => {
    state.keepVideoAudio = keepVideoAudioInput.checked;
  });

  startDelayInput.addEventListener("input", updateDelays);
  endDelayInput.addEventListener("input", updateDelays);
  startDelayInput.addEventListener("blur", () => {
    startDelayInput.value = String(state.startDelaySec);
  });
  endDelayInput.addEventListener("blur", () => {
    endDelayInput.value = String(state.endDelaySec);
  });

  selectedVideo.onerror = () => {
    if (state.backgroundKind === "video" && state.videoPath) {
      selectedVideo.hidden = true;
      previewPlaceholder.hidden = false;
      previewEmptyLabel.hidden = false;
      previewEmptyLabel.textContent = "Không xem trước trực tiếp định dạng này (vẫn xuất video bình thường)";
      byId("videoCaption").hidden = true;
    }
  };

  document.querySelectorAll<HTMLButtonElement>(".preset-card").forEach((button) => {
    button.addEventListener("click", () => updatePreset(button.dataset.style as VisualizerStyle));
  });
  document.querySelectorAll<HTMLButtonElement>(".choice-card[data-format]").forEach((button) => {
    button.addEventListener("click", () => updateOutputFormat(button.dataset.format as OutputFormat));
  });
  document.querySelectorAll<HTMLButtonElement>(".choice-card[data-duration-mode]").forEach((button) => {
    button.addEventListener("click", () => updateDurationMode(button.dataset.durationMode as DurationMode));
  });
  document.querySelectorAll<HTMLButtonElement>(".position-card[data-position]").forEach((button) => {
    button.addEventListener("click", () => updateWavePosition(button.dataset.position as WavePosition));
  });
  byId<HTMLInputElement>("waveScale").addEventListener("input", (event) =>
    updateWaveScale(Number((event.currentTarget as HTMLInputElement).value))
  );

  updatePreset(state.style);
  updateWavePosition(state.wavePosition);
  updateWaveScale(state.waveScale);
  updateOutputSummaryText();
  updateBackgroundUI();

  byId<HTMLButtonElement>("playAudio").addEventListener("click", () => {
    if (audioElement.paused) void audioElement.play();
    else audioElement.pause();
  });
  audioElement.addEventListener("play", () => (byId("playAudio").textContent = "Ⅱ"));
  audioElement.addEventListener("pause", () => (byId("playAudio").textContent = "▶"));
  audioElement.addEventListener("ended", () => (byId("playAudio").textContent = "▶"));

  await listen<RenderEvent>("render-progress", (event) => handleRenderEvent(event.payload));
  await listen<EngineInstallEvent>("ffmpeg-install-progress", (event) => void handleEngineInstallEvent(event.payload));
  const version = await invoke<string>("app_version").catch(() => "0.5.0");
  byId("appVersion").textContent = version;
  await refreshEngineStatus();
  await initPublishing(
    (keySaved) => {
      updateRenderButton();
      if (keySaved && resultMessage.classList.contains("is-error") && resultMessage.textContent?.includes("API key")) {
        resultMessage.textContent = "";
        resultMessage.className = "result-message";
      }
    },
    () => void retryPublishing()
  );
}

void init().catch((error) => {
  engineStatus.classList.add("is-error");
  engineStatus.innerHTML = `<span class="status-dot"></span><span>Không khởi tạo được ứng dụng</span>`;
  showError(String(error));
});
