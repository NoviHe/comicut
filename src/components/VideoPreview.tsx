import React, { useEffect, useState } from "react";
import {
  ChevronDown,
  ChevronRight,
  Download,
  FolderOpen,
  Loader2,
  Pause,
  Play,
  RotateCw,
  Smartphone,
  Type,
  Volume2,
  VolumeX,
  Zap,
} from "lucide-react";
import { BeatMap, Panel, RenderConfig, TimelineClip, WatermarkConfig } from "../types/project";
import {
  defaultOutputPath,
  getStoredWatermarkConfig,
  panelImageSrc,
  renderMontagemVideo,
  saveStoredWatermarkConfig,
  WATERMARK_FONTS,
} from "../services/tauriCommands";
import { Playback } from "../services/usePlayback";

interface VideoPreviewProps {
  panels: Panel[];
  beatMap: BeatMap | null;
  timeline: TimelineClip[];
  playback: Playback;
  onStatusMessage: (msg: string) => void;
  onRenderSuccess?: (outputPath: string) => void;
}

export const VideoPreview: React.FC<VideoPreviewProps> = ({
  panels,
  beatMap,
  timeline,
  playback,
  onStatusMessage,
  onRenderSuccess,
}) => {
  const [outputPath, setOutputPath] = useState<string>("");
  const [isRendering, setIsRendering] = useState<boolean>(false);
  const [watermark, setWatermark] = useState<WatermarkConfig>(() => getStoredWatermarkConfig());
  const [watermarkExpanded, setWatermarkExpanded] = useState<boolean>(false);
  const currentTime = playback.time;

  const updateWatermark = (updater: (prev: WatermarkConfig) => WatermarkConfig) => {
    setWatermark((prev) => {
      const next = updater(prev);
      saveStoredWatermarkConfig(next);
      return next;
    });
  };

  useEffect(() => {
    defaultOutputPath().then(setOutputPath);
  }, []);

  const totalDuration =
    timeline.length > 0
      ? timeline[timeline.length - 1].endSec
      : beatMap?.durationSec || 9.0;

  const activeClip =
    timeline.find((c) => currentTime >= c.startSec && currentTime < c.endSec) ||
    timeline[0];
  const activePanel = activeClip
    ? panels.find((p) => p.id === activeClip.panelId) || panels[0]
    : panels[0];
  const activeSrc = activePanel ? panelImageSrc(activePanel) : undefined;

  const progress = activeClip
    ? Math.min(
        1,
        Math.max(
          0,
          (currentTime - activeClip.startSec) /
            Math.max(0.01, activeClip.endSec - activeClip.startSec)
        )
      )
    : 0;

  // Mirrors core/video_renderer/effects.rs (eye-friendly): squared ease-out impact, capped zoom/shake, soft flash.
  const d0 = Math.max(0, 1 - progress / 0.55);
  const decay = d0 * d0;
  const clipSec = activeClip ? activeClip.endSec - activeClip.startSec : 0;
  let scale = 1.03;
  let shiftX = 0;
  let shiftY = 0;
  if (activeClip) {
    const z = Math.min(0.08, Math.max(0, (activeClip.zoomScale || 1) - 1));
    if (activeClip.effect === "SlowPushIn") scale = 1 + 0.05 * (progress * progress * (3 - 2 * progress));
    else if (activeClip.effect === "ZoomPunch") scale = 1 + z * decay;
    else if (activeClip.effect === "CameraShake") {
      const w = 2 * Math.PI * 7;
      const t = progress * clipSec;
      const amp = Math.min(8, activeClip.shakeAmplitude) * decay * (270 / 1080);
      shiftX = Math.sin(t * w) * amp;
      shiftY = Math.cos(t * w * 0.8 + 1.3) * amp * 0.6;
    }
  }
  const flash =
    activeClip?.effect === "ImpactFlashWhite" || activeClip?.effect === "ImpactFlashRed"
      ? (36 / 255) * decay
      : 0;
  const flashColor = activeClip?.effect === "ImpactFlashRed" ? "bg-red-500" : "bg-white";

  // Long strip: fit width, scroll top to bottom (translate % is of the image's own height).
  const panRatio =
    activePanel && activePanel.bbox.height > 0
      ? (16 / 9) * (activePanel.bbox.width / activePanel.bbox.height)
      : 1;
  const doPan =
    !!activePanel &&
    (activeClip?.effect === "VerticalPanScroll" || activePanel.isVerticalPan) &&
    panRatio < 1;

  const isBeatFlash =
    beatMap?.beats.some(
      (b) => b.isSubBassHit && currentTime >= b.timestampSec && currentTime - b.timestampSec < 0.1
    ) ?? false;

  const handleBrowseOutput = async () => {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const selected = await save({
        defaultPath: outputPath,
        filters: [{ name: "MP4 Video (*.mp4)", extensions: ["mp4"] }],
      });
      if (selected) {
        setOutputPath(selected);
      }
    } catch (err) {
      console.warn("Gagal membuka dialog simpan file:", err);
    }
  };

  const handleRenderMp4 = async () => {
    if (timeline.length === 0 || !beatMap) return;
    setIsRendering(true);
    onStatusMessage(`Merender video 1080x1920 (60fps) ke ${outputPath}...`);
    try {
      const config: RenderConfig = {
        outputPath,
        width: 1080,
        height: 1920,
        fps: 60,
        blurBackground: true,
        vignetteStrength: 0.35,
        beatFlashEnabled: true,
        watermark: watermark.enabled && watermark.text.trim() ? watermark : null,
      };
      const saved = await renderMontagemVideo(panels, beatMap, timeline, config);
      onStatusMessage(`Render selesai! Video tersimpan di: ${saved}`);
      onRenderSuccess?.(saved);
      // Perbarui nama file output dengan timestamp baru untuk render berikutnya
      defaultOutputPath().then(setOutputPath);
    } catch (err: any) {
      onStatusMessage(`Render gagal: ${err?.message || String(err)}`);
    } finally {
      setIsRendering(false);
    }
  };

  return (
    <div className="card flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="card-title">
          <span className="step">5</span>
          <Smartphone className="w-4 h-4 text-violet-400" />
          Preview 9:16
        </h2>
        <span className="text-[11px] font-mono text-zinc-500">1080×1920 • 60 FPS</span>
      </div>

      <div className="flex flex-1 items-center justify-center">
        <div
          className={`relative aspect-[9/16] w-full max-w-[290px] overflow-hidden rounded-[28px] border-2 bg-black shadow-2xl transition-[border-color,box-shadow] duration-200 ${
            isBeatFlash
              ? "border-rose-400/70 shadow-[0_0_24px_rgba(244,63,94,0.25)]"
              : "border-zinc-700 shadow-black/60"
          }`}
        >
          {activeSrc ? (
            <>
              <img
                src={activeSrc}
                alt=""
                className="absolute inset-0 h-full w-full scale-125 object-cover opacity-40 blur-2xl"
              />
              {doPan ? (
                <img
                  src={activeSrc}
                  alt={`Panel ${activePanel!.index + 1}`}
                  className="absolute left-0 top-0 h-auto w-full"
                  style={{ transform: `translateY(${-progress * (1 - panRatio) * 100}%)` }}
                />
              ) : (
                <img
                  src={activeSrc}
                  alt={`Panel ${activePanel!.index + 1}`}
                  className="absolute inset-0 h-full w-full object-contain"
                  style={{
                    transform: `translate(${shiftX}px, ${shiftY}px) scale(${scale})`,
                  }}
                />
              )}
            </>
          ) : (
            <div className="absolute inset-0 grid place-items-center p-6 text-center text-xs text-zinc-500">
              {activePanel
                ? activePanel.caption || `Panel #${activePanel.index + 1}`
                : "Muat panel & lagu, lalu sinkronkan timeline untuk melihat preview"}
            </div>
          )}

          {flash > 0 && (
            <div
              className={`pointer-events-none absolute inset-0 ${flashColor}`}
              style={{ opacity: flash }}
            />
          )}

          {/* Live Watermark Overlay */}
          {watermark.enabled && watermark.text.trim() && (
            <div
              className="pointer-events-none absolute select-none text-center whitespace-nowrap transition-all duration-75"
              style={{
                left: `${watermark.positionX}%`,
                top: `${watermark.positionY}%`,
                transform: "translate(-50%, -50%)",
                opacity: watermark.opacity,
                fontFamily:
                  WATERMARK_FONTS.find((f) => f.id === watermark.font)?.fontFamily ||
                  "Impact, sans-serif",
                fontWeight:
                  WATERMARK_FONTS.find((f) => f.id === watermark.font)?.fontWeight ||
                  "normal",
                fontSize: `${Math.max(10, Math.round((watermark.fontSize || 48) * (290 / 1080)))}px`,
                color: "white",
                textShadow:
                  "0 1px 3px rgba(0,0,0,0.95), 0 0 4px rgba(0,0,0,0.95), 0 0 1px black",
                zIndex: 25,
                letterSpacing: "0.02em",
              }}
            >
              {watermark.text}
            </div>
          )}

          <div className="absolute inset-x-0 top-0 z-10 flex items-center justify-between p-3 text-[10px]">
            <span className="rounded-full border border-white/10 bg-black/60 px-2 py-0.5 font-medium text-zinc-200 backdrop-blur">
              {activeClip ? activeClip.effect : "Ready"}
            </span>
            {isBeatFlash && (
              <span className="flex items-center gap-0.5 rounded-full bg-rose-600 px-2 py-0.5 font-bold text-white">
                <Zap className="h-2.5 w-2.5" />
                BEAT
              </span>
            )}
          </div>

          <div className="absolute inset-x-0 bottom-0 z-10 space-y-1.5 bg-gradient-to-t from-black/80 to-transparent p-3 pt-8">
            <div className="flex justify-between font-mono text-[10px] text-zinc-300">
              <span>{currentTime.toFixed(2)}s</span>
              <span>{totalDuration.toFixed(2)}s</span>
            </div>
            <div
              className="h-1.5 w-full cursor-pointer overflow-hidden rounded-full bg-white/15"
              onClick={(e) => {
                const r = e.currentTarget.getBoundingClientRect();
                playback.seek(((e.clientX - r.left) / r.width) * totalDuration);
              }}
            >
              <div
                style={{ width: `${Math.min(100, (currentTime / Math.max(0.1, totalDuration)) * 100)}%` }}
                className="h-full bg-gradient-to-r from-violet-500 to-rose-500"
              />
            </div>
          </div>
        </div>
      </div>

      <div className="space-y-2.5 border-t border-white/[0.06] pt-3">
        <button type="button" onClick={playback.toggle} className="btn-ghost w-full">
          {playback.playing ? (
            <>
              <Pause className="w-3.5 h-3.5" /> Pause
            </>
          ) : (
            <>
              <Play className="w-3.5 h-3.5" /> Putar dengan Suara
            </>
          )}
          {playback.hasAudio ? (
            <Volume2 className="ml-auto w-3.5 h-3.5 text-emerald-400" />
          ) : (
            <VolumeX className="ml-auto w-3.5 h-3.5 text-zinc-500" />
          )}
        </button>

        {/* Watermark Video Settings Card */}
        <div className="rounded-xl border border-white/[0.08] bg-zinc-900/60 overflow-hidden">
          <button
            type="button"
            onClick={() => setWatermarkExpanded(!watermarkExpanded)}
            className="w-full flex items-center justify-between p-2.5 text-left hover:bg-white/[0.02] transition"
          >
            <div className="flex items-center gap-2 min-w-0">
              <Type className="w-3.5 h-3.5 text-violet-400 shrink-0" />
              <span className="text-xs font-semibold text-zinc-200">Watermark Video</span>
              {watermark.enabled && watermark.text.trim() ? (
                <span className="rounded-full bg-violet-500/20 px-1.5 py-0.5 text-[9px] font-semibold text-violet-300 truncate max-w-[120px]">
                  {watermark.text}
                </span>
              ) : (
                <span className="text-[10px] text-zinc-500">(Nonaktif)</span>
              )}
            </div>
            <div className="flex items-center gap-1.5 shrink-0">
              <input
                type="checkbox"
                checked={watermark.enabled}
                onChange={(e) => {
                  e.stopPropagation();
                  updateWatermark((w) => ({ ...w, enabled: e.target.checked }));
                }}
                className="rounded border-zinc-700 bg-zinc-800 text-violet-500 focus:ring-0 w-3.5 h-3.5 cursor-pointer"
                title="Aktifkan / Nonaktifkan Watermark"
              />
              {watermarkExpanded ? (
                <ChevronDown className="w-3.5 h-3.5 text-zinc-400" />
              ) : (
                <ChevronRight className="w-3.5 h-3.5 text-zinc-500" />
              )}
            </div>
          </button>

          {watermarkExpanded && (
            <div className="border-t border-white/[0.06] p-3 space-y-3 bg-zinc-950/40">
              {/* Text Input */}
              <div>
                <label className="block text-[10px] text-zinc-400 mb-1">
                  Teks Watermark
                </label>
                <input
                  type="text"
                  value={watermark.text}
                  onChange={(e) =>
                    updateWatermark((w) => ({ ...w, text: e.target.value }))
                  }
                  placeholder="@ComiCut"
                  className="field text-xs font-mono"
                />
              </div>

              {/* Font Preset Selection */}
              <div>
                <div className="flex items-center justify-between mb-1">
                  <label className="text-[10px] text-zinc-400">
                    Preset Font (Popular, Bold &amp; Lebar)
                  </label>
                  <span className="text-[9px] text-violet-400 font-mono">
                    {watermark.font}
                  </span>
                </div>
                <div className="grid grid-cols-2 gap-1.5">
                  {WATERMARK_FONTS.map((f) => {
                    const isSelected = watermark.font === f.id;
                    return (
                      <button
                        key={f.id}
                        type="button"
                        onClick={() => updateWatermark((w) => ({ ...w, font: f.id }))}
                        className={`flex flex-col text-left px-2 py-1.5 rounded-lg border transition ${
                          isSelected
                            ? "border-violet-500/70 bg-violet-500/15 text-white"
                            : "border-white/[0.06] bg-zinc-900/50 text-zinc-400 hover:text-zinc-200 hover:border-white/10"
                        }`}
                      >
                        <span
                          className="text-xs truncate"
                          style={{
                            fontFamily: f.fontFamily,
                            fontWeight: f.fontWeight,
                          }}
                        >
                          {f.name}
                        </span>
                        <span className="text-[8px] text-zinc-500 truncate leading-tight">
                          {f.tag}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Position Presets & Sliders */}
              <div>
                <label className="block text-[10px] text-zinc-400 mb-1">
                  Posisi Watermark
                </label>
                {/* Quick Buttons */}
                <div className="grid grid-cols-3 gap-1 mb-2">
                  {[
                    { label: "Bawah (Default)", x: 50, y: 82 },
                    { label: "Bawah Kiri", x: 15, y: 82 },
                    { label: "Bawah Kanan", x: 85, y: 82 },
                    { label: "Tengah", x: 50, y: 50 },
                    { label: "Atas Kanan", x: 85, y: 10 },
                    { label: "Atas", x: 50, y: 10 },
                  ].map((pos) => {
                    const isActive =
                      watermark.positionX === pos.x && watermark.positionY === pos.y;
                    return (
                      <button
                        key={pos.label}
                        type="button"
                        onClick={() =>
                          updateWatermark((w) => ({
                            ...w,
                            positionX: pos.x,
                            positionY: pos.y,
                          }))
                        }
                        className={`text-[9px] py-1 px-1.5 rounded border transition truncate ${
                          isActive
                            ? "border-violet-500 bg-violet-500/20 text-white font-semibold"
                            : "border-white/[0.06] bg-zinc-900/40 text-zinc-400 hover:text-zinc-200"
                        }`}
                      >
                        {pos.label}
                      </button>
                    );
                  })}
                </div>

                {/* Fine Sliders */}
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between text-[9px] text-zinc-400">
                    <span>Posisi X (Horizontal)</span>
                    <span className="font-mono text-zinc-300">{watermark.positionX}%</span>
                  </div>
                  <input
                    type="range"
                    min="5"
                    max="95"
                    step="1"
                    value={watermark.positionX}
                    onChange={(e) =>
                      updateWatermark((w) => ({
                        ...w,
                        positionX: Number(e.target.value),
                      }))
                    }
                    className="w-full accent-violet-500 cursor-pointer h-1 bg-zinc-800 rounded-lg"
                  />

                  <div className="flex items-center justify-between text-[9px] text-zinc-400 pt-1">
                    <span>Posisi Y (Vertikal)</span>
                    <span className="font-mono text-zinc-300">
                      {watermark.positionY}% {watermark.positionY === 82 ? "(Safe Space Bawah)" : ""}
                    </span>
                  </div>
                  <input
                    type="range"
                    min="5"
                    max="95"
                    step="1"
                    value={watermark.positionY}
                    onChange={(e) =>
                      updateWatermark((w) => ({
                        ...w,
                        positionY: Number(e.target.value),
                      }))
                    }
                    className="w-full accent-violet-500 cursor-pointer h-1 bg-zinc-800 rounded-lg"
                  />
                  <p className="text-[8px] text-zinc-500 italic">
                    *Default 82% menyisakan space di bawah agar tidak tertutup caption/UI TikTok &amp; Reels.
                  </p>
                </div>
              </div>

              {/* Opacity Slider */}
              <div>
                <div className="flex items-center justify-between text-[10px] text-zinc-400 mb-1">
                  <span>Opacity (Transparansi)</span>
                  <span className="font-mono font-semibold text-violet-300">
                    {Math.round(watermark.opacity * 100)}% {watermark.opacity === 0.7 ? "(Default)" : ""}
                  </span>
                </div>
                <input
                  type="range"
                  min="0.1"
                  max="1.0"
                  step="0.05"
                  value={watermark.opacity}
                  onChange={(e) =>
                    updateWatermark((w) => ({
                      ...w,
                      opacity: Number(e.target.value),
                    }))
                  }
                  className="w-full accent-violet-500 cursor-pointer h-1 bg-zinc-800 rounded-lg"
                />
              </div>

              {/* Font Size Slider */}
              <div>
                <div className="flex items-center justify-between text-[10px] text-zinc-400 mb-1">
                  <span>Ukuran Font</span>
                  <span className="font-mono text-zinc-300">{watermark.fontSize || 48}px</span>
                </div>
                <input
                  type="range"
                  min="24"
                  max="84"
                  step="2"
                  value={watermark.fontSize || 48}
                  onChange={(e) =>
                    updateWatermark((w) => ({
                      ...w,
                      fontSize: Number(e.target.value),
                    }))
                  }
                  className="w-full accent-violet-500 cursor-pointer h-1 bg-zinc-800 rounded-lg"
                />
              </div>
            </div>
          )}
        </div>

        <div>
          <div className="mb-1 flex items-center justify-between">
            <label className="text-[10px] text-zinc-500">
              Lokasi output MP4 (H.264 60fps)
            </label>
            <button
              type="button"
              onClick={() => defaultOutputPath().then(setOutputPath)}
              className="flex items-center gap-1 text-[10px] text-violet-400 hover:text-violet-300 transition-colors"
              title="Perbarui nama file dengan timestamp waktu saat ini"
            >
              <RotateCw className="w-2.5 h-2.5" />
              Reset Nama Baru
            </button>
          </div>
          <div className="flex gap-2">
            <input
              type="text"
              value={outputPath}
              onChange={(e) => setOutputPath(e.target.value)}
              className="field font-mono flex-1 text-xs"
              placeholder="comicut_render_..."
            />
            <button
              type="button"
              onClick={handleBrowseOutput}
              className="btn-ghost px-2.5 py-1.5 text-xs text-zinc-300 hover:text-white"
              title="Pilih folder atau nama file lain"
            >
              <FolderOpen className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        <button
          type="button"
          disabled={timeline.length === 0 || isRendering}
          onClick={handleRenderMp4}
          className="btn-primary w-full py-2.5"
        >
          {isRendering ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Merender...
            </>
          ) : (
            <>
              <Download className="w-4 h-4" />
              Render Video (.MP4)
            </>
          )}
        </button>
      </div>
    </div>
  );
};
