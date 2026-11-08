import React from "react";
import { Film, Wand2, Zap } from "lucide-react";
import { BeatMap, Panel, TimelineClip } from "../types/project";

interface TimelineEditorProps {
  timeline: TimelineClip[];
  panels: Panel[];
  beatMap: BeatMap | null;
  onGenerateTimeline: () => void;
}

export const TimelineEditor: React.FC<TimelineEditorProps> = ({
  timeline,
  panels,
  beatMap,
  onGenerateTimeline,
}) => {
  return (
    <div className="bg-zinc-900/90 border border-zinc-800 rounded-xl p-4 space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Film className="w-4 h-4 text-amber-400" />
          <h2 className="text-sm font-semibold text-zinc-100">
            4. Auto-Director Beat Timeline ({timeline.length} Clips)
          </h2>
        </div>

        <button
          type="button"
          disabled={panels.length === 0 || !beatMap}
          onClick={onGenerateTimeline}
          className="bg-amber-500 hover:bg-amber-400 disabled:opacity-40 text-zinc-950 text-xs font-bold px-3 py-1.5 rounded-lg flex items-center gap-1.5 transition"
        >
          <Wand2 className="w-3.5 h-3.5" />
          Sinkronkan Panel ke Beat Lagu
        </button>
      </div>

      {timeline.length === 0 ? (
        <div className="bg-zinc-950/60 border border-zinc-800/60 rounded-lg p-4 text-center text-xs text-zinc-500">
          Pastikan <b>Panel Komik</b> dan <b>Audio Beat Map</b> sudah dimuat,
          lalu klik <b>Sinkronkan Panel ke Beat Lagu</b>.
        </div>
      ) : (
        <div className="flex gap-2 overflow-x-auto pb-2 pt-1">
          {timeline.map((clip, i) => {
            const isDropClip =
              beatMap && clip.startSec >= beatMap.mainDropSec - 0.05;
            return (
              <div
                key={clip.id}
                className={`min-w-[165px] rounded-lg border p-2.5 flex flex-col justify-between shrink-0 ${
                  isDropClip
                    ? "bg-rose-950/30 border-rose-500/60"
                    : "bg-zinc-950 border-zinc-800"
                }`}
              >
                <div>
                  <div className="flex items-center justify-between text-[10px] mb-1">
                    <span className="font-bold text-zinc-200 flex items-center gap-1">
                      {isDropClip && (
                        <Zap className="w-3 h-3 text-rose-400 fill-rose-400" />
                      )}
                      Clip #{i + 1} (Panel #{clip.panelIndex + 1})
                    </span>
                    <span className="font-mono text-zinc-400">
                      {clip.startSec.toFixed(1)}s–{clip.endSec.toFixed(1)}s
                    </span>
                  </div>

                  <div className="text-[11px] font-medium text-violet-300">
                    Role: {clip.role}
                  </div>
                </div>

                <div className="mt-2 pt-1.5 border-t border-zinc-800/80 flex items-center justify-between text-[10px]">
                  <span className="bg-zinc-900 text-amber-300 px-1.5 py-0.5 rounded border border-zinc-800 font-mono">
                    FX: {clip.effect}
                  </span>
                  <span className="text-zinc-500 font-mono">
                    Zoom {clip.zoomScale.toFixed(2)}x
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
