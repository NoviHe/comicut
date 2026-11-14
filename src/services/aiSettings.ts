import { PanelStyle, VisionReviewConfig } from "../types/project";

export interface AiSettings {
  provider: VisionReviewConfig["provider"];
  baseUrl: string;
  modelName: string;
  apiKey: string;
}

const KEY = "comicut.ai";
const STYLE_KEY = "comicut.panelStyle";

export function loadPanelStyle(): PanelStyle {
  try {
    const v = localStorage.getItem(STYLE_KEY);
    if (v === "FaceCharacter" || v === "EpicScene" || v === "Default") return v;
  } catch {
    // ignore
  }
  return "Default";
}

export function savePanelStyle(style: PanelStyle): void {
  try {
    localStorage.setItem(STYLE_KEY, style);
  } catch {
    // ignore
  }
}

export const DEFAULT_AI_SETTINGS: AiSettings = {
  provider: "HeuristicOnly",
  baseUrl: "",
  modelName: "",
  apiKey: "",
};

export const defaultModel = (p: AiSettings["provider"]) =>
  p === "Anthropic" ? "claude-sonnet-5-5" : "gpt-4o-mini";

export function loadAiSettings(): AiSettings {
  try {
    return {
      ...DEFAULT_AI_SETTINGS,
      ...JSON.parse(localStorage.getItem(KEY) || "{}"),
    };
  } catch {
    return DEFAULT_AI_SETTINGS;
  }
}

// shortcut: API key stored plaintext in localStorage, move to OS keychain (tauri stronghold) on shared machines.
export const saveAiSettings = (s: AiSettings) =>
  localStorage.setItem(KEY, JSON.stringify(s));

export const toReviewConfig = (
  s: AiSettings,
  maxPanels: number,
  panelStyle: PanelStyle = loadPanelStyle()
): VisionReviewConfig => ({
  provider: s.provider,
  apiKey: s.apiKey.trim() || undefined,
  baseUrl: s.baseUrl.trim() || undefined,
  modelName: s.modelName.trim() || defaultModel(s.provider),
  maxPanelsToSelect: maxPanels,
  panelStyle,
});
