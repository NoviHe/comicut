import React, { useState } from "react";
import {
  Flame,
  Eye,
  CheckSquare,
  Square,
  ArrowUpDown,
  Cpu,
  Sparkles,
} from "lucide-react";
import { Panel, PanelRole, VisionReviewConfig } from "../types/project";
import { reviewEpicPanels } from "../services/tauriCommands";

interface PanelGalleryProps {
  panels: Panel[];
  chapterTitle: string;
  onUpdatePanels: (panels: Panel[]) => void;
  onStatusMessage: (msg: string) => void;
}

const ROLE_BADGE_STYLES: Record<PanelRole, string> = {
  Hook: "bg-sky-500/20 text-sky-300 border-sky-500/40",
  BuildUp: "bg-amber-500/20 text-amber-300 border-amber-500/40",
  AuraCloseup: "bg-purple-500/20 text-purple-300 border-purple-500/40",
  Climax: "bg-rose-500/20 text-rose-300 border-rose-500/40",
  VerticalPan: "bg-emerald-500/20 text-emerald-300 border-emerald-500/40",
  Filler: "bg-zinc-800 text-zinc-400 border-zinc-700",
};

export const PanelGallery: React.FC<PanelGalleryProps> = ({
  panels,
  chapterTitle,
  onUpdatePanels,
  onStatusMessage,
}) => {
  const [provider, setProvider] =
    useState<VisionReviewConfig["provider"]>("HeuristicOnly");
  const [apiKey, setApiKey] = useState<string>("");
  const [isReviewing, setIsReviewing] = useState<boolean>(false);

  const toggleSelect = (id: string) => {
    onUpdatePanels(
      panels.map((p) => (p.id === id ? { ...p, selected: !p.selected } : p))
    );
  };

  const changeRole = (id: string, role: PanelRole) => {
    onUpdatePanels(panels.map((p) => (p.id === id ? { ...p, role } : p)));
  };

  const handleRunEpicReviewer = async () => {
    if (panels.length === 0) return;
    setIsReviewing(true);
    onStatusMessage("Menganalisis panel epic (Heuristik Visual + AI Reviewer)...");
    try {
      const reviewed = await reviewEpicPanels(panels, {
        provider,
        apiKey: apiKey.trim() || undefined,
        modelName:
          provider === "Gemini"
            ? "gemini-2.5-flash"
            : provider === "OpenAI"
            ? "gpt-4o-mini"
            : "llava",
        maxPanelsToSelect: 10,
      });
      onUpdatePanels(reviewed);
      onStatusMessage(
        `Review selesai! ${
          reviewed.filter((p) => p.selected).length
        } panel terbaik dipilih untuk disinkronkan ke beat lagu.`
      );
    } catch (err: any) {
      onStatusMessage(`Gagal review AI: ${err?.message || String(err)}`);
    } finally {
      setIsReviewing(false);
    }
  };

  const selectedCount = panels.filter((p) => p.selected).length;

  return (
    <div className="bg-zinc-900/90 border border-zinc-800 rounded-xl p-4 flex flex-col h-full">
      <div className="flex flex-wrap items-center justify-between gap-2 pb-3 border-b border-zinc-800">
        <div>
          <div className="flex items-center gap-2">
            <Flame className="w-4 h-4 text-rose-400" />
            <h2 className="text-sm font-semibold text-zinc-100">
              2. Smart Panel Reviewer ({selectedCount}/{panels.length} Terpilih)
            </h2>
          </div>
          <p className="text-[11px] text-zinc-400 mt-0.5 truncate">
            {chapterTitle || "Belum ada chapter yang dimuat"}
          </p>
        </div>

        {/* AI / Heuristic Reviewer Bar */}
        <div className="flex items-center gap-2">
          <select
            value={provider}
            onChange={(e) =>
              setProvider(e.target.value as VisionReviewConfig["provider"])
            }
            className="bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200"
          >
            <option value="HeuristicOnly">
              Rust Local Heuristics (Edge + Aspect + Contrast)
            </option>
            <option value="Gemini">Hybrid + Gemini 2.5 Flash Vision</option>
            <option value="OpenAI">Hybrid + OpenAI GPT-4o Vision</option>
            <option value="Ollama">Hybrid + Ollama Local Vision</option>
          </select>

          {provider !== "HeuristicOnly" && provider !== "Ollama" && (
            <input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="API Key..."
              className="w-28 bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200"
            />
          )}

          <button
            type="button"
            disabled={panels.length === 0 || isReviewing}
            onClick={handleRunEpicReviewer}
            className="bg-rose-600 hover:bg-rose-500 disabled:opacity-40 text-white text-xs font-semibold px-3 py-1.5 rounded-lg flex items-center gap-1.5 transition"
          >
            <Sparkles className="w-3.5 h-3.5" />
            {isReviewing ? "Menilai..." : "Auto-Pilih Panel Epic"}
          </button>
        </div>
      </div>

      {/* Daftar Panel */}
      {panels.length === 0 ? (
        <div className="flex-1 flex flex-col items-center justify-center text-center py-12 text-zinc-500">
          <Cpu className="w-8 h-8 mb-2 text-zinc-700" />
          <p className="text-xs font-medium text-zinc-400">
            Belum ada panel yang diekstrak
          </p>
          <p className="text-[11px] text-zinc-600 max-w-xs mt-1">
            Masukkan URL chapter dari Webtoon, MangaDex,{" "}
            <span className="text-zinc-400">voratoon.web.id</span>, atau klik
            tombol <b>Simulasi</b> di kiri untuk menguji pipeline.
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-2 sm:grid-cols-3 gap-2.5 mt-3 overflow-y-auto pr-1 max-h-[360px]">
          {panels.map((panel) => {
            const epicPercent = Math.round(
              (panel.aiEpicScore ?? panel.heuristicScore) * 100
            );
            return (
              <div
                key={panel.id}
                className={`group relative rounded-lg border p-2.5 transition flex flex-col justify-between ${
                  panel.selected
                    ? "bg-zinc-950/90 border-violet-500/60 shadow-sm"
                    : "bg-zinc-950/30 border-zinc-800/60 opacity-60"
                }`}
              >
                <div>
                  <div className="flex items-center justify-between gap-1 mb-1.5">
                    <button
                      type="button"
                      onClick={() => toggleSelect(panel.id)}
                      className="flex items-center gap-1.5 text-xs font-semibold text-zinc-200"
                    >
                      {panel.selected ? (
                        <CheckSquare className="w-3.5 h-3.5 text-violet-400" />
                      ) : (
                        <Square className="w-3.5 h-3.5 text-zinc-600" />
                      )}
                      <span>Panel #{panel.index + 1}</span>
                    </button>

                    <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-zinc-900 text-amber-400 border border-zinc-800">
                      Epic {epicPercent}%
                    </span>
                  </div>

                  {/* Visual Box / Placeholder Representation */}
                  <div className="h-24 rounded bg-gradient-to-br from-zinc-900 via-zinc-950 to-violet-950/30 border border-zinc-800/80 flex flex-col items-center justify-center p-2 text-center relative overflow-hidden">
                    {panel.isVerticalPan && (
                      <span className="absolute top-1 right-1 text-[9px] bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 px-1.5 py-0.5 rounded flex items-center gap-0.5">
                        <ArrowUpDown className="w-2.5 h-2.5" />
                        Long Strip
                      </span>
                    )}
                    <Eye className="w-4 h-4 text-zinc-600 mb-1" />
                    <p className="text-[10px] text-zinc-300 line-clamp-2">
                      {panel.caption ||
                        `${panel.bbox.width}x${panel.bbox.height}px`}
                    </p>
                    <span className="text-[9px] text-zinc-500 font-mono mt-0.5">
                      {panel.bbox.width}×{panel.bbox.height} • Edge{" "}
                      {Math.round(panel.edgeDensity * 100)}%
                    </span>
                  </div>
                </div>

                {/* Role Selector */}
                <div className="mt-2 flex items-center justify-between gap-1">
                  <span className="text-[10px] text-zinc-500">Peran Beat:</span>
                  <select
                    value={panel.role}
                    onChange={(e) =>
                      changeRole(panel.id, e.target.value as PanelRole)
                    }
                    className={`text-[10px] font-medium px-2 py-0.5 rounded border ${
                      ROLE_BADGE_STYLES[panel.role]
                    } bg-zinc-950 focus:outline-none`}
                  >
                    <option value="Hook">Hook (0s - Awal)</option>
                    <option value="BuildUp">BuildUp (Pre-Drop)</option>
                    <option value="AuraCloseup">Aura Closeup</option>
                    <option value="Climax">Climax (Bass Drop!)</option>
                    <option value="VerticalPan">Vertical Pan Scroll</option>
                    <option value="Filler">Filler / Skip</option>
                  </select>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
