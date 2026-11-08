import { VisionReviewConfig } from "../types/project";

export interface AiSettings {
  provider: VisionReviewConfig["provider"];
  baseUrl: string;
  modelName: string;
  apiKey: string;
}

const KEY = "comicut.ai";

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

export const toReviewConfig = (s: AiSettings, maxPanels: number): VisionReviewConfig => ({
  provider: s.provider,
  apiKey: s.apiKey.trim() || undefined,
  baseUrl: s.baseUrl.trim() || undefined,
  modelName: s.modelName.trim() || defaultModel(s.provider),
  maxPanelsToSelect: maxPanels,
});
