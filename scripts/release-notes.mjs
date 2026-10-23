import { readFile } from "node:fs/promises";

const inputTag = process.argv[2]?.trim();
if (!inputTag) throw new Error("Cách dùng: node scripts/release-notes.mjs vX.Y.Z");
const version = inputTag.replace(/^v/, "");
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) throw new Error(`Tag không phải SemVer: ${inputTag}`);
const current = (await readFile(new URL("../VERSION", import.meta.url), "utf8")).trim();
if (current !== version) throw new Error(`Tag ${inputTag} không khớp VERSION ${current}.`);
const changelog = await readFile(new URL("../CHANGELOG.md", import.meta.url), "utf8");
const escaped = version.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const heading = new RegExp(`^## \\[${escaped}\\] - \\d{4}-\\d{2}-\\d{2}\\s*$`, "m");
const match = heading.exec(changelog);
if (!match) throw new Error(`CHANGELOG.md thiếu mục ## [${version}] - YYYY-MM-DD.`);
const afterHeading = changelog.slice(match.index + match[0].length);
const nextSection = /^## \[/m.exec(afterHeading);
const details = afterHeading.slice(0, nextSection?.index ?? afterHeading.length).trim();
if (!details || /^### [A-Za-z]+\s*-\s*$/m.test(details)) throw new Error(`Mục changelog ${version} chưa có nội dung.`);

const notes = `# v${version} — Tạo video từ ảnh và âm thanh\n\nWavely xử lý file trực tiếp trên máy Windows hoặc macOS.\n\n## Có gì mới\n\n${details}\n\n## Tải và cài đặt\n\n- **Windows:** tải file .exe hoặc .msi phù hợp trong Assets, rồi chạy installer.\n- **macOS:** tải DMG phù hợp kiến trúc Intel hoặc Apple Silicon, kéo Wavely vào Applications. Nếu Gatekeeper cảnh báo app chưa ký, control-click Wavely rồi chọn **Open**.\n- Xem hướng dẫn FFmpeg tương ứng với phiên bản trong CHANGELOG và README.\n\n## Lưu ý\n\n- File cài hiện chưa được ký số/notarize. Windows có thể hiện SmartScreen; macOS có thể yêu cầu xác nhận mở lần đầu.\n`;
process.stdout.write(notes);
