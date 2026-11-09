export type ComicFormat = "WebtoonVertical" | "MangaRTL" | "MangaLTR";

export type PlatformKind = "Webtoon" | "MangaDex" | "VoraToon" | "Custom";

export interface PlatformPreset {
  id: PlatformKind;
  name: string;
  baseUrl: string;
  exampleUrl: string;
  defaultFormat: ComicFormat;
  imageSelector: string;
  refererHeader: string;
  description: string;
}

export interface ScraperRequest {
  url: string;
  platform: PlatformKind;
  comicFormat: ComicFormat;
  customSelector?: string;
  customReferer?: string;
}

export interface ScrapeResult {
  chapterTitle: string;
  sourcePlatform: string;
  sourceUrl: string;
  downloadedPaths: string[];
  extractedPanels: Panel[];
}

export type PanelRole =
  | "Hook"
  | "BuildUp"
  | "AuraCloseup"
  | "Climax"
  | "VerticalPan"
  | "Filler";

export interface BoundingBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Panel {
  id: string;
  index: number;
  imagePath: string;
  previewDataUrl?: string;
  bbox: BoundingBox;
  aspectRatio: number;
  edgeDensity: number;
  contrastScore: number;
  heuristicScore: number;
  aiEpicScore?: number;
  role: PanelRole;
  isVerticalPan: boolean;
  selected: boolean;
  caption?: string;
}

export interface BeatMarker {
  timestampSec: number;
  intensity: number;
  isBassDrop: boolean;
  isSubBassHit: boolean;
}

export interface BeatMap {
  audioPath: string;
  fileName: string;
  durationSec: number;
  bpm: number;
  mainDropSec: number;
  beats: BeatMarker[];
  waveformPeaks: number[];
}

export type EffectType =
  | "ZoomPunch"
  | "CameraShake"
  | "VerticalPanScroll"
  | "ImpactFlashWhite"
  | "ImpactFlashRed"
  | "SlowPushIn";

export interface TimelineClip {
  id: string;
  panelId: string;
  panelIndex: number;
  role: PanelRole;
  startSec: number;
  endSec: number;
  durationSec: number;
  effect: EffectType;
  beatIntensity: number;
  shakeAmplitude: number;
  zoomScale: number;
}

export interface VisionReviewConfig {
  provider: "OpenAI" | "Anthropic" | "HeuristicOnly";
  apiKey?: string;
  /** OpenAI-compatible: https://host/v1 ; Anthropic-compatible: https://host (or .../v1). Empty = official. */
  baseUrl?: string;
  modelName: string;
  maxPanelsToSelect: number;
}

export interface RenderConfig {
  outputPath: string;
  width: number;
  height: number;
  fps: number;
  blurBackground: boolean;
  vignetteStrength: number;
  beatFlashEnabled: boolean;
}
