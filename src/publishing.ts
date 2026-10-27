import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

export type PublishingRequest = { language: "en" | "vi"; platforms: string[]; transcript: string };
type Post = { platform: string; title: string; description: string; hashtags: string[]; keywords: string[] };
type PublishingResult = { posts: Post[]; textPath: string; text: string; warning: string | null };
type KeyStatus = { saved: boolean; provider: "google" | "xpiki" | null };
const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const platformNames: Record<string, string> = { youtube: "YouTube", tiktok: "TikTok", facebook: "Facebook", instagram: "Instagram" };
const preferenceKey = "wavely-publishing-preferences";
let hasKey = false;
let keyProvider: KeyStatus["provider"] = null;
let busy = false;
let savingKey = false;
let current: PublishingResult | null = null;
let selectedPost: Post | null = null;
let onChange: (keySaved?: boolean) => void = () => {};

export function publishingEnabled(): boolean { return $<HTMLInputElement>("publishingEnabled").checked; }

function platforms(): string[] {
  return Array.from(document.querySelectorAll<HTMLInputElement>(".platform-options input:checked")).map((input) => input.value);
}

function updateControls(): void {
  $<HTMLInputElement>("publishingEnabled").disabled = busy || savingKey;
  $<HTMLFieldSetElement>("publishingOptions").disabled = busy || !publishingEnabled();
  $<HTMLInputElement>("geminiKey").disabled = busy || savingKey;
  for (const id of ["saveGeminiKey", "deleteGeminiKey", "toggleGeminiKey"]) $<HTMLButtonElement>(id).disabled = busy || savingKey;
  $<HTMLButtonElement>("retryPublishing").disabled = busy || savingKey;
}

export function setPublishingBusy(value: boolean): void { busy = value; updateControls(); }

function keyStatus(message: string, error = false): void {
  $("geminiKeyStatus").textContent = message;
  $("geminiKeyStatus").classList.toggle("is-error", error);
}

function updateKeyState(): void {
  $("geminiBadge").textContent = hasKey ? "✦ Gemini · đã kết nối" : "✦ Gemini · chưa lưu key";
  $("geminiBadge").classList.toggle("has-key", hasKey);
  $("deleteGeminiKey").hidden = !hasKey;
  $<HTMLInputElement>("geminiKey").placeholder = hasKey ? "•••••••••••••••• · Nhập key Gemini mới để thay" : "Nhập Gemini API key";
  const destination = keyProvider === "google" ? "Google Gemini" : keyProvider === "xpiki" ? "XPiki" : "dịch vụ theo key bạn lưu";
  $("geminiDisclosure").textContent = `Transcript/tóm tắt bạn nhập sẽ được gửi tới ${destination} khi bật tạo nội dung. Wavely không gửi âm thanh hoặc video. Cần Internet và có thể phát sinh phí theo quota API.`;
}

function savePreferences(): void {
  // Only user preferences. Never store a key, prompt, generated post, or history here.
  try { localStorage.setItem(preferenceKey, JSON.stringify({ enabled: publishingEnabled(), language: $<HTMLSelectElement>("publishingLanguage").value, platforms: platforms() })); }
  catch { /* Preferences are optional if storage is unavailable. */ }
}

export function capturePublishingRequest(): PublishingRequest | null {
  if (!publishingEnabled()) return null;
  if (savingKey) throw new Error("Đang lưu API key. Hãy chờ hoàn tất.");
  if ($<HTMLInputElement>("geminiKey").value.trim()) throw new Error("Hãy bấm Lưu để kiểm tra và lưu API key vừa nhập.");
  if (!hasKey) throw new Error("Hãy nhập và lưu Gemini API key trước khi tạo nội dung.");
  const selected = platforms();
  if (!selected.length) throw new Error("Hãy chọn ít nhất một nền tảng để tạo nội dung.");
  const transcript = $<HTMLTextAreaElement>("publishingTranscript").value.trim();
  if (!transcript) throw new Error("Hãy dán transcript hoặc tóm tắt nội dung âm thanh.");
  if (transcript.length > 40000) throw new Error("Transcript hoặc tóm tắt tối đa 40.000 ký tự.");
  return { language: $<HTMLSelectElement>("publishingLanguage").value as "en" | "vi", platforms: selected, transcript };
}

function selectTab(content: boolean): void {
  for (const [id, selected] of [["videoTab", !content], ["contentTab", content]] as const) {
    $(id).setAttribute("aria-selected", String(selected));
    $(id).tabIndex = selected ? 0 : -1;
  }
  $("videoPreviewPanel").hidden = content;
  $("contentPreviewPanel").hidden = !content;
}

export function resetPublishingResult(): void {
  current = null;
  selectedPost = null;
  $("postResults").hidden = true;
  $("retryPublishing").hidden = true;
  $("contentActionStatus").textContent = "";
  $("publishingStatus").className = "publishing-status";
  $("publishingStatus").textContent = publishingEnabled()
    ? "Nội dung sẽ được tạo sau khi xuất video xong."
    : "Bật tạo nội dung đăng bài để dùng Gemini sau khi xuất video.";
  selectTab(false);
}

function selectPost(post: Post): void {
  selectedPost = post;
  $("postTitle").textContent = post.title;
  $("postDescription").textContent = post.description;
  $("postHashtags").textContent = post.hashtags.length ? post.hashtags.join(" ") : "Không đề xuất hashtag.";
  $("postKeywords").textContent = post.keywords.length ? post.keywords.join(", ") : "Không đề xuất từ khóa.";
  document.querySelector<HTMLButtonElement>('[data-copy="hashtags"]')!.disabled = !post.hashtags.length;
  document.querySelector<HTMLButtonElement>('[data-copy="keywords"]')!.disabled = !post.keywords.length;
  $("postPlatforms").querySelectorAll<HTMLButtonElement>("button").forEach((b) => {
    b.classList.toggle("is-selected", b.dataset.platform === post.platform);
    b.setAttribute("aria-pressed", String(b.dataset.platform === post.platform));
  });
}

export async function generatePublishing(request: PublishingRequest): Promise<void> {
  current = null;
  selectedPost = null;
  $("postResults").hidden = true;
  $("retryPublishing").hidden = true;
  $("contentActionStatus").textContent = "";
  $("publishingStatus").className = "publishing-status is-generating";
  $("publishingStatus").textContent = "Video đã xuất xong. Gemini đang xử lý transcript/tóm tắt…";
  selectTab(true);
  try {
    current = await invoke<PublishingResult>("generate_publishing_content", { request });
    $("publishingStatus").className = `publishing-status ${current.warning ? "is-warning" : "is-success"}`;
    $("publishingStatus").textContent = current.warning ?? "✓ Video và nội dung đã sẵn sàng. File .txt đã được lưu cạnh video.";
    $("contentFilePath").textContent = current.textPath;
    const buttons = current.posts.map((post) => {
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = platformNames[post.platform] ?? post.platform;
      button.dataset.platform = post.platform;
      button.addEventListener("click", () => selectPost(post));
      return button;
    });
    $("postPlatforms").replaceChildren(...buttons);
    selectPost(current.posts[0]);
    $("postResults").hidden = false;
  } catch (error) {
    $("publishingStatus").className = "publishing-status is-error";
    $("publishingStatus").textContent = `Video đã được lưu. Chưa tạo được nội dung: ${String(error)}`;
    $("retryPublishing").hidden = false;
    throw error;
  }
}

async function copy(text: string): Promise<void> {
  try { await writeText(text); $("contentActionStatus").textContent = "Đã sao chép vào clipboard."; }
  catch { $("contentActionStatus").textContent = "Không sao chép được. Bạn có thể mở file .txt để lấy nội dung."; }
}

export async function initPublishing(change: (keySaved?: boolean) => void, retry: () => void): Promise<void> {
  onChange = change;
  try {
    const stored: unknown = JSON.parse(localStorage.getItem(preferenceKey) ?? "null");
    if (stored && typeof stored === "object") {
      const p = stored as Record<string, unknown>;
      $<HTMLInputElement>("publishingEnabled").checked = p.enabled === true;
      $<HTMLSelectElement>("publishingLanguage").value = p.language === "vi" ? "vi" : "en";
      if (Array.isArray(p.platforms)) document.querySelectorAll<HTMLInputElement>(".platform-options input").forEach((input) => { input.checked = (p.platforms as unknown[]).includes(input.value); });
    }
  } catch { /* Keep English and the default platforms if preferences are invalid. */ }
  $("publishingEnabled").addEventListener("change", () => { updateControls(); savePreferences(); onChange(); });
  $("publishingLanguage").addEventListener("change", savePreferences);
  document.querySelectorAll<HTMLInputElement>(".platform-options input").forEach((input) => input.addEventListener("change", savePreferences));
  $("videoTab").addEventListener("click", () => selectTab(false));
  $("contentTab").addEventListener("click", () => selectTab(true));
  document.querySelector(".preview-tabs")!.addEventListener("keydown", (event) => {
    const key = (event as KeyboardEvent).key;
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(key)) return;
    event.preventDefault();
    const content = key === "End" || (key !== "Home" && $("videoTab").getAttribute("aria-selected") === "true");
    selectTab(content); $(content ? "contentTab" : "videoTab").focus();
  });
  $("toggleGeminiKey").addEventListener("click", () => {
    const input = $<HTMLInputElement>("geminiKey"); const reveal = input.type === "password";
    input.type = reveal ? "text" : "password";
    $("toggleGeminiKey").textContent = reveal ? "Ẩn" : "Hiện";
    $("toggleGeminiKey").setAttribute("aria-label", reveal ? "Ẩn API key vừa nhập" : "Hiện API key vừa nhập");
    $("toggleGeminiKey").setAttribute("aria-pressed", String(reveal));
  });
  $("saveGeminiKey").addEventListener("click", () => void (async () => {
    const key = $<HTMLInputElement>("geminiKey").value.trim();
    if (!key) { keyStatus("Hãy nhập Gemini API key trước khi lưu.", true); return; }
    savingKey = true; updateControls(); keyStatus("Đang nhận diện và kiểm tra kết nối Gemini…");
    try {
      const status = await invoke<KeyStatus>("save_publishing_key", { apiKey: key });
      hasKey = status.saved;
      keyProvider = status.provider;
      $<HTMLInputElement>("geminiKey").value = "";
      $<HTMLInputElement>("geminiKey").type = "password";
      $("toggleGeminiKey").textContent = "Hiện";
      $("toggleGeminiKey").setAttribute("aria-label", "Hiện API key vừa nhập");
      $("toggleGeminiKey").setAttribute("aria-pressed", "false");
      keyStatus("✓ Đã kết nối Gemini và lưu an toàn trên máy.");
      onChange(true);
    } catch (error) { keyStatus(String(error), true); }
    finally { savingKey = false; updateKeyState(); updateControls(); onChange(); }
  })());
  $("deleteGeminiKey").addEventListener("click", () => void (async () => {
    savingKey = true; updateControls();
    try {
      await invoke("delete_publishing_key"); hasKey = false; keyProvider = null; $<HTMLInputElement>("geminiKey").value = "";
      $<HTMLInputElement>("publishingEnabled").checked = false; savePreferences();
      keyStatus("Đã xóa API key khỏi kho bảo mật.");
    }
    catch (error) { keyStatus(String(error), true); }
    finally { savingKey = false; updateKeyState(); updateControls(); onChange(); }
  })());
  $("retryPublishing").addEventListener("click", retry);
  document.querySelectorAll<HTMLButtonElement>("[data-copy]").forEach((button) => button.addEventListener("click", () => {
    if (!selectedPost) return;
    const value = selectedPost[button.dataset.copy as keyof Post];
    void copy(Array.isArray(value) ? value.join(button.dataset.copy === "keywords" ? ", " : " ") : value);
  }));
  $("copyAllContent").addEventListener("click", () => { if (current) void copy(current.text); });
  for (const [id, folder] of [["openContentFile", false], ["openContentFolder", true]] as const) $(id).addEventListener("click", () => {
    void invoke("open_publishing_output", { folder }).catch((error) => { $("contentActionStatus").textContent = String(error); });
  });
  await listen<string>("publishing-progress", (event) => { if (busy) $("publishingStatus").textContent = event.payload; });
  try {
    const status = await invoke<KeyStatus>("publishing_key_status");
    hasKey = status.saved; keyProvider = status.provider;
    keyStatus(hasKey ? "✓ Đã lưu an toàn trên máy." : "Hỗ trợ key Google Gemini và XPiki; tự nhận diện.");
  }
  catch (error) { keyStatus(String(error), true); }
  updateKeyState(); updateControls(); onChange();
}
