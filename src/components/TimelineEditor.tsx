import React from "react";
import { Film, Wand2, Zap } from "lucide-react";
import { BeatMap, Panel, TimelineClip } from "../types/project";
import { panelImageSrc } from "../services/tauriCommands";

interface TimelineEditorProps {
  timeline: TimelineClip[];
  panels: Panel[];
  beatMap: BeatMap | null;
  currentTime: number;
  onSeek: (t: number) => void;
  onGenerateTimeline: () => void;
}

export const TimelineEditor: React.FC<TimelineEditorProps> = ({
  timeline,
  panels,
  beatMap,
  currentTime,
  onSeek,
  onGenerateTimeline,
}) => (
  <div className="card space-y-3">
    <div className="flex items-center justify-between">
      <h2 className="card-title">
        <span className="step">4</span>
        <Film className="w-4 h-4 text-amber-400" />
        Auto-Director Timeline ({timeline.length} klip)
      </h2>
      <button
        type="button"
        disabled={panels.every((p) => !p.selected) || !beatMap}
        onClick={onGenerateTimeline}
        className="btn bg-amber-500 text-zinc-950 hover:bg-amber-400"
      >
        <Wand2 className="w-3.5 h-3.5" />
        Sinkronkan ke Beat
      </button>
    </div>

    {timeline.length === 0 ? (
      <div className="rounded-xl border border-white/[0.06] bg-zinc-950/50 p-4 text-center text-xs text-zinc-500">
        Pilih panel (langkah 2) dan deteksi beat (langkah 3), lalu klik <b>Sinkronkan ke Beat</b>.
      </div>
    ) : (
      <div className="flex gap-2 overflow-x-auto pb-2 pt-1">
        {timeline.map((clip, i) => {
          const panel = panels.find((p) => p.id === clip.panelId);
          const src = panel ? panelImageSrc(panel) : undefined;
          const active = currentTime >= clip.startSec && currentTime < clip.endSec;
          const isDrop = beatMap && Math.abs(clip.startSec - beatMap.mainDropSec) < 0.06;
          const after = beatMap && clip.startSec >= beatMap.mainDropSec - 0.05;
          return (
            <button
              key={clip.id}
              type="button"
              onClick={() => onSeek(clip.startSec)}
              className={`w-[132px] shrink-0 overflow-hidden rounded-xl border text-left transition ${
                active
                  ? "border-violet-400 ring-1 ring-violet-400/60"
                  : after
                  ? "border-rose-500/40 hover:border-rose-400"
                  : "border-white/10 hover:border-white/30"
              } bg-zinc-950`}
            >
              <div className="relative h-20 bg-zinc-900">
                {src && (
                  <img src={src} alt="" loading="lazy" className="h-full w-full object-cover object-top" />
                )}
                <span className="absolute left-1 top-1 rounded bg-black/70 px-1 font-mono text-[9px] text-zinc-200">
                  #{i + 1} · P{clip.panelIndex + 1}
                </span>
                {isDrop && (
                  <span className="absolute right-1 top-1 flex items-center gap-0.5 rounded bg-rose-600 px-1 text-[9px] font-bold text-white">
                    <Zap className="h-2.5 w-2.5" />
                    DROP
                  </span>
                )}
              </div>
              <div className="space-y-0.5 p-1.5 text-[10px]">
                <div className="truncate font-mono text-amber-300">{clip.effect}</div>
                <div className="font-mono text-zinc-500">
                  {clip.startSec.toFixed(2)}–{clip.endSec.toFixed(2)}s
                </div>
              </div>
            </button>
          );
        })}
      </div>
    )}
  </div>
);
