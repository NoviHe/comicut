import { useCallback, useEffect, useRef, useState } from "react";
import { BeatMap } from "../types/project";
import { audioSrc } from "./tauriCommands";

export interface Playback {
  playing: boolean;
  time: number;
  hasAudio: boolean;
  /** Audible tick on every detected beat, to hear whether the beat map fits the song. */
  metronome: boolean;
  setMetronome: (v: boolean) => void;
  toggle: () => void;
  seek: (t: number) => void;
}

/** One clock for the whole app: real audio when available, wall-clock fallback (silent) otherwise. */
export function usePlayback(
  beatMap: BeatMap | null,
  duration: number,
  onError: (msg: string) => void
): Playback {
  const src = beatMap ? audioSrc(beatMap.audioPath) : undefined;
  const audio = useRef<HTMLAudioElement | null>(null);
  const timeRef = useRef(0);
  const wallBase = useRef(0);
  const beatsRef = useRef<number[]>([]);
  const metroRef = useRef(false);
  const ctx = useRef<AudioContext | null>(null);
  const [playing, setPlaying] = useState(false);
  const [time, setTime] = useState(0);
  const [metronome, setMetro] = useState(false);
  const [hasAudio, setHasAudio] = useState(false);

  beatsRef.current = beatMap ? beatMap.beats.map((b) => b.timestampSec) : [];
  metroRef.current = metronome;

  useEffect(() => {
    setPlaying(false);
    timeRef.current = 0;
    setTime(0);
    if (!src) {
      audio.current = null;
      setHasAudio(false);
      return;
    }
    const a = new Audio(src);
    a.preload = "auto";
    a.onerror = () => {
      audio.current = null;
      setHasAudio(false);
      onError(`Audio preview gagal dimuat: ${src}`);
    };
    audio.current = a;
    setHasAudio(true);
    return () => {
      a.pause();
      a.removeAttribute("src");
    };
  }, [src]);

  const blip = () => {
    const c = (ctx.current ??= new AudioContext());
    const o = c.createOscillator();
    const g = c.createGain();
    o.frequency.value = 1600;
    g.gain.setValueAtTime(0.3, c.currentTime);
    g.gain.exponentialRampToValueAtTime(0.001, c.currentTime + 0.06);
    o.connect(g).connect(c.destination);
    o.start();
    o.stop(c.currentTime + 0.07);
  };

  useEffect(() => {
    if (!playing) return;
    const a = audio.current;
    wallBase.current = performance.now() - timeRef.current * 1000;
    if (a) {
      a.currentTime = timeRef.current;
      a.play().catch((e) => {
        audio.current = null;
        onError(`Audio tidak bisa diputar: ${e?.message || e}`);
      });
    }
    let prev = timeRef.current;
    let lastPaint = 0;
    let raf = 0;
    const step = () => {
      const t =
        a && audio.current === a
          ? a.currentTime
          : (performance.now() - wallBase.current) / 1000;
      if (t >= duration) {
        timeRef.current = 0;
        setTime(0);
        setPlaying(false);
        return;
      }
      if (metroRef.current && beatsRef.current.some((b) => b > prev && b <= t)) {
        blip();
      }
      prev = t;
      timeRef.current = t;
      if (t - lastPaint >= 0.033) {
        lastPaint = t;
        setTime(t);
      }
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => {
      cancelAnimationFrame(raf);
      a?.pause();
    };
  }, [playing, duration]);

  const seek = useCallback(
    (t: number) => {
      const c = Math.max(0, Math.min(duration - 0.01, t));
      timeRef.current = c;
      wallBase.current = performance.now() - c * 1000;
      setTime(c);
      if (audio.current) audio.current.currentTime = c;
    },
    [duration]
  );

  const toggle = useCallback(() => {
    setPlaying((p) => {
      if (!p && timeRef.current >= duration - 0.05) timeRef.current = 0;
      return !p;
    });
  }, [duration]);

  return { playing, time, hasAudio, metronome, setMetronome: setMetro, toggle, seek };
}
