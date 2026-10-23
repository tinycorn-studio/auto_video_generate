import { readFile, writeFile } from "node:fs/promises";

const version = (await readFile(new URL("../VERSION", import.meta.url), "utf8")).trim();
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`VERSION không phải SemVer hợp lệ: ${version}`);
}

const packagePath = new URL("../package.json", import.meta.url);
const packageJson = JSON.parse(await readFile(packagePath, "utf8"));
packageJson.version = version;
await writeFile(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);

const lockPath = new URL("../package-lock.json", import.meta.url);
try {
  const lock = JSON.parse(await readFile(lockPath, "utf8"));
  lock.version = version;
  if (lock.packages?.[""]) lock.packages[""].version = version;
  await writeFile(lockPath, `${JSON.stringify(lock, null, 2)}\n`);
} catch (error) {
  if (error?.code !== "ENOENT") throw error;
}

const cargoPath = new URL("../src-tauri/Cargo.toml", import.meta.url);
let cargo = await readFile(cargoPath, "utf8");
const packageStart = cargo.indexOf("[package]");
const nextSection = cargo.indexOf("\n[", packageStart + 1);
if (packageStart < 0 || nextSection < 0) throw new Error("Không tìm thấy [package] trong Cargo.toml.");
const packageSection = cargo.slice(packageStart, nextSection).replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`);
cargo = `${cargo.slice(0, packageStart)}${packageSection}${cargo.slice(nextSection)}`;
await writeFile(cargoPath, cargo);

const tauriPath = new URL("../src-tauri/tauri.conf.json", import.meta.url);
const tauriConfig = JSON.parse(await readFile(tauriPath, "utf8"));
tauriConfig.version = version;
await writeFile(tauriPath, `${JSON.stringify(tauriConfig, null, 2)}\n`);

console.log(`Version đồng bộ: ${version}`);
