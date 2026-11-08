import React, { useState, useEffect } from "react";
import {
  Play,
  Pause,
  Download,
  Smartphone,
  Sparkles,
  Zap,
  Loader2,
} from "lucide-react";
import {
  BeatMap,
  Panel,
  RenderConfig,
  TimelineClip,
} from "../types/project";
import { renderMontagemVideo } from "../services/tauriCommands";

interface VideoPreviewProps {
  panels: Panel[];
  beatMap: BeatMap | null;
  timeline: TimelineClip[];
  onStatusMessage: (msg: string) => void;
}

export const VideoPreview: React.FC<VideoPreviewProps> = ({
  panels,
  beatMap,
  timeline,
  onStatusMessage,
}) => {
  const [isPlaying, setIsPlaying] = useState<boolean>(false);
  const [currentTime, setCurrentTime] = useState<number>(0);
  const [outputPath, setOutputPath] = useState<string>(
    "D:\\Laravel\\comicut\\output_montagem_reels.mp4"
  );
  const [isRendering, setIsRendering] = useState<boolean>(false);

  const totalDuration =
    timeline.length > 0
      ? timeline[timeline.length - 1].endSec
      : beatMap?.durationSec || 9.0;

  useEffect(() => {
    if (!isPlaying) return;
    const interval = setInterval(() => {
      setCurrentTime((prev) => {
        if (prev + 0.05 >= totalDuration) {
          return 0;
        }
        return prev + 0.05;
      });
    }, 50);
    return () => clearInterval(interval);
  }, [isPlaying, totalDuration]);

  const activeClip =
    timeline.find(
      (c) => currentTime >= c.startSec && currentTime <= c.endSec
    ) || timeline[0];

  const activePanel = activeClip
    ? panels.find((p) => p.id === activeClip.panelId) || panels[0]
    : panels[0];

  const isBeatFlash =
    beatMap?.beats.some(
      (b) => b.isSubBassHit && Math.abs(b.timestampSec - currentTime) < 0.08
    ) ?? false;

  const handleRenderMp4 = async () => {
    if (timeline.length === 0 || !beatMap) return;
    setIsRendering(true);
    onStatusMessage(
      `Merender video 1080x1920 (60fps) dengan efek Montagem ke ${outputPath}...`
    );
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
      const savedPath = await renderMontagemVideo(
        panels,
        beatMap,
        timeline,
        config
      );
      onStatusMessage(`Render selesai! Video tersimpan di: ${savedPath}`);
    } catch (err: any) {
      onStatusMessage(`Info Render: ${err?.message || String(err)}`);
    } finally {
      setIsRendering(false);
    }
  };

  return (
    <div className="bg-zinc-900/90 border border-zinc-800 rounded-xl p-4 flex flex-col justify-between h-full space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Smartphone className="w-4 h-4 text-violet-400" />
          <h2 className="text-sm font-semibold text-zinc-100">
            5. Preview 9:16 (Reels / Shorts / TikTok)
          </h2>
        </div>
        <span className="text-[11px] font-mono text-zinc-400">
          1080×1920 • 60 FPS
        </span>
      </div>

      {/* 9:16 Vertical Canvas Simulator */}
      <div className="flex-1 flex items-center justify-center py-1">
        <div
          className={`relative w-[230px] h-[408px] rounded-2xl border-2 overflow-hidden flex flex-col justify-between p-3 transition-transform duration-75 ${
            isBeatFlash
              ? "border-rose-400 scale-[1.03] shadow-[0_0_30px_rgba(244,63,94,0.45)]"
              : "border-zinc-700 bg-zinc-950"
          }`}
        >
          {/* Impact Flash Overlay */}
          {isBeatFlash && (
            <div className="absolute inset-0 bg-white/25 pointer-events-none z-20 animate-pulse" />
          )}

          {/* Top HUD inside 9:16 frame */}
          <div className="relative z-10 flex items-center justify-between text-[10px]">
            <span className="bg-black/70 backdrop-blur px-2 py-0.5 rounded-full text-zinc-200 font-medium border border-white/10">
              {activeClip ? activeClip.effect : "Ready"}
            </span>
            {isBeatFlash && (
              <span className="bg-rose-600 text-white font-bold px-2 py-0.5 rounded-full flex items-center gap-0.5">
                <Zap className="w-2.5 h-2.5" />
                BASS HIT
              </span>
            )}
          </div>

          {/* Center Panel Content */}
          <div className="relative z-10 my-auto bg-gradient-to-b from-zinc-900/95 to-zinc-950/95 border border-zinc-700/80 rounded-xl p-4 text-center shadow-inner">
            {activePanel ? (
              <>
                <span className="text-[10px] uppercase tracking-wider text-violet-400 font-bold">
                  Panel #{activePanel.index + 1} • {activePanel.role}
                </span>
                <p className="text-xs font-semibold text-zinc-100 mt-1.5">
                  {activePanel.caption || activePanel.imagePath}
                </p>
                <div className="mt-3 text-[10px] font-mono text-zinc-400">
                  {activePanel.bbox.width}×{activePanel.bbox.height}px
                </div>
                {activePanel.isVerticalPan && (
                  <div className="mt-2 inline-block text-[10px] bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 px-2 py-0.5 rounded">
                    Vertical Camera Scroll Active
                  </div>
                )}
              </>
            ) : (
              <div className="text-xs text-zinc-500 py-8">
                Muat Panel &amp; Lagu untuk melihat simulasi gerakan kamera
                Montagem
              </div>
            )}
          </div>

          {/* Bottom Audio Caption Bar */}
          <div className="relative z-10 space-y-1.5">
            <div className="flex items-center justify-between text-[10px] text-zinc-300 font-mono">
              <span>{currentTime.toFixed(2)}s</span>
              <span>{totalDuration.toFixed(2)}s</span>
            </div>
            <div className="w-full h-1.5 bg-zinc-800 rounded-full overflow-hidden">
              <div
                style={{
                  width: `${Math.min(
                    100,
                    (currentTime / Math.max(0.1, totalDuration)) * 100
                  )}%`,
                }}
                className="h-full bg-gradient-to-r from-violet-500 to-rose-500"
              />
            </div>
          </div>
        </div>
      </div>

      {/* Controls & Export MP4 */}
      <div className="space-y-2.5 pt-2 border-t border-zinc-800">
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setIsPlaying(!isPlaying)}
            className="flex-1 bg-zinc-800 hover:bg-zinc-700 text-zinc-100 text-xs font-semibold py-2 px-3 rounded-lg flex items-center justify-center gap-1.5 transition"
          >
            {isPlaying ? (
              <>
                <Pause className="w-3.5 h-3.5" /> Pause Preview
              </>
            ) : (
              <>
                <Play className="w-3.5 h-3.5" /> Play Beat Sync Preview
              </>
            )}
          </button>
        </div>

        <div>
          <label className="block text-[10px] text-zinc-400 mb-1">
            Lokasi Output Render MP4 (FFmpeg H.264 60fps)
          </label>
          <input
            type="text"
            value={outputPath}
            onChange={(e) => setOutputPath(e.target.value)}
            className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 font-mono"
          />
        </div>

        <button
          type="button"
          disabled={timeline.length === 0 || isRendering}
          onClick={handleRenderMp4}
          className="w-full bg-gradient-to-r from-violet-600 to-rose-600 hover:from-violet-500 hover:to-rose-500 disabled:opacity-40 text-white text-xs font-bold py-2.5 px-4 rounded-lg flex items-center justify-center gap-2 transition shadow-lg shadow-rose-950/40"
        >
          {isRendering ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Merender Frame ke FFmpeg...
            </>
          ) : (
            <>
              <Download className="w-4 h-4" />
              Render Video Reels / Shorts (.MP4)
            </>
          )}
        </button>
      </div>
    </div>
  );
};
