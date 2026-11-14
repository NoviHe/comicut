import React, { useEffect, useMemo, useRef, useState } from "react";
import { Scissors, Settings, Terminal } from "lucide-react";
import { BeatMap, EffectType, Panel, PanelStyle, TimelineClip, WatermarkConfig } from "./types/project";
import { ChapterImporter } from "./components/ChapterImporter";
import { PanelGallery } from "./components/PanelGallery";
import { AudioWaveform } from "./components/AudioWaveform";
import { TimelineEditor } from "./components/TimelineEditor";
import { VideoPreview } from "./components/VideoPreview";
import { WatermarkSettings } from "./components/WatermarkSettings";
import { useDependencies } from "./components/DependencyStatus";
import { AiSettingsModal } from "./components/AiSettingsModal";
import { NavTab, SidebarNav } from "./components/SidebarNav";
import { TopBar } from "./components/TopBar";
import { ProjectsManager } from "./components/ProjectsManager";
import { SettingsView } from "./components/SettingsView";
import { UpdateModal } from "./components/UpdateModal";
import { AppUpdateInfo, checkComiCutUpdate } from "./services/appUpdater";
import {
  buildProjectSnapshot,
  loadSavedProjects,
  SavedProject,
  saveProject,
} from "./services/projectStorage";
import { loadAiSettings, loadPanelStyle, savePanelStyle, toReviewConfig } from "./services/aiSettings";
import { applyAdjust, BeatAdjust, localSongQueries, NO_ADJUST, songFit, targetPanelCount } from "./services/beatAdjust";
import { usePlayback } from "./services/usePlayback";
import {
  aiDirectTimeline,
  analyzeAudioBeats,
  defaultOutputPath,
  downloadAudio,
  generateAutoTimeline,
  getStoredWatermarkConfig,
  renderMontagemVideo,
  reviewEpicPanels,
  saveStoredWatermarkConfig,
  suggestSongQueries,
  scrapeComicChapter,
} from "./services/tauriCommands";
import { AutoPilot, AutoInput, platformFor } from "./components/AutoPilot";

// Browser-only fallback (no Rust backend): same cut pattern, simplified.
function localTimeline(selected: Panel[], audio: BeatMap): TimelineClip[] {
  const cuts = [0, ...audio.beats.map((b) => b.timestampSec).filter((t) => t > 0.15)];
  return cuts.map((start, i) => {
    const end = cuts[i + 1] ?? audio.durationSec;
    const panel = selected[i % selected.length];
    const after = start >= audio.mainDropSec - 0.05;
    const effect: EffectType = panel.isVerticalPan
      ? "VerticalPanScroll"
      : after
      ? i % 2 === 0
        ? "ZoomPunch"
        : "CameraShake"
      : "SlowPushIn";
    return {
      id: `clip-${i + 1}`,
      panelId: panel.id,
      panelIndex: panel.index,
      role: panel.role,
      startSec: start,
      endSec: end,
      durationSec: Math.max(0.2, end - start),
      effect,
      beatIntensity: after ? 0.95 : 0.5,
      shakeAmplitude: after ? 14 : 2,
      zoomScale: after ? 1.18 : 1.06,
    };
  });
}

export function App() {
  const [panels, setPanels] = useState<Panel[]>([]);
  const [chapterTitle, setChapterTitle] = useState<string>("");
  const [rawBeatMap, setRawBeatMap] = useState<BeatMap | null>(null);
  const [adjust, setAdjust] = useState<BeatAdjust>(NO_ADJUST);
  const [timeline, setTimeline] = useState<TimelineClip[]>([]);
  const [ai, setAi] = useState(loadAiSettings);
  const [panelStyle, setPanelStyleState] = useState<PanelStyle>(loadPanelStyle);
  const setPanelStyle = (s: PanelStyle) => {
    setPanelStyleState(s);
    savePanelStyle(s);
  };
  const [showSettings, setShowSettings] = useState(false);
  const [activeNav, setActiveNav] = useState<NavTab>("home");
  const [currentProjectId, setCurrentProjectId] = useState<string | undefined>();
  const [sessionKey, setSessionKey] = useState(0);
  const [autoDirty, setAutoDirty] = useState(false);
  const [savedProjects, setSavedProjects] = useState<SavedProject[]>(() => loadSavedProjects());
  const [watermark, setWatermark] = useState<WatermarkConfig>(() => getStoredWatermarkConfig());

  const updateWatermark = (updater: (prev: WatermarkConfig) => WatermarkConfig) => {
    setWatermark((prev) => {
      const next = updater(prev);
      saveStoredWatermarkConfig(next);
      return next;
    });
  };
  const [statusMessage, setStatusMessage] = useState<string>(
    "Siap! Mulai dari langkah 1: pilih sumber komik."
  );
  const [availableUpdate, setAvailableUpdate] = useState<AppUpdateInfo | null>(null);
  const deps = useDependencies(setStatusMessage);

  // Check for app updates once on startup (ref guards StrictMode's double effect).
  const updateChecked = useRef(false);
  useEffect(() => {
    if (updateChecked.current) return;
    updateChecked.current = true;
    void checkComiCutUpdate().then((info) => {
      if (info && info.available) {
        setAvailableUpdate(info);
      }
    });
  }, []);

  const beatMap = useMemo(
    () => (rawBeatMap ? applyAdjust(rawBeatMap, adjust) : null),
    [rawBeatMap, adjust]
  );
  const duration = timeline.length
    ? timeline[timeline.length - 1].endSec
    : beatMap?.durationSec || 9;
  const playback = usePlayback(beatMap, duration, setStatusMessage);

  const selected = panels.filter((p) => p.selected);

  const targetPanels = beatMap ? targetPanelCount(beatMap) : 12;

  const sync = async (bm: BeatMap, sel: Panel[], useAi = false): Promise<TimelineClip[]> => {
    const done = (clips: TimelineClip[]) => {
      setTimeline(clips);
      return clips;
    };
    if (sel.length === 0) return done([]);
    if (useAi && ai.provider !== "HeuristicOnly") {
      setStatusMessage("AI director menyusun panel & efek per potongan...");
      try {
        const directed = await aiDirectTimeline(sel, bm, toReviewConfig(ai, sel.length));
        setStatusMessage(`AI director selesai: ${directed.length} potongan.`);
        return done(directed);
      } catch (e: any) {
        setStatusMessage(`AI director gagal (${e?.message || e}); memakai director lokal.`);
      }
    }
    try {
      const generated = await generateAutoTimeline(sel, bm);
      return done(generated.length ? generated : localTimeline(sel, bm));
    } catch {
      return done(localTimeline(sel, bm));
    }
  };

  // The beatMap effect below must not race the auto pipeline's own sync.
  const autoRunning = useRef(false);

  /** One-click pipeline: scrape -> audio -> AI panel review -> AI director -> optional render. */
  const runAuto = async (input: AutoInput, step: (i: number, label: string) => void) => {
    autoRunning.current = true;
    const useAi = ai.provider !== "HeuristicOnly";
    try {
      step(0, "Mengambil & memotong panel...");
      setTimeline([]);
      const scraped = await scrapeComicChapter({
        url: input.url,
        platform: platformFor(input.url),
        comicFormat: input.url.includes("mangadex") ? "MangaRTL" : "WebtoonVertical",
        panelStyle: input.panelStyle,
      });
      let list = scraped.extractedPanels;
      if (list.length === 0) throw new Error("Tidak ada panel yang berhasil diekstrak.");
      setPanels(list);
      setChapterTitle(scraped.chapterTitle);

      step(1, "Menyiapkan audio & mendeteksi beat...");
      const looksLocal = /\.(mp3|wav|m4a|aac|ogg|flac)$/i.test(input.song) && !/^https?:\/\//i.test(input.song);
      let bm: BeatMap;
      if (input.song) {
        bm = await analyzeAudioBeats(looksLocal ? input.song : await downloadAudio(input.song));
      } else {
        // No song given: pick one whose mood and length fit the panels.
        const want = Math.max(6, list.filter((p) => p.selected).length);
        let queries = localSongQueries(list);
        if (useAi) {
          step(1, "AI membaca mood panel untuk memilih lagu...");
          try {
            queries = await suggestSongQueries(list, toReviewConfig(ai, want, input.panelStyle));
          } catch (e: any) {
            setStatusMessage(`Saran lagu AI gagal (${e?.message || e}); memakai preset lokal.`);
          }
        }
        let best: { bm: BeatMap; fit: number; q: string } | null = null;
        for (const [i, q] of queries.slice(0, 3).entries()) {
          step(1, `Mencoba lagu ${i + 1}/3: ${q}`);
          try {
            const cand = await analyzeAudioBeats(await downloadAudio(q));
            const fit = songFit(cand, want);
            if (!best || fit > best.fit) best = { bm: cand, fit, q };
            if (fit >= 0.8) break;
          } catch (e: any) {
            setStatusMessage(`Lagu "${q}" dilewati: ${e?.message || e}`);
          }
        }
        if (!best) throw new Error("Tidak ada lagu yang berhasil diunduh. Isi kolom lagu secara manual.");
        bm = best.bm;
        setStatusMessage(`Lagu terpilih: ${bm.fileName} (kecocokan ${(best.fit * 100).toFixed(0)}%, query "${best.q}").`);
      }
      setRawBeatMap(bm);
      setAdjust(NO_ADJUST);

      step(2, useAi ? "AI memilih panel epic..." : "Memilih panel epic (lokal)...");
      try {
        list = await reviewEpicPanels(list, toReviewConfig(ai, targetPanelCount(bm), input.panelStyle));
      } catch (e: any) {
        setStatusMessage(`Review AI gagal (${e?.message || e}); memakai pilihan lokal.`);
      }
      setPanels(list);
      const chosen = list.filter((p) => p.selected);

      step(3, useAi ? "AI director menyusun timeline..." : "Menyusun timeline (lokal)...");
      const clips = await sync(bm, chosen, true);

      if (input.render) {
        step(4, "Merender video MP4...");
        const out = await defaultOutputPath();
        await renderMontagemVideo(list, bm, clips, {
          outputPath: out,
          width: 1080,
          height: 1920,
          fps: 60,
          blurBackground: true,
          vignetteStrength: 0.35,
          beatFlashEnabled: true,
          watermark: watermark.enabled && watermark.text.trim() ? watermark : null,
        });
        setStatusMessage(`Auto selesai! Video tersimpan di: ${out}`);
      } else {
        setStatusMessage(`Auto selesai: ${chosen.length} panel, ${clips.length} potongan. Putar preview lalu render.`);
      }

      // Auto-save to project history
      try {
        const snap = buildProjectSnapshot({
          id: currentProjectId,
          chapterTitle: scraped.chapterTitle,
          panels: list,
          rawBeatMap: bm,
          adjust: NO_ADJUST,
          timeline: clips,
        });
        saveProject(snap);
        setCurrentProjectId(snap.id);
        setSavedProjects(loadSavedProjects());
      } catch (err) {
        console.warn("Auto-save project snapshot failed:", err);
      }
    } finally {
      autoRunning.current = false;
    }
  };

  const handleOpenProject = (proj: SavedProject) => {
    setCurrentProjectId(proj.id);
    setPanels(proj.panels);
    setChapterTitle(proj.chapterTitle);
    setRawBeatMap(proj.rawBeatMap);
    setAdjust(proj.adjust);
    setTimeline(proj.timeline);
    setActiveNav("home");
    setStatusMessage(`Proyek "${proj.title}" berhasil dimuat. Siap diedit.`);
  };

  const hasActiveSession =
    panels.length > 0 ||
    timeline.length > 0 ||
    !!rawBeatMap ||
    !!currentProjectId ||
    autoDirty ||
    panelStyle !== "Default";

  const handleNewSession = () => {
    if (playback.playing) {
      playback.toggle();
    }
    playback.seek(0);
    setCurrentProjectId(undefined);
    setPanels([]);
    setChapterTitle("");
    setRawBeatMap(null);
    setAdjust(NO_ADJUST);
    setTimeline([]);
    setPanelStyle("Default");
    setAutoDirty(false);
    setSessionKey((k) => k + 1);
    setActiveNav("home");
    setStatusMessage("Sesi baru dimulai. Silakan pilih sumber komik untuk memulai.");
  };

  const handleNewProject = () => {
    handleNewSession();
  };

  const handleSaveCurrentProject = () => {
    if (panels.length === 0 && timeline.length === 0) {
      setStatusMessage("Belum ada panel atau timeline untuk disimpan.");
      return;
    }
    const snap = buildProjectSnapshot({
      id: currentProjectId,
      chapterTitle,
      panels,
      rawBeatMap,
      adjust,
      timeline,
    });
    saveProject(snap);
    setCurrentProjectId(snap.id);
    setSavedProjects(loadSavedProjects());
    setStatusMessage(`Proyek "${snap.title}" berhasil disimpan ke riwayat.`);
  };

  // Keep the timeline in step with beat adjustments / panel selection once one exists.
  const hadTimeline = useRef(false);
  hadTimeline.current = timeline.length > 0;
  useEffect(() => {
    if (beatMap && hadTimeline.current && !autoRunning.current) void sync(beatMap, selected);
  }, [beatMap]);

  return (
    <div className="flex h-screen select-none flex-col overflow-hidden bg-zinc-950 text-zinc-100">
      <div className="pointer-events-none fixed inset-0 -z-0 bg-[radial-gradient(ellipse_at_top_left,rgba(124,58,237,0.14),transparent_55%),radial-gradient(ellipse_at_bottom_right,rgba(236,72,153,0.08),transparent_50%)]" />

      {/* Integrated Custom Frameless Titlebar */}
      <TopBar
        canGoBack={activeNav !== "home"}
        onBack={() => setActiveNav("home")}
        onOpenSettings={() => setShowSettings(true)}
        aiProvider={ai.provider}
        hasActiveSession={hasActiveSession}
        onNewSession={handleNewSession}
      />

      {/* App Body: Left Sidebar + Content Area */}
      <div className="relative z-10 flex min-h-0 flex-1 overflow-hidden">
        <SidebarNav active={activeNav} onSelect={setActiveNav} />

        <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
          {activeNav === "home" ? (
            <main className="relative z-10 grid flex-1 grid-cols-12 gap-4 overflow-y-auto p-4">
              <div className="col-span-12 flex flex-col gap-4 xl:col-span-8 2xl:col-span-9">
                <AutoPilot
                  key={`auto-${sessionKey}`}
                  ai={ai}
                  panelStyle={panelStyle}
                  onChangePanelStyle={setPanelStyle}
                  onRun={runAuto}
                  onOpenSettings={() => setShowSettings(true)}
                  onDirtyChange={setAutoDirty}
                />
                <WatermarkSettings watermark={watermark} onChange={updateWatermark} />
                <div className="grid grid-cols-1 gap-4 2xl:grid-cols-12">
                  <div className="2xl:col-span-5">
                    <ChapterImporter
                      key={`importer-${sessionKey}`}
                      onPanelsLoaded={(p, title) => {
                        setPanels(p);
                        setChapterTitle(title);
                        setTimeline([]);
                      }}
                      onStatusMessage={setStatusMessage}
                      panelStyle={panelStyle}
                      onChangePanelStyle={setPanelStyle}
                    />
                  </div>
                  <div className="2xl:col-span-7">
                    <PanelGallery
                      panels={panels}
                      chapterTitle={chapterTitle}
                      onUpdatePanels={setPanels}
                      onStatusMessage={setStatusMessage}
                      ai={ai}
                      panelStyle={panelStyle}
                      targetPanels={targetPanels}
                      onOpenSettings={() => setShowSettings(true)}
                    />
                  </div>
                </div>

                <AudioWaveform
                  key={`audio-${sessionKey}`}
                  beatMap={rawBeatMap ? beatMap : null}
                  adjust={adjust}
                  onAdjust={setAdjust}
                  playback={playback}
                  onBeatMapLoaded={(bm) => {
                    setRawBeatMap(bm);
                    setAdjust(NO_ADJUST);
                    void sync(bm, selected);
                  }}
                  onStatusMessage={setStatusMessage}
                />

                <TimelineEditor
                  timeline={timeline}
                  panels={panels}
                  beatMap={beatMap}
                  currentTime={playback.time}
                  onSeek={playback.seek}
                  onGenerateTimeline={async () => {
                    if (!beatMap) return;
                    await sync(beatMap, selected, true);
                  }}
                />
              </div>

              <div className="col-span-12 xl:col-span-4 2xl:col-span-3">
                <div className="xl:sticky xl:top-0">
                  <VideoPreview
                    key={`preview-${sessionKey}`}
                    panels={panels}
                    beatMap={beatMap}
                    timeline={timeline}
                    playback={playback}
                    watermark={watermark}
                    onStatusMessage={setStatusMessage}
                    onRenderSuccess={(out) => {
                      if (panels.length > 0) {
                        const snap = buildProjectSnapshot({
                          id: currentProjectId,
                          chapterTitle,
                          panels,
                          rawBeatMap,
                          adjust,
                          timeline,
                          lastOutputPath: out,
                        });
                        saveProject(snap);
                        setCurrentProjectId(snap.id);
                        setSavedProjects(loadSavedProjects());
                      }
                    }}
                  />
                </div>
              </div>
            </main>
          ) : activeNav === "projects" ? (
            <ProjectsManager
              projects={savedProjects}
              currentProjectId={currentProjectId}
              hasCurrentWork={panels.length > 0 || timeline.length > 0}
              onRefreshProjects={() => setSavedProjects(loadSavedProjects())}
              onOpenProject={handleOpenProject}
              onNewProject={handleNewProject}
              onSaveCurrentProject={handleSaveCurrentProject}
              onStatusMessage={setStatusMessage}
            />
          ) : activeNav === "settings" ? (
            <SettingsView
              onCheckUpdate={() => {
                void checkComiCutUpdate().then((info) => {
                  if (info && info.available) {
                    setAvailableUpdate(info);
                  }
                });
              }}
              onUpdateFound={setAvailableUpdate}
              deps={deps}
            />
          ) : (
            <main className="relative z-10 flex flex-1 items-center justify-center p-6">
              <div className="card flex max-w-md flex-col items-center gap-2 px-8 py-10 text-center">
                <h2 className="text-sm font-semibold capitalize text-zinc-100">{activeNav}</h2>
                <p className="text-xs text-zinc-500">Menu ini masih kosong.</p>
              </div>
            </main>
          )}

          <footer className="relative z-10 flex h-8 shrink-0 items-center gap-2 border-t border-white/[0.06] bg-zinc-950/80 px-4 text-[11px] text-zinc-400">
            <Terminal className="h-3.5 w-3.5 shrink-0 text-violet-400" />
            <span className="truncate">{statusMessage}</span>
          </footer>
        </div>
      </div>

      {/* Dynamic In-App Update Modal */}
      {availableUpdate && (
        <UpdateModal
          updateInfo={availableUpdate}
          onClose={() => setAvailableUpdate(null)}
          onStatusMessage={setStatusMessage}
        />
      )}

      {showSettings && (
        <AiSettingsModal settings={ai} onSave={setAi} onClose={() => setShowSettings(false)} />
      )}
    </div>
  );
}

export default App;
