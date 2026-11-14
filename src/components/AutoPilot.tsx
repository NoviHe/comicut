import React, { useEffect, useState } from "react";
import { AlertCircle, Check, Loader2, Settings, Sparkles } from "lucide-react";
import { PanelStyle, PlatformKind } from "../types/project";
import { AiSettings, defaultModel } from "../services/aiSettings";
import { PANEL_STYLES } from "./ChapterImporter";

export interface AutoInput {
  url: string;
  song: string;
  render: boolean;
  panelStyle: PanelStyle;
}

export const platformFor = (url: string): PlatformKind => {
  const u = url.toLowerCase();
  if (u.includes("webtoons.com")) return "Webtoon";
  if (u.includes("mangadex.org")) return "MangaDex";
  return "Custom";
};

const STEPS = ["Scrape panel", "Pilih lagu & beat", "AI pilih panel", "AI director", "Render MP4"];

interface Props {
  ai: AiSettings;
  panelStyle: PanelStyle;
  onChangePanelStyle: (style: PanelStyle) => void;
  onRun: (input: AutoInput, step: (i: number, label: string) => void) => Promise<void>;
  onOpenSettings: () => void;
  onDirtyChange?: (dirty: boolean) => void;
}

export const AutoPilot: React.FC<Props> = ({
  ai,
  panelStyle,
  onChangePanelStyle,
  onRun,
  onOpenSettings,
  onDirtyChange,
}) => {
  const [url, setUrl] = useState("");
  const [song, setSong] = useState("");
  const [render, setRender] = useState(false);
  const [running, setRunning] = useState(false);
  const [stepIdx, setStepIdx] = useState(-1);
  const [stepLabel, setStepLabel] = useState("");
  const [error, setError] = useState("");
  const useAi = ai.provider !== "HeuristicOnly";
  const steps = render ? STEPS : STEPS.slice(0, 4);

  useEffect(() => {
    onDirtyChange?.(Boolean(url.trim() || song.trim() || render || stepIdx >= 0 || error));
  }, [url, song, render, stepIdx, error, onDirtyChange]);

  const go = async () => {
    setRunning(true);
    setError("");
    setStepIdx(0);
    try {
      await onRun({ url: url.trim(), song: song.trim(), render, panelStyle }, (i, label) => {
        setStepIdx(i);
        setStepLabel(label);
      });
      setStepIdx(steps.length);
      setStepLabel("Selesai");
    } catch (e: any) {
      setError(e?.message || String(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="card space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="card-title">
          <Sparkles className="w-4 h-4 text-fuchsia-400" />
          Satu Klik
        </h2>
        <button type="button" onClick={onOpenSettings} className="chip flex items-center gap-1">
          <Settings className="w-3 h-3" />
          {useAi ? `AI: ${ai.modelName || defaultModel(ai.provider)}` : "Mode lokal (tanpa AI)"}
        </button>
      </div>

      <div className="grid gap-2 md:grid-cols-2">
        <input
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="Link chapter komik (Webtoon / MangaDex / Shinigami / lainnya)"
          className="field"
          disabled={running}
        />
        <input
          value={song}
          onChange={(e) => setSong(e.target.value)}
          placeholder="Lagu (opsional): kosongkan = otomatis pilih yang cocok dengan panel"
          className="field"
          disabled={running}
        />
      </div>

      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-3">
          <div className="flex items-center gap-1 rounded-lg border border-zinc-800 bg-zinc-950 p-0.5 text-xs">
            {PANEL_STYLES.map(({ id, label, icon: Icon }) => {
              const active = panelStyle === id;
              return (
                <button
                  key={id}
                  type="button"
                  disabled={running}
                  onClick={() => onChangePanelStyle(id)}
                  className={`flex items-center gap-1.5 rounded-md px-2 py-1 transition ${
                    active
                      ? "bg-violet-600/25 text-violet-200 font-medium"
                      : "text-zinc-400 hover:text-zinc-200"
                  }`}
                >
                  <Icon className={`h-3.5 w-3.5 shrink-0 ${active ? "text-violet-300" : "text-zinc-500"}`} />
                  <span>{label}</span>
                </button>
              );
            })}
          </div>

          <label className="flex items-center gap-2 text-[11px] text-zinc-400">
            <input
              type="checkbox"
              checked={render}
              onChange={(e) => setRender(e.target.checked)}
              disabled={running}
              className="accent-fuchsia-500"
            />
            Render MP4 otomatis ke folder Videos
          </label>
        </div>
        <button
          type="button"
          onClick={go}
          disabled={running || !url.trim()}
          className="btn-primary"
        >
          {running ? <Loader2 className="w-4 h-4 animate-spin" /> : <Sparkles className="w-4 h-4" />}
          {running ? "Memproses..." : useAi ? "Jalankan Otomatis dengan AI" : "Jalankan Otomatis"}
        </button>
      </div>

      {(running || stepIdx >= 0) && (
        <ol className="grid gap-1.5 sm:grid-cols-4 2xl:grid-cols-5">
          {steps.map((name, i) => {
            const done = stepIdx > i;
            const active = running && stepIdx === i;
            return (
              <li
                key={name}
                className={`flex items-center gap-1.5 rounded-lg border px-2 py-1.5 text-[11px] ${
                  active
                    ? "border-fuchsia-500/60 bg-fuchsia-500/10 text-fuchsia-200"
                    : done
                    ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-300"
                    : "border-white/10 bg-white/5 text-zinc-500"
                }`}
              >
                {done ? <Check className="w-3 h-3" /> : active ? <Loader2 className="w-3 h-3 animate-spin" /> : <span className="w-3" />}
                {name}
              </li>
            );
          })}
        </ol>
      )}
      {running && stepLabel && <p className="text-[11px] text-zinc-400">{stepLabel}</p>}
      {error && (
        <p className="flex items-start gap-1.5 text-[11px] text-rose-300">
          <AlertCircle className="mt-0.5 h-3 w-3 shrink-0" />
          {error}
        </p>
      )}
    </div>
  );
};
