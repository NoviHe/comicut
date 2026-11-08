import React, { useState } from "react";
import { X } from "lucide-react";
import {
  AiSettings,
  defaultModel,
  saveAiSettings,
} from "../services/aiSettings";

interface Props {
  settings: AiSettings;
  onSave: (s: AiSettings) => void;
  onClose: () => void;
}

const field =
  "w-full bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 font-mono";

export const AiSettingsModal: React.FC<Props> = ({
  settings,
  onSave,
  onClose,
}) => {
  const [s, setS] = useState<AiSettings>(settings);
  const set = <K extends keyof AiSettings>(k: K, v: AiSettings[K]) =>
    setS((p) => ({ ...p, [k]: v }));
  const ai = s.provider !== "HeuristicOnly";

  return (
    <div
      className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4"
      onClick={onClose}
    >
      <div
        className="w-full max-w-md card !bg-zinc-900 p-5 space-y-3"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-zinc-100">
            Pengaturan AI Vision
          </h2>
          <button type="button" onClick={onClose} aria-label="Tutup">
            <X className="w-4 h-4 text-zinc-400" />
          </button>
        </div>

        <label className="block text-[11px] text-zinc-400">
          Mode reviewer
          <select
            value={s.provider}
            onChange={(e) =>
              set("provider", e.target.value as AiSettings["provider"])
            }
            className={`${field} mt-1`}
          >
            <option value="HeuristicOnly">
              Rust Local Heuristics (tanpa AI)
            </option>
            <option value="OpenAI">AI Vision — OpenAI-compatible</option>
            <option value="Anthropic">AI Vision — Anthropic-compatible</option>
          </select>
        </label>

        {ai && (
          <>
            <label className="block text-[11px] text-zinc-400">
              Base URL
              <input
                value={s.baseUrl}
                onChange={(e) => set("baseUrl", e.target.value)}
                placeholder={
                  s.provider === "Anthropic"
                    ? "https://api.anthropic.com"
                    : "https://api.openai.com/v1  (Ollama: http://localhost:11434/v1)"
                }
                className={`${field} mt-1`}
              />
            </label>
            <label className="block text-[11px] text-zinc-400">
              Model (harus mendukung vision)
              <input
                value={s.modelName}
                onChange={(e) => set("modelName", e.target.value)}
                placeholder={defaultModel(s.provider)}
                className={`${field} mt-1`}
              />
            </label>
            <label className="block text-[11px] text-zinc-400">
              API Key (disimpan lokal di perangkat ini)
              <input
                type="password"
                value={s.apiKey}
                onChange={(e) => set("apiKey", e.target.value)}
                className={`${field} mt-1`}
              />
            </label>
          </>
        )}

        <button
          type="button"
          onClick={() => {
            saveAiSettings(s);
            onSave(s);
            onClose();
          }}
          className="w-full bg-violet-600 hover:bg-violet-500 text-white text-xs font-semibold py-2 rounded-lg transition"
        >
          Simpan
        </button>
      </div>
    </div>
  );
};
