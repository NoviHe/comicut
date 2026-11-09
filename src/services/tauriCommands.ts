import { convertFileSrc, invoke } from "@tauri-apps/api/core";
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
} from "../types/project";

const isTauri = () =>
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
}

export interface DepReport {
  deps: DepStatus[];
  system: { cpu: string; threads: number; gpuEncoder?: string; gpus: string[] };
}

export async function checkDependencies(): Promise<DepReport> {
  if (!isTauri()) throw new Error("Cek dependensi membutuhkan runtime Tauri.");
  return await invoke<DepReport>("check_dependencies");
}

export async function updateYtdlp(): Promise<string> {
  if (!isTauri()) throw new Error("Update membutuhkan runtime Tauri.");
  return await invoke<string>("update_ytdlp");
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

export async function defaultOutputPath(): Promise<string> {
  try {
    const { videoDir, join } = await import("@tauri-apps/api/path");
    return await join(await videoDir(), "comicut_montagem.mp4");
  } catch {
    return "comicut_montagem.mp4";
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
