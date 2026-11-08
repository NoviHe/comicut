import React, { useState } from "react";
import { Scissors, Sparkles, Terminal } from "lucide-react";
import { BeatMap, EffectType, Panel, TimelineClip } from "./types/project";
import { ChapterImporter } from "./components/ChapterImporter";
import { PanelGallery } from "./components/PanelGallery";
import { AudioWaveform } from "./components/AudioWaveform";
import { TimelineEditor } from "./components/TimelineEditor";
import { VideoPreview } from "./components/VideoPreview";
import { generateAutoTimeline } from "./services/tauriCommands";

export function App() {
  const [panels, setPanels] = useState<Panel[]>([]);
  const [chapterTitle, setChapterTitle] = useState<string>("");
  const [beatMap, setBeatMap] = useState<BeatMap | null>(null);
  const [timeline, setTimeline] = useState<TimelineClip[]>([]);
  const [statusMessage, setStatusMessage] = useState<string>(
    "Siap! Pilih platform (Webtoon, MangaDex, VoraToon https://voratoon.web.id/, atau Web Custom) untuk memulai."
  );

  const buildLocalTimelineFallback = (
    selectedPanels: Panel[],
    audio: BeatMap
  ): TimelineClip[] => {
    if (selectedPanels.length === 0) return [];
    const beats = audio.beats;
    const clips: TimelineClip[] = [];

    for (let i = 0; i < selectedPanels.length; i++) {
      const panel = selectedPanels[i];
      const startSec =
        i < beats.length
          ? beats[i].timestampSec
          : (i / selectedPanels.length) * audio.durationSec;
      const endSec =
        i + 1 < beats.length && i + 1 < selectedPanels.length
          ? beats[i + 1].timestampSec
          : i === selectedPanels.length - 1
          ? audio.durationSec
          : startSec + audio.durationSec / selectedPanels.length;

      const isAfterDrop = startSec >= audio.mainDropSec - 0.05;
      let effect: EffectType = "SlowPushIn";
      if (panel.isVerticalPan) {
        effect = "VerticalPanScroll";
      } else if (isAfterDrop && i % 2 === 0) {
        effect = "ZoomPunch";
      } else if (isAfterDrop) {
        effect = "CameraShake";
      }

      clips.push({
        id: `clip-${i + 1}`,
        panelId: panel.id,
        panelIndex: panel.index,
        role: panel.role,
        startSec,
        endSec,
        durationSec: Math.max(0.2, endSec - startSec),
        effect,
        beatIntensity: isAfterDrop ? 0.95 : 0.5,
        shakeAmplitude: isAfterDrop ? 14.0 : 2.0,
        zoomScale: isAfterDrop ? 1.18 : 1.05,
      });
    }
    return clips;
  };

  const handleSyncTimeline = async () => {
    const activePanels = panels.filter((p) => p.selected);
    if (activePanels.length === 0 || !beatMap) return;

    try {
      const generated = await generateAutoTimeline(activePanels, beatMap);
      if (generated && generated.length > 0) {
        setTimeline(generated);
      } else {
        setTimeline(buildLocalTimelineFallback(activePanels, beatMap));
      }
      setStatusMessage(
        `Timeline otomatis dibuat! ${activePanels.length} panel telah disinkronkan dengan Beat Drop lagu Montagem.`
      );
    } catch {
      const fallback = buildLocalTimelineFallback(activePanels, beatMap);
      setTimeline(fallback);
      setStatusMessage(
        `Timeline disinkronkan (${fallback.length} klip mengikuti ketukan bass)!`
      );
    }
  };

  return (
    <div className="min-h-screen h-screen flex flex-col bg-zinc-950 text-zinc-100 select-none">
      {/* Header Navbar */}
      <header className="h-13 px-5 py-2.5 border-b border-zinc-800/90 bg-zinc-950/90 flex items-center justify-between shrink-0">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-violet-600 to-rose-600 flex items-center justify-center shadow-md shadow-violet-950">
            <Scissors className="w-4 h-4 text-white" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="text-sm font-bold tracking-tight text-white">
                ComiCut Studio
              </h1>
              <span className="text-[10px] font-semibold bg-violet-500/20 text-violet-300 border border-violet-500/30 px-2 py-0.2 rounded-full">
                Rust + Tauri v2
              </span>
            </div>
            <p className="text-[11px] text-zinc-400">
              Auto Webtoon/Manga Epic Panel Slicer &amp; Montagem Beat-Sync
              Video Maker
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2 text-xs">
          <span className="text-zinc-500 hidden md:inline">
            Supported: LINE Webtoon • MangaDex • VoraToon (voratoon.web.id) •
            Custom Web
          </span>
          <span className="bg-zinc-900 border border-zinc-800 px-2.5 py-1 rounded-lg text-[11px] text-amber-300 flex items-center gap-1">
            <Sparkles className="w-3 h-3" />
            9:16 Reels / Shorts Engine
          </span>
        </div>
      </header>

      {/* Main Workspace Grid */}
      <main className="flex-1 grid grid-cols-12 gap-3.5 p-4 overflow-y-auto">
        {/* Left & Center Column (Importer, Panel Reviewer, Audio, Timeline) */}
        <div className="col-span-12 lg:col-span-8 xl:col-span-9 flex flex-col gap-3.5">
          <div className="grid grid-cols-1 xl:grid-cols-12 gap-3.5">
            <div className="xl:col-span-5">
              <ChapterImporter
                onPanelsLoaded={(loadedPanels, title) => {
                  setPanels(loadedPanels);
                  setChapterTitle(title);
                }}
                onStatusMessage={setStatusMessage}
              />
            </div>
            <div className="xl:col-span-7">
              <PanelGallery
                panels={panels}
                chapterTitle={chapterTitle}
                onUpdatePanels={setPanels}
                onStatusMessage={setStatusMessage}
              />
            </div>
          </div>

          <AudioWaveform
            beatMap={beatMap}
            onBeatMapLoaded={(bm) => {
              setBeatMap(bm);
              if (panels.length > 0) {
                const active = panels.filter((p) => p.selected);
                setTimeline(buildLocalTimelineFallback(active, bm));
              }
            }}
            onStatusMessage={setStatusMessage}
          />

          <TimelineEditor
            timeline={timeline}
            panels={panels}
            beatMap={beatMap}
            onGenerateTimeline={handleSyncTimeline}
          />
        </div>

        {/* Right Column (9:16 Vertical Preview & Export) */}
        <div className="col-span-12 lg:col-span-4 xl:col-span-3">
          <VideoPreview
            panels={panels}
            beatMap={beatMap}
            timeline={timeline}
            onStatusMessage={setStatusMessage}
          />
        </div>
      </main>

      {/* Status Footer */}
      <footer className="h-8 px-4 bg-zinc-900 border-t border-zinc-800 flex items-center justify-between text-[11px] text-zinc-400 shrink-0">
        <div className="flex items-center gap-2 truncate">
          <Terminal className="w-3.5 h-3.5 text-violet-400 shrink-0" />
          <span className="truncate">{statusMessage}</span>
        </div>
        <span className="text-zinc-500 font-mono shrink-0">
          Engine: Rust ImageProc + Symphonia FFT + FFmpeg Pipe
        </span>
      </footer>
    </div>
  );
}

export default App;
