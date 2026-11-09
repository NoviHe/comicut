import React, { useState } from "react";
import {
  Clock,
  Copy,
  Download,
  Edit3,
  Film,
  Folder,
  Music,
  Plus,
  RefreshCw,
  Search,
  Trash2,
  CheckCircle2,
  Loader2,
} from "lucide-react";
import {
  deleteSavedProject,
  duplicateSavedProject,
  SavedProject,
  saveProject,
} from "../services/projectStorage";
import { panelImageSrc, defaultOutputPath, renderMontagemVideo } from "../services/tauriCommands";
import { RenderConfig } from "../types/project";
import { applyAdjust } from "../services/beatAdjust";

interface ProjectsManagerProps {
  projects: SavedProject[];
  currentProjectId?: string;
  hasCurrentWork: boolean;
  onRefreshProjects: () => void;
  onOpenProject: (project: SavedProject) => void;
  onNewProject: () => void;
  onSaveCurrentProject: () => void;
  onStatusMessage: (msg: string) => void;
}

export const ProjectsManager: React.FC<ProjectsManagerProps> = ({
  projects,
  currentProjectId,
  hasCurrentWork,
  onRefreshProjects,
  onOpenProject,
  onNewProject,
  onSaveCurrentProject,
  onStatusMessage,
}) => {
  const [search, setSearch] = useState("");
  const [renderingId, setRenderingId] = useState<string | null>(null);
  const [editingTitleId, setEditingTitleId] = useState<string | null>(null);
  const [tempTitle, setTempTitle] = useState("");
  const [projectToDelete, setProjectToDelete] = useState<SavedProject | null>(null);

  const filtered = projects.filter((p) => {
    const q = search.toLowerCase();
    return (
      p.title.toLowerCase().includes(q) ||
      (p.chapterTitle && p.chapterTitle.toLowerCase().includes(q)) ||
      (p.rawBeatMap?.fileName && p.rawBeatMap.fileName.toLowerCase().includes(q))
    );
  });

  const handleConfirmDelete = () => {
    if (!projectToDelete) return;
    deleteSavedProject(projectToDelete.id);
    onRefreshProjects();
    onStatusMessage(`Proyek "${projectToDelete.title}" telah dihapus.`);
    setProjectToDelete(null);
  };

  const handleDuplicate = (id: string) => {
    const cloned = duplicateSavedProject(id);
    if (cloned) {
      onRefreshProjects();
      onStatusMessage(`Proyek diduplikasi: "${cloned.title}"`);
    }
  };

  const handleStartRename = (project: SavedProject) => {
    setEditingTitleId(project.id);
    setTempTitle(project.title);
  };

  const handleSaveRename = (project: SavedProject) => {
    if (tempTitle.trim() && tempTitle.trim() !== project.title) {
      saveProject({ ...project, title: tempTitle.trim() });
      onRefreshProjects();
      onStatusMessage(`Nama proyek diubah menjadi "${tempTitle.trim()}".`);
    }
    setEditingTitleId(null);
  };

  const handleQuickRender = async (project: SavedProject) => {
    if (!project.rawBeatMap || project.timeline.length === 0) {
      onStatusMessage("Proyek belum memiliki audio atau timeline untuk dirender.");
      return;
    }

    setRenderingId(project.id);
    onStatusMessage(`Memulai render ulang untuk "${project.title}"...`);

    try {
      const activeBeatMap = applyAdjust(project.rawBeatMap, project.adjust);
      const out = project.lastOutputPath || (await defaultOutputPath());
      const config: RenderConfig = {
        outputPath: out,
        width: 1080,
        height: 1920,
        fps: 60,
        blurBackground: true,
        vignetteStrength: 0.35,
        beatFlashEnabled: true,
      };

      const saved = await renderMontagemVideo(
        project.panels,
        activeBeatMap,
        project.timeline,
        config
      );

      // Update project with last output path
      saveProject({ ...project, lastOutputPath: saved });
      onRefreshProjects();
      onStatusMessage(`Render ulang selesai! File tersimpan di: ${saved}`);
    } catch (err: any) {
      onStatusMessage(`Gagal render ulang: ${err?.message || String(err)}`);
    } finally {
      setRenderingId(null);
    }
  };

  const formatDate = (ms: number) => {
    const d = new Date(ms);
    return d.toLocaleDateString("id-ID", {
      day: "numeric",
      month: "short",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  return (
    <div className="relative z-10 flex flex-1 flex-col overflow-y-auto p-5 space-y-4">
      {/* Top Header Controls */}
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-white/[0.06] pb-4">
        <div>
          <h2 className="text-base font-bold text-white flex items-center gap-2">
            <Folder className="w-5 h-5 text-violet-400" />
            Riwayat Proyek
          </h2>
          <p className="text-xs text-zinc-400 mt-0.5">
            Kelola, buka kembali untuk diedit, atau render ulang video montagem dari sesi sebelumnya.
          </p>
        </div>

        <div className="flex items-center gap-2">
          {hasCurrentWork && (
            <button
              type="button"
              onClick={onSaveCurrentProject}
              className="btn-ghost text-xs"
              title="Simpan sesi kerja saat ini ke daftar proyek"
            >
              <RefreshCw className="w-3.5 h-3.5 text-emerald-400" />
              Simpan Proyek Aktif
            </button>
          )}

          <button
            type="button"
            onClick={onNewProject}
            className="btn-primary text-xs"
          >
            <Plus className="w-4 h-4" />
            Proyek Baru
          </button>
        </div>
      </div>

      {/* Search & Statistics Bar */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Cari berdasarkan judul, nama chapter, atau lagu..."
            className="field pl-9 text-xs"
          />
        </div>

        <div className="flex items-center gap-2 text-xs text-zinc-400">
          <span className="chip font-mono">
            {filtered.length} {filtered.length === 1 ? "proyek" : "proyek"}
          </span>
        </div>
      </div>

      {/* Projects List */}
      {filtered.length === 0 ? (
        <div className="card flex flex-col items-center justify-center p-12 text-center gap-3">
          <div className="grid h-12 w-12 place-items-center rounded-2xl bg-white/5 border border-white/10 text-zinc-400">
            <Film className="w-6 h-6 text-zinc-500" />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-zinc-200">
              {search ? "Tidak ada proyek yang sesuai" : "Belum ada riwayat proyek"}
            </h3>
            <p className="text-xs text-zinc-500 max-w-sm mt-1">
              {search
                ? `Tidak ditemukan proyek dengan kata kunci "${search}".`
                : "Mulai buat video montagem di tab Home atau klik Proyek Baru di atas untuk memulai."}
            </p>
          </div>
          {!search && (
            <button
              type="button"
              onClick={onNewProject}
              className="btn-primary text-xs mt-2"
            >
              <Plus className="w-3.5 h-3.5" />
              Mulai Proyek Pertama
            </button>
          )}
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3">
          {filtered.map((proj) => {
            const isCurrent = proj.id === currentProjectId;
            const isRenderingThis = renderingId === proj.id;
            const firstPanel = proj.panels.find((p) => p.selected) || proj.panels[0];
            const thumbSrc = firstPanel ? panelImageSrc(firstPanel) : undefined;
            const selectedCount = proj.panels.filter((p) => p.selected).length;
            const durationSec = proj.timeline.length
              ? proj.timeline[proj.timeline.length - 1].endSec
              : proj.rawBeatMap?.durationSec || 0;

            return (
              <div
                key={proj.id}
                className={`card flex flex-col sm:flex-row items-start sm:items-center gap-4 transition hover:border-violet-500/30 ${
                  isCurrent ? "border-violet-500/50 bg-violet-950/10 shadow-lg shadow-violet-950/20" : ""
                }`}
              >
                {/* Thumbnail Preview */}
                <div className="relative aspect-[9/16] w-20 sm:w-24 shrink-0 overflow-hidden rounded-xl bg-zinc-950 border border-white/10 shadow-inner flex items-center justify-center">
                  {thumbSrc ? (
                    <img
                      src={thumbSrc}
                      alt={proj.title}
                      className="h-full w-full object-cover"
                    />
                  ) : (
                    <div className="p-2 text-center text-[10px] text-zinc-600 font-mono">
                      No Preview
                    </div>
                  )}

                  {/* Duration overlay badge */}
                  {durationSec > 0 && (
                    <span className="absolute bottom-1 right-1 rounded bg-black/80 px-1 py-0.5 text-[9px] font-mono font-medium text-zinc-300">
                      {durationSec.toFixed(1)}s
                    </span>
                  )}

                  {isCurrent && (
                    <span className="absolute top-1 left-1 rounded bg-violet-600 px-1 py-0.5 text-[8px] font-bold text-white tracking-wider uppercase">
                      Aktif
                    </span>
                  )}
                </div>

                {/* Project Details */}
                <div className="flex-1 min-w-0 space-y-2">
                  <div className="flex items-center gap-2">
                    {editingTitleId === proj.id ? (
                      <div className="flex items-center gap-2">
                        <input
                          type="text"
                          value={tempTitle}
                          onChange={(e) => setTempTitle(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") handleSaveRename(proj);
                            if (e.key === "Escape") setEditingTitleId(null);
                          }}
                          className="field py-1 text-sm font-semibold text-white max-w-xs"
                          autoFocus
                        />
                        <button
                          type="button"
                          onClick={() => handleSaveRename(proj)}
                          className="btn-primary py-1 px-2 text-xs"
                        >
                          Simpan
                        </button>
                      </div>
                    ) : (
                      <>
                        <h3 className="text-sm font-bold text-white truncate hover:text-violet-300 transition cursor-pointer"
                          onClick={() => handleStartRename(proj)}
                          title="Klik untuk mengubah nama"
                        >
                          {proj.title}
                        </h3>
                        <button
                          type="button"
                          onClick={() => handleStartRename(proj)}
                          className="text-zinc-500 hover:text-zinc-300 transition"
                          title="Ubah nama proyek"
                        >
                          <Edit3 className="w-3 h-3" />
                        </button>
                      </>
                    )}

                    {proj.lastOutputPath && (
                      <span
                        className="inline-flex items-center gap-1 rounded-md bg-emerald-500/10 border border-emerald-500/20 px-1.5 py-0.5 text-[10px] text-emerald-300 font-medium shrink-0"
                        title={`Tersimpan: ${proj.lastOutputPath}`}
                      >
                        <CheckCircle2 className="w-3 h-3 text-emerald-400" />
                        Telah Dirender
                      </span>
                    )}
                  </div>

                  {/* Metadata Chips */}
                  <div className="flex flex-wrap items-center gap-1.5 text-[11px] text-zinc-400">
                    {proj.chapterTitle && (
                      <span className="chip border-white/10 bg-zinc-900/80 text-zinc-300">
                        📖 {proj.chapterTitle}
                      </span>
                    )}

                    {proj.rawBeatMap && (
                      <span className="chip border-white/10 bg-zinc-900/80 text-zinc-300 flex items-center gap-1">
                        <Music className="w-3 h-3 text-sky-400" />
                        <span className="truncate max-w-[140px]" title={proj.rawBeatMap.fileName}>
                          {proj.rawBeatMap.fileName}
                        </span>
                        <span>• {proj.rawBeatMap.bpm} BPM</span>
                      </span>
                    )}

                    <span className="chip border-white/10 bg-zinc-900/80 text-zinc-300">
                      🖼️ {selectedCount}/{proj.panels.length} panel
                    </span>

                    <span className="chip border-white/10 bg-zinc-900/80 text-zinc-300">
                      ✂️ {proj.timeline.length} klip
                    </span>

                    <span className="flex items-center gap-1 text-[10px] text-zinc-500 ml-auto">
                      <Clock className="w-3 h-3" />
                      {formatDate(proj.updatedAt)}
                    </span>
                  </div>
                </div>

                {/* Action Buttons */}
                <div className="flex sm:flex-col items-center sm:items-end gap-1.5 shrink-0 w-full sm:w-auto pt-2 sm:pt-0 border-t sm:border-t-0 border-white/[0.06]">
                  <div className="flex items-center gap-1.5 w-full sm:w-auto">
                    <button
                      type="button"
                      onClick={() => onOpenProject(proj)}
                      className="btn-primary py-1.5 px-3 text-xs flex-1 sm:flex-initial"
                      title="Buka proyek di editor untuk diubah"
                    >
                      <Edit3 className="w-3.5 h-3.5" />
                      Buka &amp; Edit
                    </button>

                    <button
                      type="button"
                      disabled={isRenderingThis || proj.timeline.length === 0}
                      onClick={() => handleQuickRender(proj)}
                      className="btn-ghost py-1.5 px-3 text-xs flex-1 sm:flex-initial"
                      title="Render ulang langsung menjadi video MP4 1080x1920"
                    >
                      {isRenderingThis ? (
                        <>
                          <Loader2 className="w-3.5 h-3.5 animate-spin text-violet-400" />
                          <span>Merender...</span>
                        </>
                      ) : (
                        <>
                          <Download className="w-3.5 h-3.5 text-violet-400" />
                          <span>Render Ulang</span>
                        </>
                      )}
                    </button>

                    <button
                      type="button"
                      onClick={() => handleDuplicate(proj.id)}
                      className="btn-ghost p-2"
                      title="Duplikasi proyek"
                    >
                      <Copy className="w-3.5 h-3.5 text-zinc-400" />
                    </button>

                    <button
                      type="button"
                      onClick={() => setProjectToDelete(proj)}
                      className="btn-ghost p-2 text-rose-400 hover:text-rose-300 hover:bg-rose-500/10"
                      title="Hapus proyek"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Custom Sleek In-App Delete Confirmation Modal */}
      {projectToDelete && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm"
          onClick={() => setProjectToDelete(null)}
        >
          <div
            className="w-full max-w-sm rounded-2xl border border-white/10 bg-[#121216] p-5 shadow-2xl space-y-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-start gap-3.5">
              <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl border border-rose-500/20 bg-rose-500/10 text-rose-400">
                <Trash2 className="h-5 w-5" />
              </div>
              <div className="min-w-0 flex-1">
                <h3 className="text-sm font-bold text-white leading-tight">
                  Hapus Proyek?
                </h3>
                <p className="mt-1.5 text-xs text-zinc-400 leading-relaxed">
                  Proyek <span className="font-semibold text-zinc-200">"{projectToDelete.title}"</span> akan dihapus dari riwayat lokal. Tindakan ini tidak dapat dibatalkan.
                </p>
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-white/[0.06]">
              <button
                type="button"
                onClick={() => setProjectToDelete(null)}
                className="btn-ghost py-1.5 px-3 text-xs text-zinc-400 hover:text-white"
              >
                Batal
              </button>
              <button
                type="button"
                onClick={handleConfirmDelete}
                className="flex items-center gap-1.5 rounded-lg bg-rose-600 px-3.5 py-1.5 text-xs font-semibold text-white shadow-lg shadow-rose-950/50 transition hover:bg-rose-500 active:scale-95"
              >
                <Trash2 className="w-3.5 h-3.5" />
                <span>Hapus Proyek</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
