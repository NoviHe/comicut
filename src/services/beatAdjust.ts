import { BeatMap } from "../types/project";

export interface BeatAdjust {
  offsetMs: number;
  mode: "half" | "normal" | "double";
  /** Manual drop position (sec) set from the playhead; null = auto-detected. */
  dropSec: number | null;
}

export const NO_ADJUST: BeatAdjust = { offsetMs: 0, mode: "normal", dropSec: null };

/** Client-side correction of a detected beat map: shift, thin out/densify, and move the drop. */
export function applyAdjust(bm: BeatMap, a: BeatAdjust): BeatMap {
  let beats = bm.beats.map((b) => ({ ...b, isBassDrop: false }));
  if (a.mode === "half") beats = beats.filter((_, i) => i % 2 === 0);
  if (a.mode === "double") {
    beats = beats.flatMap((b, i) => {
      const n = beats[i + 1];
      return n
        ? [
            b,
            {
              ...b,
              timestampSec: (b.timestampSec + n.timestampSec) / 2,
              intensity: b.intensity * 0.6,
              isSubBassHit: false,
            },
          ]
        : [b];
    });
  }
  const off = a.offsetMs / 1000;
  beats = beats
    .map((b) => ({ ...b, timestampSec: b.timestampSec + off }))
    .filter((b) => b.timestampSec >= 0 && b.timestampSec < bm.durationSec);

  const drop = a.dropSec ?? bm.mainDropSec + off;
  let best = -1;
  beats.forEach((b, i) => {
    if (best < 0 || Math.abs(b.timestampSec - drop) < Math.abs(beats[best].timestampSec - drop)) best = i;
  });
  if (best >= 0) {
    beats[best].isBassDrop = true;
    beats[best].isSubBassHit = true;
  }
  const mult = a.mode === "half" ? 0.5 : a.mode === "double" ? 2 : 1;
  return { ...bm, beats, mainDropSec: best >= 0 ? beats[best].timestampSec : drop, bpm: bm.bpm * mult };
}

/** Cuts a song can carry: one per 2 beats before the drop, one per beat after (matches the Rust timeline). */
export function targetPanelCount(bm: BeatMap): number {
  const pre = bm.beats.filter((b) => b.timestampSec < bm.mainDropSec - 0.05).length;
  const post = bm.beats.length - pre;
  return Math.max(4, 1 + Math.ceil(pre / 2) + post);
}

/** Local mood guess from slicer metrics -> search queries (used when no AI is set or the AI call fails). */
export function localSongQueries(panels: { edgeDensity: number; contrastScore: number; heuristicScore: number }[]): string[] {
  const n = Math.max(1, panels.length);
  const energy = panels.reduce((a, p) => a + p.edgeDensity * 0.5 + p.contrastScore * 0.3 + p.heuristicScore * 0.2, 0) / n;
  return energy >= 0.5
    ? ["Montagem Coral phonk", "Brazilian phonk aura", "Montagem Tomada", "Montagem Mysterious Game"]
    : ["Montagem Xonada slowed", "Montagem Mysterious Game", "slowed phonk aura", "Montagem Coral phonk"];
}

/** How well a song's beat count carries `panelCount` panels: 1 = perfect, 0 = hopeless. Also needs a usable drop. */
export function songFit(bm: BeatMap, panelCount: number): number {
  const ratio = targetPanelCount(bm) / Math.max(1, panelCount);
  const lenFit = Math.exp(-Math.abs(Math.log(ratio)) * 1.5);
  const dropPos = bm.mainDropSec / Math.max(1, bm.durationSec);
  const dropFit = dropPos > 0.1 && dropPos < 0.7 ? 1 : 0.7;
  return lenFit * dropFit;
}
