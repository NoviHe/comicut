import React, { useState } from "react";
import {
  Activity,
  Loader2,
  Music,
  Pause,
  Play,
  Radio,
  Target,
  Volume2,
  Zap,
} from "lucide-react";
import { BeatMap, BeatMarker } from "../types/project";
import { analyzeAudioBeats, downloadAudio } from "../services/tauriCommands";
import { BeatAdjust } from "../services/beatAdjust";
import { Playback } from "../services/usePlayback";

interface AudioWaveformProps {
  beatMap: BeatMap | null;
  adjust: BeatAdjust;
  onAdjust: (a: BeatAdjust) => void;
  playback: Playback;
  onBeatMapLoaded: (beatMap: BeatMap) => void;
  onStatusMessage: (msg: string) => void;
}

// CapCut-style one-click montagem presets (searched on YouTube via yt-dlp).
const MONTAGEM_PRESETS = [
  "Montagem Coral phonk",
  "Montagem Xonada slowed",
  "Montagem Mysterious Game",
  "Montagem Tomada",
  "Brazilian phonk aura",
];

const isLocalAudioFile = (v: string) =>
  /\.(mp3|wav|m4a|aac|ogg|flac)$/i.test(v) && !/^https?:\/\//i.test(v);

const MODES: { id: BeatAdjust["mode"]; label: string }[] = [
  { id: "half", label: "½ Lambat" },
  { id: "normal", label: "Normal" },
  { id: "double", label: "2× Cepat" },
];

export const AudioWaveform: React.FC<AudioWaveformProps> = ({
  beatMap,
  adjust,
  onAdjust,
  playback,
  onBeatMapLoaded,
  onStatusMessage,
}) => {
  const [audioPath, setAudioPath] = useState<string>("");
  const [isAnalyzing, setIsAnalyzing] = useState<boolean>(false);

  const handleAnalyzeAudio = async (override?: string) => {
    const input = (override ?? audioPath).trim();
    if (!input) return;
    setIsAnalyzing(true);
    try {
      let file = input;
      if (!isLocalAudioFile(input)) {
        onStatusMessage(`Mengunduh audio via yt-dlp: ${input}...`);
        file = await downloadAudio(input);
        setAudioPath(file);
      }
      onStatusMessage(`Menganalisis ketukan & bass dari ${file}...`);
      const result = await analyzeAudioBeats(file);
      onBeatMapLoaded(result);
      onStatusMessage(
        `Beat Map selesai! ${result.bpm.toFixed(0)} BPM, drop @ ${result.mainDropSec.toFixed(
          2
        )}s, ${result.beats.length} ketukan. Putar lalu nyalakan Metronom untuk cek kecocokan.`
      );
    } catch (err: any) {
      onStatusMessage(`Gagal analisis audio: ${err?.message || String(err)}`);
    } finally {
      setIsAnalyzing(false);
    }
  };

  const handleLoadSampleMontagem = () => {
    const beats: BeatMarker[] = [0, 1.4, 2.8, 3.6, 4.5, 5.4, 6.3, 7.2, 8.1].map(
      (t, i) => ({
        timestampSec: t,
        intensity: i < 3 ? 0.5 + i * 0.1 : 0.93,
        isBassDrop: t === 3.6,
        isSubBassHit: i >= 3,
      })
    );
    const peaks = Array.from({ length: 96 }, (_, i) => {
      const sec = (i / 96) * 9.0;
      return sec < 3.5
        ? 0.25 + Math.abs(Math.sin(i * 0.7)) * 0.15
        : 0.65 + Math.abs(Math.sin(i * 1.9)) * 0.35;
    });
    onBeatMapLoaded({
      audioPath: "sample://MONTAGEM_CORAL_REMIX_135BPM.mp3",
      fileName: "MONTAGEM_CORAL_REMIX_135BPM.mp3 (sample, tanpa suara)",
      durationSec: 9.0,
      bpm: 135,
      mainDropSec: 3.6,
      beats,
      waveformPeaks: peaks,
    });
    onStatusMessage("Sample Montagem (135 BPM, drop @ 3.60s) dimuat. Sample tidak punya suara.");
  };

  const pct = (sec: number) =>
    beatMap ? `${Math.min(100, Math.max(0, (sec / beatMap.durationSec) * 100))}%` : "0%";

  const seekFromClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!beatMap) return;
    const r = e.currentTarget.getBoundingClientRect();
    playback.seek(((e.clientX - r.left) / r.width) * beatMap.durationSec);
  };

  return (
    <div className="card space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="card-title">
          <span className="step">3</span>
          <Music className="w-4 h-4 text-emerald-400" />
          Audio &amp; Beat Montagem
        </h2>
        {beatMap && (
          <div className="flex items-center gap-2 text-[11px]">
            <span className="bg-emerald-500/15 text-emerald-300 border border-emerald-500/30 px-2 py-0.5 rounded font-mono">
              {beatMap.bpm} BPM
            </span>
            <span className="bg-rose-500/15 text-rose-300 border border-rose-500/30 px-2 py-0.5 rounded font-mono flex items-center gap-1">
              <Zap className="w-3 h-3" />
              Drop @ {beatMap.mainDropSec.toFixed(2)}s
            </span>
          </div>
        )}
      </div>

      <div className="flex gap-2">
        <input
          type="text"
          value={audioPath}
          onChange={(e) => setAudioPath(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleAnalyzeAudio()}
          placeholder="Path file lagu, link YouTube/TikTok/SoundCloud, atau kata kunci (misal: montagem coral)"
          className="field flex-1"
        />
        <button
          type="button"
          disabled={isAnalyzing}
          onClick={() => handleAnalyzeAudio()}
          className="btn bg-emerald-600 hover:bg-emerald-500 text-white"
        >
          {isAnalyzing ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <Activity className="w-3.5 h-3.5" />
          )}
          Deteksi Beat
        </button>
        <button type="button" onClick={handleLoadSampleMontagem} className="btn-ghost">
          <Radio className="w-3.5 h-3.5 text-emerald-400" />
          Sample
        </button>
      </div>

      <div className="flex flex-wrap items-center gap-1.5">
        <span className="text-[10px] text-zinc-500">Preset Montagem:</span>
        {MONTAGEM_PRESETS.map((q) => (
          <button
            key={q}
            type="button"
            disabled={isAnalyzing}
            onClick={() => {
              setAudioPath(q);
              handleAnalyzeAudio(q);
            }}
            className="chip"
          >
            {q}
          </button>
        ))}
      </div>

      {beatMap ? (
        <div className="rounded-xl border border-white/[0.06] bg-zinc-950/70 p-3 space-y-3">
          <div className="flex items-center justify-between gap-3 text-[11px] text-zinc-400">
            <div className="flex items-center gap-2 min-w-0">
              <button
                type="button"
                onClick={playback.toggle}
                className="grid h-7 w-7 shrink-0 place-items-center rounded-full bg-emerald-600 hover:bg-emerald-500 text-white transition"
                aria-label={playback.playing ? "Pause" : "Putar"}
              >
                {playback.playing ? (
                  <Pause className="w-3.5 h-3.5" />
                ) : (
                  <Play className="w-3.5 h-3.5 ml-0.5" />
                )}
              </button>
              <span className="font-medium text-zinc-200 truncate">{beatMap.fileName}</span>
            </div>
            <span className="font-mono shrink-0">
              {playback.time.toFixed(1)}s / {beatMap.durationSec.toFixed(1)}s •{" "}
              {beatMap.beats.length} beat
            </span>
          </div>

          {/* Waveform: klik untuk seek. Garis = beat terdeteksi, merah = drop. */}
          <div
            className="relative h-24 cursor-pointer select-none"
            onClick={seekFromClick}
            role="slider"
            aria-label="Posisi lagu"
            aria-valuemin={0}
            aria-valuemax={beatMap.durationSec}
            aria-valuenow={playback.time}
          >
            <div className="absolute inset-0 flex items-center gap-px">
              {beatMap.waveformPeaks.map((peak, idx) => {
                const sec = (idx / beatMap.waveformPeaks.length) * beatMap.durationSec;
                const played = sec <= playback.time;
                const dropZone = sec >= beatMap.mainDropSec;
                return (
                  <div
                    key={idx}
                    style={{ height: `${Math.max(6, Math.round(peak * 100))}%` }}
                    className={`flex-1 rounded-full ${
                      played
                        ? dropZone
                          ? "bg-gradient-to-t from-rose-500 to-amber-300"
                          : "bg-emerald-400"
                        : dropZone
                        ? "bg-rose-900/70"
                        : "bg-zinc-700"
                    }`}
                  />
                );
              })}
            </div>
            {beatMap.beats.map((b, i) => (
              <div
                key={i}
                style={{ left: pct(b.timestampSec) }}
                className={`absolute top-0 bottom-0 w-px pointer-events-none ${
                  b.isBassDrop ? "bg-rose-400" : "bg-white/25"
                }`}
              />
            ))}
            <div
              style={{ left: pct(beatMap.mainDropSec) }}
              className="absolute top-0 bottom-0 w-0.5 bg-rose-500 shadow-[0_0_8px_#f43f5e] pointer-events-none"
            >
              <span className="absolute -top-1 -translate-x-1/2 bg-rose-600 text-white text-[9px] font-bold px-1.5 rounded">
                DROP
              </span>
            </div>
            <div
              style={{ left: pct(playback.time) }}
              className="absolute top-0 bottom-0 w-0.5 bg-white pointer-events-none"
            />
          </div>

          {/* Penyesuaian jika deteksi kurang pas */}
          <div className="grid gap-3 md:grid-cols-3 text-[11px] text-zinc-400">
            <label className="block">
              Geser beat: {adjust.offsetMs > 0 ? "+" : ""}
              {adjust.offsetMs} ms
              <input
                type="range"
                min={-300}
                max={300}
                step={5}
                value={adjust.offsetMs}
                onChange={(e) => onAdjust({ ...adjust, offsetMs: Number(e.target.value) })}
                className="w-full mt-1 accent-emerald-500"
              />
            </label>
            <div>
              Kerapatan beat
              <div className="mt-1 grid grid-cols-3 gap-1">
                {MODES.map((m) => (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => onAdjust({ ...adjust, mode: m.id })}
                    className={`rounded-md border py-1 transition ${
                      adjust.mode === m.id
                        ? "border-emerald-500 bg-emerald-500/15 text-emerald-200"
                        : "border-white/10 bg-white/5 hover:bg-white/10"
                    }`}
                  >
                    {m.label}
                  </button>
                ))}
              </div>
            </div>
            <div className="flex flex-wrap items-end gap-1.5">
              <button
                type="button"
                onClick={() => playback.setMetronome(!playback.metronome)}
                className={`btn-ghost ${playback.metronome ? "!border-emerald-500 !bg-emerald-500/15 !text-emerald-200" : ""}`}
              >
                <Volume2 className="w-3.5 h-3.5" />
                Metronom {playback.metronome ? "ON" : "OFF"}
              </button>
              <button
                type="button"
                onClick={() => onAdjust({ ...adjust, dropSec: playback.time })}
                className="btn-ghost"
                title="Jadikan posisi playhead sebagai titik drop"
              >
                <Target className="w-3.5 h-3.5 text-rose-400" />
                Drop di sini
              </button>
              {(adjust.offsetMs !== 0 || adjust.mode !== "normal" || adjust.dropSec !== null) && (
                <button
                  type="button"
                  onClick={() => onAdjust({ offsetMs: 0, mode: "normal", dropSec: null })}
                  className="text-[10px] text-zinc-500 hover:text-zinc-200 underline"
                >
                  reset
                </button>
              )}
            </div>
          </div>
          {!playback.hasAudio && (
            <p className="text-[10px] text-amber-400/80">
              File audio tidak bisa diputar (sample / di luar runtime Tauri): preview berjalan tanpa suara.
            </p>
          )}
        </div>
      ) : (
        <div className="rounded-xl border border-white/[0.06] bg-zinc-950/50 p-4 text-center text-[11px] text-zinc-500">
          Pilih preset Montagem, tempel link, atau isi path lagu untuk mendeteksi beat &amp; drop.
        </div>
      )}
    </div>
  );
};
