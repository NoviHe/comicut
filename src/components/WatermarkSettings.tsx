import React, { useState } from "react";
import {
  ChevronDown,
  ChevronRight,
  Sparkles,
  Type,
} from "lucide-react";
import { WatermarkConfig } from "../types/project";
import { WATERMARK_FONTS } from "../services/tauriCommands";

interface WatermarkSettingsProps {
  watermark: WatermarkConfig;
  onChange: (updater: (prev: WatermarkConfig) => WatermarkConfig) => void;
}

export const WatermarkSettings: React.FC<WatermarkSettingsProps> = ({
  watermark,
  onChange,
}) => {
  const [expanded, setExpanded] = useState<boolean>(false);

  const update = (partial: Partial<WatermarkConfig>) => {
    onChange((prev) => ({ ...prev, ...partial }));
  };

  return (
    <div className={`card transition-all ${expanded ? "space-y-3" : "py-2.5"}`}>
      {/* Header with Title and Toggle */}
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div
          className="flex items-center gap-2.5 cursor-pointer select-none min-w-0"
          onClick={() => setExpanded(!expanded)}
        >
          <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-violet-500/15 text-violet-400 border border-violet-500/30 shrink-0">
            <Type className="w-4 h-4" />
          </div>
          <div className="min-w-0">
            <div className="flex items-center gap-2 flex-wrap">
              <h2 className="card-title text-sm font-semibold text-white">
                Watermark Video
              </h2>
              {watermark.enabled && watermark.text.trim() ? (
                <>
                  <span className="rounded-full bg-violet-500/20 px-2 py-0.5 text-[10px] font-semibold text-violet-300 truncate max-w-[140px]">
                    {watermark.text}
                  </span>
                  <span className="text-[10px] text-zinc-500 font-mono">
                    {watermark.font} • {Math.round(watermark.opacity * 100)}%
                  </span>
                </>
              ) : (
                <span className="text-[10px] text-zinc-500 font-normal">
                  (Nonaktif)
                </span>
              )}
            </div>
            {expanded && (
              <p className="text-[11px] text-zinc-400 mt-0.5">
                Teks nama channel/brand otomatis tersemat di video montagem
              </p>
            )}
          </div>
        </div>

        <div className="flex items-center gap-3 shrink-0">
          <label className="flex items-center gap-2 cursor-pointer text-xs text-zinc-300 select-none">
            <input
              type="checkbox"
              checked={watermark.enabled}
              onChange={(e) => update({ enabled: e.target.checked })}
              className="rounded border-zinc-700 bg-zinc-800 text-violet-500 focus:ring-0 w-4 h-4 cursor-pointer"
            />
            <span>{watermark.enabled ? "Aktif" : "Mati"}</span>
          </label>

          <button
            type="button"
            onClick={() => setExpanded(!expanded)}
            className="btn-ghost p-1 text-zinc-400 hover:text-white"
            title={expanded ? "Tutup panel" : "Buka panel"}
          >
            {expanded ? (
              <ChevronDown className="w-4 h-4" />
            ) : (
              <ChevronRight className="w-4 h-4" />
            )}
          </button>
        </div>
      </div>

      {/* Expanded Settings Content */}
      {expanded && (
        <div className="border-t border-white/[0.06] pt-3 space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-12 gap-4">
            {/* Left Section: Text & Font Preset */}
            <div className="md:col-span-6 space-y-3">
              {/* Watermark Text Input */}
              <div>
                <label className="block text-xs font-medium text-zinc-300 mb-1">
                  Teks Watermark
                </label>
                <input
                  type="text"
                  value={watermark.text}
                  onChange={(e) => update({ text: e.target.value })}
                  placeholder="@ComiCut"
                  className="field text-xs font-mono"
                />
              </div>

              {/* Font Preset Selection */}
              <div>
                <div className="flex items-center justify-between mb-1.5">
                  <label className="text-xs font-medium text-zinc-300">
                    Preset Font (Popular, Bold &amp; Lebar)
                  </label>
                  <span className="text-[10px] font-mono text-violet-400 bg-violet-500/10 px-1.5 py-0.5 rounded border border-violet-500/20">
                    {watermark.font}
                  </span>
                </div>
                <div className="grid grid-cols-2 gap-2">
                  {WATERMARK_FONTS.map((f) => {
                    const isSelected = watermark.font === f.id;
                    return (
                      <button
                        key={f.id}
                        type="button"
                        onClick={() => update({ font: f.id })}
                        className={`flex flex-col text-left px-2.5 py-2 rounded-xl border transition ${
                          isSelected
                            ? "border-violet-500 bg-violet-500/15 text-white shadow-sm shadow-violet-500/10"
                            : "border-white/[0.06] bg-zinc-900/50 text-zinc-400 hover:text-zinc-200 hover:border-white/15"
                        }`}
                      >
                        <span
                          className="text-xs truncate tracking-wide"
                          style={{
                            fontFamily: f.fontFamily,
                            fontWeight: f.fontWeight,
                          }}
                        >
                          {f.name}
                        </span>
                        <span className="text-[9px] text-zinc-500 truncate leading-tight mt-0.5">
                          {f.tag}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>
            </div>

            {/* Right Section: Position, Opacity, Font Size */}
            <div className="md:col-span-6 space-y-3">
              {/* Position Presets & Sliders */}
              <div>
                <label className="block text-xs font-medium text-zinc-300 mb-1.5">
                  Posisi Watermark
                </label>
                {/* Quick Buttons */}
                <div className="grid grid-cols-3 gap-1.5 mb-2.5">
                  {[
                    { label: "Bawah (Safe)", x: 50, y: 82 },
                    { label: "Bawah Kiri", x: 15, y: 82 },
                    { label: "Bawah Kanan", x: 85, y: 82 },
                    { label: "Tengah", x: 50, y: 50 },
                    { label: "Atas Kanan", x: 85, y: 10 },
                    { label: "Atas Tengah", x: 50, y: 10 },
                  ].map((pos) => {
                    const isActive =
                      watermark.positionX === pos.x && watermark.positionY === pos.y;
                    return (
                      <button
                        key={pos.label}
                        type="button"
                        onClick={() =>
                          update({
                            positionX: pos.x,
                            positionY: pos.y,
                          })
                        }
                        className={`text-[10px] py-1 px-2 rounded-lg border transition truncate ${
                          isActive
                            ? "border-violet-500 bg-violet-500/20 text-white font-semibold"
                            : "border-white/[0.06] bg-zinc-900/40 text-zinc-400 hover:text-zinc-200 hover:border-white/10"
                        }`}
                      >
                        {pos.label}
                      </button>
                    );
                  })}
                </div>

                {/* Fine Sliders */}
                <div className="space-y-2 bg-zinc-900/40 border border-white/[0.04] rounded-xl p-2.5">
                  <div>
                    <div className="flex items-center justify-between text-[10px] text-zinc-400 mb-1">
                      <span>Posisi X (Horizontal)</span>
                      <span className="font-mono text-zinc-300">{watermark.positionX}%</span>
                    </div>
                    <input
                      type="range"
                      min="5"
                      max="95"
                      step="1"
                      value={watermark.positionX}
                      onChange={(e) => update({ positionX: Number(e.target.value) })}
                      className="w-full accent-violet-500 cursor-pointer h-1.5 bg-zinc-800 rounded-lg"
                    />
                  </div>

                  <div>
                    <div className="flex items-center justify-between text-[10px] text-zinc-400 mb-1">
                      <span>Posisi Y (Vertikal)</span>
                      <span className="font-mono text-zinc-300">
                        {watermark.positionY}% {watermark.positionY === 82 ? "(Safe Margin Bawah)" : ""}
                      </span>
                    </div>
                    <input
                      type="range"
                      min="5"
                      max="95"
                      step="1"
                      value={watermark.positionY}
                      onChange={(e) => update({ positionY: Number(e.target.value) })}
                      className="w-full accent-violet-500 cursor-pointer h-1.5 bg-zinc-800 rounded-lg"
                    />
                    <p className="text-[9px] text-zinc-500 italic mt-0.5">
                      *Default 82% menyisakan space di bawah agar tidak tertutup caption/UI TikTok &amp; Reels.
                    </p>
                  </div>
                </div>
              </div>

              {/* Opacity & Font Size Controls */}
              <div className="grid grid-cols-2 gap-3">
                {/* Opacity Slider */}
                <div className="bg-zinc-900/40 border border-white/[0.04] rounded-xl p-2.5 space-y-1.5">
                  <div className="flex items-center justify-between text-[10px] text-zinc-400">
                    <span>Opacity</span>
                    <span className="font-mono font-semibold text-violet-300">
                      {Math.round(watermark.opacity * 100)}% {watermark.opacity === 0.7 ? "(Def)" : ""}
                    </span>
                  </div>
                  <input
                    type="range"
                    min="0.1"
                    max="1.0"
                    step="0.05"
                    value={watermark.opacity}
                    onChange={(e) => update({ opacity: Number(e.target.value) })}
                    className="w-full accent-violet-500 cursor-pointer h-1.5 bg-zinc-800 rounded-lg"
                  />
                </div>

                {/* Font Size Slider */}
                <div className="bg-zinc-900/40 border border-white/[0.04] rounded-xl p-2.5 space-y-1.5">
                  <div className="flex items-center justify-between text-[10px] text-zinc-400">
                    <span>Ukuran Font</span>
                    <span className="font-mono text-zinc-300">{watermark.fontSize || 48}px</span>
                  </div>
                  <input
                    type="range"
                    min="24"
                    max="84"
                    step="2"
                    value={watermark.fontSize || 48}
                    onChange={(e) => update({ fontSize: Number(e.target.value) })}
                    className="w-full accent-violet-500 cursor-pointer h-1.5 bg-zinc-800 rounded-lg"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
