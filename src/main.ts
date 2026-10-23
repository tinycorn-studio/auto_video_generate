import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import "./style.css";

type VisualizerStyle = "minimal" | "spectrum";
type OutputFormat = "portrait" | "landscape";
type WavePosition = "top" | "center" | "bottom";
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
const state = { imagePath: "", audioPath: "", style: "minimal" as VisualizerStyle, outputFormat: "portrait" as OutputFormat, wavePosition: "bottom" as WavePosition, engine: null as EngineStatus | null, jobId: "", busy: false, installing: false };

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
      ? "Tải FFmpeg một lần vào thư mục riêng của Wavely. Sau đó có thể xuất video offline; file ảnh và âm thanh không được tải lên."
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
  document.querySelectorAll<HTMLButtonElement>(".preset-card, .choice-card").forEach((button) => (button.disabled = state.busy));
}

function showError(message: string): void {
  resultMessage.textContent = message;
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
  const name = stem(selection);
  byId("audioName").textContent = name;
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
    const outputPath = await chooseOutput();
    if (!outputPath) return;
    setBusy(true);
    byId("progressMessage").textContent = "Đang chuẩn bị video…";
    byId("progressPercent").textContent = "0%";
    byId("progressFill").style.width = "0%";
    state.jobId = await invoke<string>("start_render", {
      imagePath: state.imagePath,
      audioPath: state.audioPath,
      outputPath,
      style: state.style,
      outputFormat: state.outputFormat,
      wavePosition: state.wavePosition,
    });
    resultMessage.textContent = "Đang xuất video. Bạn có thể hủy nếu cần.";
    resultMessage.className = "result-message";
  } catch (error) {
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
  byId("presetNote").textContent = spectrum ? "Cột tần số gọn, phù hợp nhạc và beat." : "Sóng mảnh, nhẹ và dễ nhìn trên nền ảnh.";
  byId("videoWave").classList.toggle("is-spectrum", spectrum);
  byId("videoWave").querySelector("svg")!.toggleAttribute("hidden", spectrum);
  byId("videoWave").querySelector<HTMLElement>(".spectrum-bars")!.hidden = !spectrum;
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
  document.querySelectorAll<HTMLButtonElement>(".choice-card[data-position]").forEach((button) => {
    const selected = button.dataset.position === position;
    button.classList.toggle("is-selected", selected);
    button.setAttribute("aria-checked", String(selected));
  });
  byId("videoWave").dataset.position = position;
}

function handleRenderEvent(event: RenderEvent): void {
  if (event.jobId !== state.jobId) return;
  const percent = Math.max(0, Math.min(100, Math.round(event.percent)));
  byId("progressPercent").textContent = `${percent}%`;
  byId("progressFill").style.width = `${percent}%`;
  byId("progressMessage").textContent = event.message;

  if (event.status === "completed") {
    setBusy(false);
    state.jobId = "";
    resultMessage.textContent = `Đã tạo video: ${event.outputPath ?? ""}`;
    resultMessage.className = "result-message is-success";
    byId("progressPercent").textContent = "100%";
    byId("progressFill").style.width = "100%";
  } else if (event.status === "cancelled") {
    setBusy(false);
    state.jobId = "";
    resultMessage.textContent = "Đã hủy xuất video. File tạm đã được dọn.";
    resultMessage.className = "result-message";
  } else if (event.status === "error") {
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
  document.querySelectorAll<HTMLButtonElement>(".choice-card[data-position]").forEach((button) => {
    button.addEventListener("click", () => updateWavePosition(button.dataset.position as WavePosition));
  });
  byId<HTMLButtonElement>("playAudio").addEventListener("click", () => {
    if (audioElement.paused) void audioElement.play();
    else audioElement.pause();
  });
  audioElement.addEventListener("play", () => (byId("playAudio").textContent = "Ⅱ"));
  audioElement.addEventListener("pause", () => (byId("playAudio").textContent = "▶"));
  audioElement.addEventListener("ended", () => (byId("playAudio").textContent = "▶"));
  await listen<RenderEvent>("render-progress", (event) => handleRenderEvent(event.payload));
  await listen<EngineInstallEvent>("ffmpeg-install-progress", (event) => void handleEngineInstallEvent(event.payload));
  const version = await invoke<string>("app_version").catch(() => "0.2.0");
  byId("appVersion").textContent = version;
  await refreshEngineStatus();
}

void init().catch((error) => {
  engineStatus.classList.add("is-error");
  engineStatus.innerHTML = `<span class="status-dot"></span><span>Không khởi tạo được ứng dụng</span>`;
  showError(String(error));
});
