import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  BeatMap,
  ComicFormat,
  Panel,
  PlatformPreset,
  RenderConfig,
  ScrapeResult,
  ScraperRequest,
  TimelineClip,
  VisionReviewConfig,
  WatermarkConfig,
} from "../types/project";

export const isTauri = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Real image URL for a panel (sliced PNG on disk); undefined for sample/browser panels. */
export const panelImageSrc = (panel: Panel): string | undefined =>
  isTauri() && panel.imagePath && !panel.imagePath.startsWith("sample://")
    ? convertFileSrc(panel.imagePath)
    : undefined;

export const DEFAULT_PLATFORM_PRESETS: PlatformPreset[] = [
  {
    id: "Webtoon",
    name: "LINE Webtoon",
    baseUrl: "https://www.webtoons.com",
    exampleUrl:
      "https://www.webtoons.com/id/action/solo-leveling/episode-1/viewer?title_no=123&episode_no=1",
    defaultFormat: "WebtoonVertical",
    imageSelector: "#_imageList img._images",
    refererHeader: "https://www.webtoons.com/",
    description:
      "Ekstrak gambar vertikal dari LINE Webtoon otomatis dengan header Referer anti-403.",
  },
  {
    id: "MangaDex",
    name: "MangaDex (API v5)",
    baseUrl: "https://mangadex.org",
    exampleUrl: "https://mangadex.org/chapter/801513ba-a712-498c-8f57-cae55b38cc92",
    defaultFormat: "MangaRTL",
    imageSelector: "mangadex-at-home-api",
    refererHeader: "https://mangadex.org/",
    description:
      "Mengambil halaman resolusi penuh langsung melalui endpoint resmi MangaDex At-Home Server.",
  },
  {
    id: "VoraToon",
    name: "VoraToon (voratoon.web.id)",
    baseUrl: "https://voratoon.web.id",
    exampleUrl: "https://voratoon.web.id/judul-manhwa-chapter-01/",
    defaultFormat: "WebtoonVertical",
    imageSelector:
      '#readerarea img, .reading-content img, .chapter-area img, img[class*="chapterPage"]',
    refererHeader: "https://voratoon.web.id/",
    description:
      "Preset khusus untuk https://voratoon.web.id/ (mendukung ekstraksi #readerarea, lazy-src, dan ts_reader JSON).",
  },
  {
    id: "Custom",
    name: "Website Komik Lainnya (Custom URL)",
    baseUrl: "https://",
    exampleUrl: "https://voratoon.web.id/chapter-baru/",
    defaultFormat: "WebtoonVertical",
    imageSelector: "#readerarea img, .reading-content img, article img",
    refererHeader: "",
    description:
      "Universal Scraper untuk situs komik/manhwa/manga manapun. Mendeteksi gambar chapter & script reader secara otomatis.",
  },
];

export async function getPlatformPresets(): Promise<PlatformPreset[]> {
  if (!isTauri()) return DEFAULT_PLATFORM_PRESETS;
  try {
    return await invoke<PlatformPreset[]>("get_platform_presets");
  } catch {
    return DEFAULT_PLATFORM_PRESETS;
  }
}

export async function scrapeComicChapter(
  request: ScraperRequest
): Promise<ScrapeResult> {
  if (!isTauri()) {
    throw new Error(
      "Fitur Scraper membutuhkan runtime Tauri Rust (jalankan dengan npm run tauri dev)."
    );
  }
  return await invoke<ScrapeResult>("scrape_comic_chapter", { request });
}

export async function importLocalChapter(
  folderPath: string,
  comicFormat: ComicFormat
): Promise<Panel[]> {
  if (!isTauri()) {
    throw new Error(
      "Import folder lokal membutuhkan runtime Tauri Rust (jalankan dengan npm run tauri dev)."
    );
  }
  return await invoke<Panel[]>("import_local_chapter", {
    folderPath,
    comicFormat,
  });
}

export async function reviewEpicPanels(
  panels: Panel[],
  config: VisionReviewConfig
): Promise<Panel[]> {
  if (!isTauri()) {
    return panels;
  }
  return await invoke<Panel[]>("review_epic_panels", { panels, config });
}

export async function analyzeAudioBeats(audioPath: string): Promise<BeatMap> {
  if (!isTauri()) {
    throw new Error("Analisis audio membutuhkan runtime Tauri Rust.");
  }
  return await invoke<BeatMap>("analyze_audio_beats", { audioPath });
}

/** Link (YouTube/TikTok/SoundCloud...) or search text -> local mp3 path via yt-dlp. */
export async function downloadAudio(source: string): Promise<string> {
  if (!isTauri()) {
    throw new Error("Download audio membutuhkan runtime Tauri Rust.");
  }
  return await invoke<string>("download_audio", { source });
}

export async function generateAutoTimeline(
  panels: Panel[],
  beatMap: BeatMap
): Promise<TimelineClip[]> {
  if (!isTauri()) {
    return [];
  }
  return await invoke<TimelineClip[]>("generate_auto_timeline", {
    panels,
    beatMap,
  });
}

export async function renderMontagemVideo(
  panels: Panel[],
  beatMap: BeatMap,
  timeline: TimelineClip[],
  config: RenderConfig
): Promise<string> {
  if (!isTauri()) {
    throw new Error("Render video FFmpeg membutuhkan runtime Tauri Rust.");
  }
  return await invoke<string>("render_montagem_video", {
    panels,
    beatMap,
    timeline,
    config,
  });
}

/** Playable URL for an analysed audio file (copied into the asset scope by the backend). */
export const audioSrc = (path: string): string | undefined =>
  isTauri() && path && !path.startsWith("sample://")
    ? convertFileSrc(path)
    : undefined;

export const isTauriRuntime = isTauri;

export interface DepStatus {
  id: string;
  name: string;
  purpose: string;
  required: boolean;
  found: boolean;
  version?: string;
  path?: string;
  outdated: boolean;
  installHint: string;
  /** Can be installed/updated from inside the app. */
  installable: boolean;
  /** Resolved binary is the app-managed copy (%LOCALAPPDATA%\ComiCut\bin). */
  managed: boolean;
  /** Resolved binary is the sidecar shipped with the installer. */
  bundled: boolean;
}

export interface DepReport {
  deps: DepStatus[];
  system: { cpu: string; threads: number; gpuEncoder?: string; gpus: string[] };
}

export interface DepUpdate {
  id: string;
  installed?: string;
  latest?: string;
  updateAvailable: boolean;
  error?: string;
}

export interface DepProgress {
  id: string;
  stage: "download" | "verify" | "install" | "done";
  downloaded: number;
  total?: number;
}

export async function checkDependencies(): Promise<DepReport> {
  if (!isTauri()) throw new Error("Cek dependensi membutuhkan runtime Tauri.");
  return await invoke<DepReport>("check_dependencies");
}

/** Latest official versions vs installed (requires internet). */
export async function checkDependencyUpdates(): Promise<DepUpdate[]> {
  if (!isTauri()) throw new Error("Cek update membutuhkan runtime Tauri.");
  return await invoke<DepUpdate[]>("check_dependency_updates");
}

/** Download + SHA-256 verify + install the latest build of a tool into the app-managed folder. */
export async function installDependency(id: string): Promise<string> {
  if (!isTauri()) throw new Error("Install membutuhkan runtime Tauri.");
  return await invoke<string>("install_dependency", { id });
}

export async function onDepProgress(cb: (p: DepProgress) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return await listen<DepProgress>("dep-progress", (e) => cb(e.payload));
}

/** AI picks panel + effect per beat-fitted cut. Throws on API failure so the caller can fall back to local. */
export async function aiDirectTimeline(
  panels: Panel[],
  beatMap: BeatMap,
  config: VisionReviewConfig
): Promise<TimelineClip[]> {
  if (!isTauri()) throw new Error("AI director membutuhkan runtime Tauri.");
  return await invoke<TimelineClip[]>("ai_direct_timeline", { panels, beatMap, config });
}

const DEFAULT_EXPORT_DIR_KEY = "comicut:default_export_dir";

export function getDefaultExportDir(): string | null {
  try {
    return localStorage.getItem(DEFAULT_EXPORT_DIR_KEY);
  } catch {
    return null;
  }
}

export function setDefaultExportDir(path: string): void {
  try {
    localStorage.setItem(DEFAULT_EXPORT_DIR_KEY, path);
  } catch (err) {
    console.warn("Failed to save default export directory:", err);
  }
}

export function clearDefaultExportDir(): void {
  try {
    localStorage.removeItem(DEFAULT_EXPORT_DIR_KEY);
  } catch (err) {
    console.warn("Failed to clear default export directory:", err);
  }
}

export async function getSystemDefaultExportDir(): Promise<string> {
  try {
    const { videoDir } = await import("@tauri-apps/api/path");
    return await videoDir();
  } catch {
    return "";
  }
}

export async function showInFolder(path: string): Promise<void> {
  if (!isTauri()) return;
  await invoke("show_in_folder", { path });
}

export async function restartApp(): Promise<void> {
  if (!isTauri()) return;
  await invoke("app_restart");
}

/**
 * Menghasilkan nama file render default dengan awalan "comicut_render_"
 * serta timestamp lengkap berisi tanggal, bulan, tahun, jam, menit, dan detik
 * agar tidak tertimpa hasil render sebelumnya.
 * Format: comicut_render_YYYY-MM-DD_HH-mm-ss.mp4
 * Contoh: comicut_render_2026-11-10_18-45-00.mp4
 */
export function generateDefaultRenderFilename(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  const hours = String(now.getHours()).padStart(2, "0");
  const minutes = String(now.getMinutes()).padStart(2, "0");
  const seconds = String(now.getSeconds()).padStart(2, "0");

  return `comicut_render_${year}-${month}-${day}_${hours}-${minutes}-${seconds}.mp4`;
}

export async function defaultOutputPath(filename?: string): Promise<string> {
  const name = filename || generateDefaultRenderFilename();
  try {
    const customDir = getDefaultExportDir();
    const { videoDir, join } = await import("@tauri-apps/api/path");
    const dir = customDir || (await videoDir());
    return await join(dir, name);
  } catch {
    return name;
  }
}

/** AI proposes YouTube search queries for songs that fit the panels' mood. */
export async function suggestSongQueries(
  panels: Panel[],
  config: VisionReviewConfig
): Promise<string[]> {
  if (!isTauri()) throw new Error("Saran lagu membutuhkan runtime Tauri.");
  return await invoke<string[]>("suggest_songs", { panels, config });
}

export async function minimizeWindow(): Promise<void> {
  if (!isTauri()) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  } catch {
    await invoke("window_minimize");
  }
}

export async function toggleMaximizeWindow(): Promise<void> {
  if (!isTauri()) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().toggleMaximize();
  } catch {
    await invoke("window_toggle_maximize");
  }
}

export async function closeWindow(): Promise<void> {
  if (!isTauri()) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  } catch {
    await invoke("window_close");
  }
}

export interface DeviceDiagnostics {
  appName: string;
  appVersion: string;
  osName: string;
  osBuild: string;
  kernel: string;
  cpuModel: string;
  architecture: string;
  cpuCoresPhysical: number;
  cpuCoresLogical: number;
  totalMemoryGb: number;
  gpus: string[];
}

export async function getDeviceDiagnostics(): Promise<DeviceDiagnostics> {
  if (!isTauri()) {
    return {
      appName: "ComiCut Studio",
      appVersion: "0.1.0",
      osName: "Windows 11",
      osBuild: "26200",
      kernel: "26200",
      cpuModel: "AMD Ryzen 5 3600XT 6-Core Processor",
      architecture: "x86_64",
      cpuCoresPhysical: 6,
      cpuCoresLogical: 12,
      totalMemoryGb: 16,
      gpus: ["MuMuPlayer Virtual Display Adapter", "NVIDIA GeForce RTX 2060"],
    };
  }
  return await invoke<DeviceDiagnostics>("get_device_diagnostics");
}

/* ============================================================================
 * WATERMARK PRESETS & PERSISTENCE
 * ============================================================================ */

export interface WatermarkFontPreset {
  id: string;
  name: string;
  fontFamily: string;
  fontWeight: string;
  tag: string;
}

export const WATERMARK_FONTS: WatermarkFontPreset[] = [
  {
    id: "Impact",
    name: "Impact",
    fontFamily: "Impact, sans-serif",
    fontWeight: "normal",
    tag: "Super Bold • Shorts/TikTok",
  },
  {
    id: "Arial Black",
    name: "Arial Black",
    fontFamily: "'Arial Black', Gadget, sans-serif",
    fontWeight: "900",
    tag: "Ultra Bold • Modern & Bersih",
  },
  {
    id: "Verdana Bold",
    name: "Verdana Bold",
    fontFamily: "Verdana, Geneva, sans-serif",
    fontWeight: "bold",
    tag: "Lebar • Ekstra Jelas",
  },
  {
    id: "Trebuchet MS Bold",
    name: "Trebuchet MS Bold",
    fontFamily: "'Trebuchet MS', Helvetica, sans-serif",
    fontWeight: "bold",
    tag: "Elegan & Tegas",
  },
  {
    id: "Segoe UI Black",
    name: "Segoe UI Black",
    fontFamily: "'Segoe UI', Tahoma, sans-serif",
    fontWeight: "900",
    tag: "Sleek • Solid",
  },
  {
    id: "Tahoma Bold",
    name: "Tahoma Bold",
    fontFamily: "Tahoma, Verdana, sans-serif",
    fontWeight: "bold",
    tag: "Kontras Tinggi & Padat",
  },
];

const WATERMARK_CONFIG_KEY = "comicut:watermark_config";

export const DEFAULT_WATERMARK_CONFIG: WatermarkConfig = {
  enabled: true,
  text: "@ComiCut",
  font: "Impact",
  fontSize: 48,
  positionX: 50,
  positionY: 82, // Di bawah dengan space aman untuk UI TikTok / Reels
  opacity: 0.7,  // Default 70%
};

export function getStoredWatermarkConfig(): WatermarkConfig {
  try {
    const raw = localStorage.getItem(WATERMARK_CONFIG_KEY);
    if (!raw) return DEFAULT_WATERMARK_CONFIG;
    return { ...DEFAULT_WATERMARK_CONFIG, ...JSON.parse(raw) };
  } catch {
    return DEFAULT_WATERMARK_CONFIG;
  }
}

export function saveStoredWatermarkConfig(cfg: WatermarkConfig): void {
  try {
    localStorage.setItem(WATERMARK_CONFIG_KEY, JSON.stringify(cfg));
  } catch (err) {
    console.warn("Failed to save watermark config:", err);
  }
}
