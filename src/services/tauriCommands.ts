import { invoke } from "@tauri-apps/api/core";
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
    imageSelector: "#readerarea img, .reading-content img, .chapter-area img",
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
