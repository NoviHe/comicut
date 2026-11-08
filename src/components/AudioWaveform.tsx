import React, { useState } from "react";
import { Music, Zap, Radio, Activity, Loader2 } from "lucide-react";
import { BeatMap, BeatMarker } from "../types/project";
import { analyzeAudioBeats } from "../services/tauriCommands";

interface AudioWaveformProps {
  beatMap: BeatMap | null;
  onBeatMapLoaded: (beatMap: BeatMap) => void;
  onStatusMessage: (msg: string) => void;
}

export const AudioWaveform: React.FC<AudioWaveformProps> = ({
  beatMap,
  onBeatMapLoaded,
  onStatusMessage,
}) => {
  const [audioPath, setAudioPath] = useState<string>("");
  const [isAnalyzing, setIsAnalyzing] = useState<boolean>(false);

  const handleAnalyzeAudio = async () => {
    if (!audioPath.trim()) return;
    setIsAnalyzing(true);
    onStatusMessage(
      `Menganalisis spektrum frekuensi bass (40Hz-150Hz) dari ${audioPath}...`
    );
    try {
      const result = await analyzeAudioBeats(audioPath.trim());
      onBeatMapLoaded(result);
      onStatusMessage(
        `Beat Map selesai! BPM: ${result.bpm.toFixed(
          0
        )}, Main Bass Drop pada ${result.mainDropSec.toFixed(2)}s (${
          result.beats.length
        } ketukan).`
      );
    } catch (err: any) {
      onStatusMessage(`Gagal analisis audio: ${err?.message || String(err)}`);
    } finally {
      setIsAnalyzing(false);
    }
  };

  const handleLoadSampleMontagem = () => {
    const beats: BeatMarker[] = [
      { timestampSec: 0.0, intensity: 0.45, isBassDrop: false, isSubBassHit: false },
      { timestampSec: 1.4, intensity: 0.55, isBassDrop: false, isSubBassHit: false },
      { timestampSec: 2.8, intensity: 0.72, isBassDrop: false, isSubBassHit: false },
      { timestampSec: 3.6, intensity: 1.0, isBassDrop: true, isSubBassHit: true },
      { timestampSec: 4.5, intensity: 0.94, isBassDrop: false, isSubBassHit: true },
      { timestampSec: 5.4, intensity: 0.91, isBassDrop: false, isSubBassHit: true },
      { timestampSec: 6.3, intensity: 0.96, isBassDrop: false, isSubBassHit: true },
      { timestampSec: 7.2, intensity: 0.89, isBassDrop: false, isSubBassHit: true },
      { timestampSec: 8.1, intensity: 0.95, isBassDrop: false, isSubBassHit: true },
    ];

    const peaks = Array.from({ length: 48 }, (_, i) => {
      const sec = (i / 48) * 9.0;
      if (sec < 3.5) return 0.25 + Math.sin(i * 0.7) * 0.15;
      return 0.65 + Math.abs(Math.sin(i * 1.9)) * 0.35;
    });

    const sampleBeatMap: BeatMap = {
      audioPath: "sample://MONTAGEM_CORAL_REMIX_135BPM.mp3",
      fileName: "MONTAGEM_CORAL_REMIX_135BPM.mp3",
      durationSec: 9.0,
      bpm: 135,
      mainDropSec: 3.6,
      beats,
      waveformPeaks: peaks,
    };

    onBeatMapLoaded(sampleBeatMap);
    onStatusMessage(
      "Preset Audio Montagem Phonk (135 BPM, Bass Drop @ 3.60s) berhasil dimuat!"
    );
  };

  return (
    <div className="bg-zinc-900/90 border border-zinc-800 rounded-xl p-4 space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Music className="w-4 h-4 text-emerald-400" />
          <h2 className="text-sm font-semibold text-zinc-100">
            3. Audio Beat Engine (Montagem / Phonk / Remix)
          </h2>
        </div>
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
          placeholder="Path file lagu (.mp3 / .wav) misal D:\Music\montagem_phonk.mp3"
          className="flex-1 bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-600 focus:outline-none focus:border-emerald-500"
        />
        <button
          type="button"
          disabled={isAnalyzing}
          onClick={handleAnalyzeAudio}
          className="bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-white text-xs font-semibold px-3.5 py-2 rounded-lg flex items-center gap-1.5 transition"
        >
          {isAnalyzing ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <Activity className="w-3.5 h-3.5" />
          )}
          Deteksi Bass
        </button>
        <button
          type="button"
          onClick={handleLoadSampleMontagem}
          className="bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs font-medium px-3 py-2 rounded-lg flex items-center gap-1.5 transition"
        >
          <Radio className="w-3.5 h-3.5 text-emerald-400" />
          Sample Montagem
        </button>
      </div>

      {/* Waveform & Sub-Bass Drop Visualizer */}
      {beatMap ? (
        <div className="bg-zinc-950 border border-zinc-800/90 rounded-lg p-3 space-y-2">
          <div className="flex items-center justify-between text-[11px] text-zinc-400">
            <span className="font-medium text-zinc-200 truncate">
              {beatMap.fileName}
            </span>
            <span className="font-mono">
              Durasi: {beatMap.durationSec.toFixed(1)}s • {beatMap.beats.length}{" "}
              Beat Hits
            </span>
          </div>

          <div className="relative h-16 flex items-end gap-1 pt-4 pb-1 px-1">
            {beatMap.waveformPeaks.map((peak, idx) => {
              const sec = (idx / beatMap.waveformPeaks.length) * beatMap.durationSec;
              const isDropZone = sec >= beatMap.mainDropSec;
              return (
                <div
                  key={idx}
                  style={{ height: `${Math.max(15, Math.round(peak * 100))}%` }}
                  className={`flex-1 rounded-t transition-all ${
                    isDropZone
                      ? "bg-gradient-to-t from-rose-600 to-amber-400"
                      : "bg-zinc-700"
                  }`}
                  title={`${sec.toFixed(2)}s`}
                />
              );
            })}

            {/* Garis Penanda Main Bass Drop */}
            <div
              style={{
                left: `${Math.min(
                  95,
                  Math.max(5, (beatMap.mainDropSec / beatMap.durationSec) * 100)
                )}%`,
              }}
              className="absolute top-0 bottom-0 w-0.5 bg-rose-500 shadow-[0_0_8px_#f43f5e]"
            >
              <span className="absolute -top-0.5 -translate-x-1/2 bg-rose-600 text-white text-[9px] font-bold px-1.5 py-0.2 rounded">
                BASS DROP
              </span>
            </div>
          </div>
        </div>
      ) : (
        <div className="bg-zinc-950/60 border border-zinc-800/60 rounded-lg p-3 text-center text-[11px] text-zinc-500">
          Pilih lagu <b>Montagem / Phonk</b> atau klik <b>Sample Montagem</b>{" "}
          untuk mendeteksi titik <i>Sub-Bass Drop</i> &amp; ketukan otomatis.
        </div>
      )}
    </div>
  );
};
