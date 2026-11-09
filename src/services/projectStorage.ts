import { BeatAdjust, NO_ADJUST } from "./beatAdjust";
import { BeatMap, Panel, TimelineClip } from "../types/project";

export interface SavedProject {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  chapterTitle: string;
  panels: Panel[];
  rawBeatMap: BeatMap | null;
  adjust: BeatAdjust;
  timeline: TimelineClip[];
  lastOutputPath?: string;
}

const STORAGE_KEY = "comicut.saved_projects.v1";

export function loadSavedProjects(): SavedProject[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const list: SavedProject[] = JSON.parse(raw);
    return Array.isArray(list)
      ? list.sort((a, b) => (b.updatedAt || 0) - (a.updatedAt || 0))
      : [];
  } catch (err) {
    console.error("[projectStorage] Failed to read saved projects:", err);
    return [];
  }
}

export function saveProject(project: SavedProject): void {
  if (typeof window === "undefined") return;
  try {
    const list = loadSavedProjects();
    const idx = list.findIndex((p) => p.id === project.id);
    const updated = {
      ...project,
      updatedAt: Date.now(),
    };
    if (idx >= 0) {
      list[idx] = updated;
    } else {
      list.unshift(updated);
    }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(list));
  } catch (err) {
    console.error("[projectStorage] Failed to save project:", err);
  }
}

export function deleteSavedProject(id: string): SavedProject[] {
  if (typeof window === "undefined") return [];
  try {
    const list = loadSavedProjects().filter((p) => p.id !== id);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(list));
    return list;
  } catch (err) {
    console.error("[projectStorage] Failed to delete project:", err);
    return loadSavedProjects();
  }
}

export function duplicateSavedProject(id: string): SavedProject | null {
  const list = loadSavedProjects();
  const target = list.find((p) => p.id === id);
  if (!target) return null;

  const clone: SavedProject = {
    ...target,
    id: `proj_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`,
    title: `${target.title} (Salinan)`,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  };

  saveProject(clone);
  return clone;
}

export function buildProjectSnapshot(params: {
  id?: string;
  title?: string;
  chapterTitle: string;
  panels: Panel[];
  rawBeatMap: BeatMap | null;
  adjust?: BeatAdjust;
  timeline: TimelineClip[];
  lastOutputPath?: string;
}): SavedProject {
  const title =
    params.title?.trim() ||
    params.chapterTitle?.trim() ||
    (params.rawBeatMap
      ? `Proyek - ${params.rawBeatMap.fileName.replace(/\.[^/.]+$/, "")}`
      : `Proyek #${new Date().toLocaleDateString("id-ID")}`);

  return {
    id:
      params.id ||
      `proj_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`,
    title,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    chapterTitle: params.chapterTitle,
    panels: params.panels,
    rawBeatMap: params.rawBeatMap,
    adjust: params.adjust || NO_ADJUST,
    timeline: params.timeline,
    lastOutputPath: params.lastOutputPath,
  };
}
