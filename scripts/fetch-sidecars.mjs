// Fetch FFmpeg + FFprobe (gyan.dev "release essentials", SHA-256 verified) as Tauri sidecars
// into src-tauri/binaries/<tool>-<target-triple>.exe. Tauri bundles them next to ComiCut.exe.
//
// Usage: npm run sidecars            (skips if already present)
//        npm run sidecars -- --force (re-download latest release)
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, createWriteStream, existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { Readable, Transform } from "node:stream";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const OUT_DIR = join(ROOT, "src-tauri", "binaries");
const TRIPLE = "x86_64-pc-windows-msvc";
const TOOLS = ["ffmpeg", "ffprobe"];
const ZIP_URL = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
const SHA_URL = `${ZIP_URL}.sha256`;
const VERSION_URL = "https://www.gyan.dev/ffmpeg/builds/release-version";

const target = (tool) => join(OUT_DIR, `${tool}-${TRIPLE}.exe`);
const log = (m) => console.log(`[sidecars] ${m}`);

if (process.platform !== "win32") {
  log("skip: FFmpeg sidecar is only bundled for Windows builds.");
  process.exit(0);
}
if (!process.argv.includes("--force") && TOOLS.every((t) => existsSync(target(t)))) {
  log("FFmpeg sidecar already present.");
  process.exit(0);
}

const text = async (url) => {
  const r = await fetch(url);
  if (!r.ok) throw new Error(`${url} -> HTTP ${r.status}`);
  return (await r.text()).trim();
};

const work = mkdtempSync(join(tmpdir(), "comicut-sidecars-"));
try {
  const [version, shaText] = await Promise.all([text(VERSION_URL), text(SHA_URL)]);
  const expected = shaText.match(/[0-9a-f]{64}/i)?.[0].toLowerCase();
  if (!expected) throw new Error("checksum not found");
  log(`downloading FFmpeg ${version} essentials...`);

  const res = await fetch(ZIP_URL);
  if (!res.ok || !res.body) throw new Error(`download failed: HTTP ${res.status}`);
  const total = Number(res.headers.get("content-length")) || 0;
  const hash = createHash("sha256");
  let done = 0;
  let lastPct = -10;
  const meter = new Transform({
    transform(chunk, _enc, cb) {
      hash.update(chunk);
      done += chunk.length;
      const pct = total ? Math.floor((done / total) * 100) : 0;
      if (pct - lastPct >= 10) {
        lastPct = pct;
        log(`  ${pct}% (${(done / 1048576).toFixed(0)} MB)`);
      }
      cb(null, chunk);
    },
  });
  const zipPath = join(work, "ffmpeg.zip");
  await pipeline(Readable.fromWeb(res.body), meter, createWriteStream(zipPath));

  const actual = hash.digest("hex");
  if (actual !== expected) throw new Error(`SHA-256 mismatch (expected ${expected}, got ${actual})`);
  log("SHA-256 OK, extracting...");

  // Windows 10+ ships bsdtar, which extracts zip archives.
  execFileSync("tar", ["-xf", zipPath, "-C", work], { stdio: "inherit" });
  const files = readdirSync(work, { recursive: true, withFileTypes: true }).filter((e) => e.isFile());

  mkdirSync(OUT_DIR, { recursive: true });
  for (const tool of TOOLS) {
    const hit = files.find((e) => basename(e.name).toLowerCase() === `${tool}.exe`);
    if (!hit) throw new Error(`${tool}.exe not found in archive`);
    copyFileSync(join(hit.parentPath ?? hit.path, hit.name), target(tool));
    log(`-> ${target(tool)}`);
  }
  log(`done (FFmpeg ${version}).`);
} catch (e) {
  console.error(`[sidecars] ERROR: ${e.message}`);
  console.error("[sidecars] Re-run with: npm run sidecars");
  process.exitCode = 1;
} finally {
  rmSync(work, { recursive: true, force: true });
}
