import React, { useEffect, useState } from "react";
import {
  Download,
  Loader2,
  Pause,
  Play,
  Smartphone,
  Volume2,
  VolumeX,
  Zap,
} from "lucide-react";
import { BeatMap, Panel, RenderConfig, TimelineClip } from "../types/project";
import { defaultOutputPath, panelImageSrc, renderMontagemVideo } from "../services/tauriCommands";
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
  const currentTime = playback.time;

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
      };
      const saved = await renderMontagemVideo(panels, beatMap, timeline, config);
      onStatusMessage(`Render selesai! Video tersimpan di: ${saved}`);
      onRenderSuccess?.(saved);
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

        <div>
          <label className="mb-1 block text-[10px] text-zinc-500">
            Lokasi output MP4 (H.264 60fps)
          </label>
          <input
            type="text"
            value={outputPath}
            onChange={(e) => setOutputPath(e.target.value)}
            className="field font-mono"
          />
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
