import { spawnSync } from "node:child_process";
import { writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const next = process.argv[2]?.trim();
if (!next || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(next)) {
  throw new Error("Cách dùng: npm run version:set -- X.Y.Z (có thể thêm hậu tố prerelease SemVer).");
}
await writeFile(new URL("../VERSION", import.meta.url), `${next}\n`);
const syncScript = fileURLToPath(new URL("./sync-version.mjs", import.meta.url));
const result = spawnSync(process.execPath, [syncScript], { stdio: "inherit" });
if (result.status !== 0) throw new Error("Không đồng bộ được metadata version.");
console.log(`Đã đặt version ${next}. Hãy cập nhật CHANGELOG trước khi commit release và tạo tag.`);
