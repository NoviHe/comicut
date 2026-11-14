export type ComicFormat = "WebtoonVertical" | "MangaRTL" | "MangaLTR";

/**
 * Panel selection & cropping style:
 * - Default: all panels, standard heuristic/AI epic scoring.
 * - FaceCharacter: only panels with a detected character face, speech bubbles cropped out.
 * - EpicScene: only high-action / fight / VFX panels, speech bubbles cropped out.
 */
export type PanelStyle = "Default" | "FaceCharacter" | "EpicScene";

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
  panelStyle?: PanelStyle;
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
  panelStyle?: PanelStyle;
}

export interface WatermarkConfig {
  enabled: boolean;
  text: string;
  font: string;
  fontSize?: number;
  positionX: number; // 0 - 100 (% horizontal)
  positionY: number; // 0 - 100 (% vertical)
  opacity: number; // 0.1 - 1.0 (default: 0.7)
}

export interface RenderConfig {
  outputPath: string;
  width: number;
  height: number;
  fps: number;
  blurBackground: boolean;
  vignetteStrength: number;
  beatFlashEnabled: boolean;
  watermark?: WatermarkConfig | null;
}
