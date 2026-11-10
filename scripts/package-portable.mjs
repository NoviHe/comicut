// Package ComiCut Portable (.zip) release bundle + SHA-256 checksum + portable.json manifest.
//
// Usage:
//   npm run portable          (packages existing src-tauri/target/release binary into release_assets/)
//   npm run build:portable    (builds frontend + Rust release binary, then packages portable .zip)

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  createReadStream,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const TAURI_CONF = JSON.parse(
  readFileSync(join(ROOT, "src-tauri", "tauri.conf.json"), "utf8")
);
const VERSION = TAURI_CONF.version || "0.1.0";
const PRODUCT_NAME = TAURI_CONF.productName || "ComiCut";
const REPO_SLUG = "NoviHe/comicut";
const TRIPLE = "x86_64-pc-windows-msvc";

const RELEASE_DIR = join(ROOT, "src-tauri", "target", "release");
const BINARIES_DIR = join(ROOT, "src-tauri", "binaries");
const OUT_DIR = join(ROOT, "release_assets");

const ZIP_NAME = `${PRODUCT_NAME}_${VERSION}_x64-portable.zip`;
const ZIP_PATH = join(OUT_DIR, ZIP_NAME);
const SHA_PATH = `${ZIP_PATH}.sha256`;
const MANIFEST_PATH = join(OUT_DIR, "portable.json");

const log = (msg) => console.log(`[portable] ${msg}`);

function resolveMainExe() {
  const candidates = [
    join(RELEASE_DIR, "comicut.exe"),
    join(RELEASE_DIR, `${PRODUCT_NAME}.exe`),
  ];
  const hit = candidates.find((p) => existsSync(p));
  if (!hit) {
    throw new Error(
      `Release executable not found in ${RELEASE_DIR}. Run "npm run build:portable" or "npm run tauri build" first.`
    );
  }
  return hit;
}

function resolveSidecar(tool) {
  const candidates = [
    join(RELEASE_DIR, `${tool}.exe`),
    join(BINARIES_DIR, `${tool}-${TRIPLE}.exe`),
  ];
  const hit = candidates.find((p) => existsSync(p));
  if (!hit) {
    throw new Error(
      `Sidecar ${tool}.exe not found. Run "npm run sidecars" first.`
    );
  }
  return hit;
}

function sha256File(filePath) {
  return new Promise((resolve, reject) => {
    const hash = createHash("sha256");
    const stream = createReadStream(filePath);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.on("end", () => resolve(hash.digest("hex")));
    stream.on("error", reject);
  });
}

const workDir = mkdtempSync(join(tmpdir(), "comicut-portable-"));
try {
  const mainExe = resolveMainExe();
  const ffmpegExe = resolveSidecar("ffmpeg");
  const ffprobeExe = resolveSidecar("ffprobe");

  log(`Staging ${PRODUCT_NAME} v${VERSION} Portable...`);
  copyFileSync(mainExe, join(workDir, `${PRODUCT_NAME}.exe`));
  copyFileSync(ffmpegExe, join(workDir, "ffmpeg.exe"));
  copyFileSync(ffprobeExe, join(workDir, "ffprobe.exe"));

  const portableNote = [
    `${PRODUCT_NAME} Studio v${VERSION} — Portable Edition`,
    "================================================",
    "Mode: Portable (.zip)",
    "",
    "- Jalankan langsung lewat ComiCut.exe tanpa instalasi.",
    "- Jangan hapus ffmpeg.exe dan ffprobe.exe di folder ini.",
    "- File portable.txt ini menandakan mode Portable agar fitur Auto-Update",
    "  mengunduh paket khusus Portable (.zip) dan memperbarui langsung di folder ini",
    "  (bukan menjalankan installer Setup/MSI).",
    "",
  ].join("\r\n");
  writeFileSync(join(workDir, "portable.txt"), portableNote, "utf8");

  mkdirSync(OUT_DIR, { recursive: true });
  if (existsSync(ZIP_PATH)) {
    rmSync(ZIP_PATH, { force: true });
  }

  log(`Compressing ${ZIP_NAME}...`);
  execFileSync(
    "tar",
    [
      "-a",
      "-cf",
      ZIP_PATH,
      "-C",
      workDir,
      `${PRODUCT_NAME}.exe`,
      "ffmpeg.exe",
      "ffprobe.exe",
      "portable.txt",
    ],
    { stdio: "inherit" }
  );

  const hash = await sha256File(ZIP_PATH);
  const sizeMb = (statSync(ZIP_PATH).size / (1024 * 1024)).toFixed(1);

  writeFileSync(SHA_PATH, `${hash}  ${ZIP_NAME}\n`, "utf8");

  // Read existing latest.json notes if present so portable.json stays in sync
  let notes = `${PRODUCT_NAME} v${VERSION} — Portable Release`;
  const latestJsonPath = join(OUT_DIR, "latest.json");
  if (existsSync(latestJsonPath)) {
    try {
      const raw = readFileSync(latestJsonPath, "utf8").replace(/^\uFEFF/, "");
      const latest = JSON.parse(raw);
      if (latest.notes) notes = latest.notes;
    } catch {
      // Ignore parse error
    }
  }

  const portableManifest = {
    version: VERSION,
    notes,
    pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
    platforms: {
      "windows-x86_64": {
        url: `https://github.com/${REPO_SLUG}/releases/download/v${VERSION}/${ZIP_NAME}`,
        sha256: hash,
      },
    },
  };
  writeFileSync(
    MANIFEST_PATH,
    JSON.stringify(portableManifest, null, 2) + "\n",
    "utf8"
  );

  log(`Created: ${ZIP_PATH} (${sizeMb} MB)`);
  log(`SHA-256: ${hash}`);
  log(`Created: ${SHA_PATH}`);
  log(`Created: ${MANIFEST_PATH}`);
} finally {
  rmSync(workDir, { recursive: true, force: true });
}
