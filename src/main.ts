import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { capturePublishingRequest, generatePublishing, initPublishing, publishingEnabled, resetPublishingResult, setPublishingBusy } from "./publishing";
import type { PublishingRequest } from "./publishing";
import "./style.css";

type VisualizerStyle = "minimal" | "balanced" | "spectrum";
type OutputFormat = "portrait" | "landscape";
type WavePosition = "top-left" | "top-center" | "top-right" | "center-left" | "center" | "center-right" | "bottom-left" | "bottom-center" | "bottom-right";
type EngineStatus = { available: boolean; version: string | null; message: string };
type EngineInstallEvent = { status: "downloading" | "extracting" | "ready" | "cancelled" | "error"; percent: number; message: string };
type RenderEvent = {
  jobId: string;
  status: "running" | "completed" | "cancelled" | "error";
  percent: number;
  message: string;
  outputPath?: string;
};

const byId = <T extends HTMLElement>(id: string) => {
  const element = document.getElementById(id);
  if (!element) throw new Error(`Missing element #${id}`);
  return element as T;
};

const imagePicker = byId<HTMLButtonElement>("imagePicker");
const audioPicker = byId<HTMLButtonElement>("audioPicker");
const renderButton = byId<HTMLButtonElement>("renderButton");
const cancelButton = byId<HTMLButtonElement>("cancelButton");
const audioElement = byId<HTMLAudioElement>("audioElement");
const selectedImage = byId<HTMLImageElement>("selectedImage");
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
const state = { imagePath: "", audioPath: "", style: "minimal" as VisualizerStyle, outputFormat: "portrait" as OutputFormat, wavePosition: "bottom-center" as WavePosition, waveScale: 100, engine: null as EngineStatus | null, jobId: "", busy: false, installing: false };
let pendingPublishing: PublishingRequest | null = null;
let startingRender = false;
let earlyRenderEvent: RenderEvent | null = null;

function humanDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "—";
  const total = Math.floor(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const remainder = total % 60;
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}` : `${minutes}:${String(remainder).padStart(2, "0")}`;
}

function stem(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? path;
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
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
  renderButton.disabled = state.busy || !state.imagePath || !state.audioPath || !state.engine?.available;
  imagePicker.disabled = state.busy;
  audioPicker.disabled = state.busy;
  document.querySelectorAll<HTMLButtonElement>(".preset-card, .choice-card, .position-card").forEach((button) => (button.disabled = state.busy));
  byId<HTMLInputElement>("waveScale").disabled = state.busy;
  if (!state.busy) renderLabel.textContent = publishingEnabled() ? "Tạo video & nội dung đăng bài" : "Tạo video";
}

function showError(message: string): void {
  resultMessage.textContent = message.replace(/^Error: /, "");
  resultMessage.className = "result-message is-error";
}

async function refreshEngineStatus(): Promise<void> {
  setEngine(await invoke<EngineStatus>("ffmpeg_status"));
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

async function pickImage(): Promise<void> {
  if (state.busy) return;
  const selection = await open({
    multiple: false,
    title: "Chọn ảnh nền",
    filters: [{ name: "Ảnh", extensions: ["jpg", "jpeg", "png", "webp", "bmp"] }],
  });
  if (typeof selection !== "string") return;
  state.imagePath = selection;
  const name = stem(selection);
  byId("imageName").textContent = name;
  byId("imageDetails").textContent = selection.split(".").pop()?.toUpperCase() ?? "Ảnh đã chọn";
  byId("imageAction").textContent = "Đổi ảnh";
  selectedImage.onload = () => {
    byId("imageDetails").textContent = `${selectedImage.naturalWidth} × ${selectedImage.naturalHeight} · ${selection.split(".").pop()?.toUpperCase() ?? "Ảnh"}`;
  };
  selectedImage.onerror = () => showError("Không thể xem trước ảnh này. Bạn vẫn có thể thử xuất video.");
  selectedImage.src = convertFileSrc(selection);
  selectedImage.hidden = false;
  previewPlaceholder.hidden = true;
  previewEmptyLabel.hidden = true;
  byId("videoCaption").hidden = true;
  if (state.engine?.available) resultMessage.textContent = "";
  updateRenderButton();
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
  resetPublishingResult();
  const name = stem(selection);
  byId("audioName").textContent = name;
  byId("publishingAudioFile").textContent = `${name}.${selection.split(".").pop()?.toUpperCase() ?? "AUDIO"} · Chỉ gửi nếu model hỗ trợ audio`;
  byId("audioTrackName").textContent = name;
  byId("audioAction").textContent = "Đổi file";
  byId("audioPreview").hidden = false;
  byId("audioDuration").textContent = "Đang đọc thời lượng…";
  audioElement.onloadedmetadata = () => {
    byId("audioDuration").textContent = `${humanDuration(audioElement.duration)} · ${selection.split(".").pop()?.toUpperCase() ?? "Audio"}`;
    updateRenderButton();
  };
  audioElement.onerror = () => {
    byId("audioDuration").textContent = "Đã chọn · kiểm tra khi xuất";
    updateRenderButton();
  };
  audioElement.src = convertFileSrc(selection);
  audioElement.load();
  if (state.engine?.available) resultMessage.textContent = "";
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
    capturePublishingRequest(); // Validate before opening the output picker or starting a render.
    const outputPath = await chooseOutput();
    if (!outputPath) return;
    const publishing = capturePublishingRequest(); // Respect changes made while the native picker was open.
    pendingPublishing = publishing;
    resetPublishingResult();
    setBusy(true);
    byId("progressMessage").textContent = "Đang chuẩn bị video…";
    byId("progressPercent").textContent = "0%";
    byId("progressFill").style.width = "0%";
    startingRender = true;
    state.jobId = await invoke<string>("start_render", {
      imagePath: state.imagePath,
      audioPath: state.audioPath,
      outputPath,
      style: state.style,
      outputFormat: state.outputFormat,
      wavePosition: state.wavePosition,
      waveScale: state.waveScale,
    });
    resultMessage.textContent = "Đang xuất video. Bạn có thể hủy nếu cần.";
    resultMessage.className = "result-message";
    startingRender = false;
    if (earlyRenderEvent) { const event = earlyRenderEvent; earlyRenderEvent = null; handleRenderEvent(event); }
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
  const spectrum = style === "spectrum";
  const balanced = style === "balanced";
  const wave = byId("videoWave");
  byId("presetNote").textContent = spectrum
    ? "Cột tần số gọn, phù hợp nhạc và beat."
    : balanced
      ? "Mỗi nhịp vươn đều lên trên và xuống dưới trục giữa."
      : "Đường sóng mảnh, nhẹ và dễ nhìn trên nền ảnh.";
  wave.classList.toggle("is-spectrum", spectrum);
  wave.classList.toggle("is-balanced", balanced);
  wave.querySelector("svg")!.toggleAttribute("hidden", spectrum);
  wave.querySelector<SVGPathElement>(".minimal-wave")!.toggleAttribute("hidden", balanced);
  wave.querySelector<SVGGElement>(".balanced-wave")!.toggleAttribute("hidden", !balanced);
  wave.querySelector<HTMLElement>(".spectrum-bars")!.hidden = !spectrum;
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
  byId("outputDimensions").textContent = `MP4 · ${dimensions} · 30 fps · H.264`;
  byId("outputAspect").textContent = aspect;
  byId("imageAspectHint").textContent = `Ảnh sẽ cắt theo ${aspect}`;
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
  wave.style.height = `${(state.style === "spectrum" ? 9.2 : 7.5) * state.waveScale / 100}%`;
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
  } finally { setBusy(false); }
}

async function retryPublishing(): Promise<void> {
  if (state.busy) return;
  try {
    const request = capturePublishingRequest();
    if (!request) throw new Error("Bật tạo nội dung đăng bài để thử lại.");
    await createPublishingContent(request);
  } catch (error) { showError(String(error)); }
}

function handleRenderEvent(event: RenderEvent): void {
  if (startingRender && !state.jobId) { earlyRenderEvent = event; return; }
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
    } else { setBusy(false); }
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

async function init(): Promise<void> {
  imagePicker.addEventListener("click", () => void pickImage().catch((error) => showError(String(error))));
  audioPicker.addEventListener("click", () => void pickAudio().catch((error) => showError(String(error))));
  renderButton.addEventListener("click", () => void startRender());
  cancelButton.addEventListener("click", () => void cancelRender());
  installEngineButton.addEventListener("click", () => void startEngineInstall());
  cancelEngineButton.addEventListener("click", () => void cancelEngineInstall());
  document.querySelectorAll<HTMLButtonElement>(".preset-card").forEach((button) => {
    button.addEventListener("click", () => updatePreset(button.dataset.style as VisualizerStyle));
  });
  document.querySelectorAll<HTMLButtonElement>(".choice-card[data-format]").forEach((button) => {
    button.addEventListener("click", () => updateOutputFormat(button.dataset.format as OutputFormat));
  });
  document.querySelectorAll<HTMLButtonElement>(".position-card[data-position]").forEach((button) => {
    button.addEventListener("click", () => updateWavePosition(button.dataset.position as WavePosition));
  });
  byId<HTMLInputElement>("waveScale").addEventListener("input", (event) => updateWaveScale(Number((event.currentTarget as HTMLInputElement).value)));
  updateWavePosition(state.wavePosition);
  updateWaveScale(state.waveScale);
  byId<HTMLButtonElement>("playAudio").addEventListener("click", () => {
    if (audioElement.paused) void audioElement.play();
    else audioElement.pause();
  });
  audioElement.addEventListener("play", () => (byId("playAudio").textContent = "Ⅱ"));
  audioElement.addEventListener("pause", () => (byId("playAudio").textContent = "▶"));
  audioElement.addEventListener("ended", () => (byId("playAudio").textContent = "▶"));
  await listen<RenderEvent>("render-progress", (event) => handleRenderEvent(event.payload));
  await listen<EngineInstallEvent>("ffmpeg-install-progress", (event) => void handleEngineInstallEvent(event.payload));
  const version = await invoke<string>("app_version").catch(() => "0.3.0");
  byId("appVersion").textContent = version;
  await refreshEngineStatus();
  await initPublishing((keySaved) => {
    updateRenderButton();
    if (keySaved && resultMessage.classList.contains("is-error") && resultMessage.textContent?.includes("API key")) {
      resultMessage.textContent = "";
      resultMessage.className = "result-message";
    }
  }, () => void retryPublishing());
}

void init().catch((error) => {
  engineStatus.classList.add("is-error");
  engineStatus.innerHTML = `<span class="status-dot"></span><span>Không khởi tạo được ứng dụng</span>`;
  showError(String(error));
});
