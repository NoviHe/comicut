import React, { useState } from "react";
import {
  ArrowUpCircle,
  Download,
  FolderArchive,
  Loader2,
  PackageCheck,
  Sparkles,
  X,
} from "lucide-react";
import {
  AppUpdateInfo,
  dismissComiCutUpdate,
  installComiCutUpdate,
  PortableUpdateStage,
} from "../services/appUpdater";

interface UpdateModalProps {
  updateInfo: AppUpdateInfo;
  onClose: () => void;
  onStatusMessage?: (msg: string) => void;
}

export const UpdateModal: React.FC<UpdateModalProps> = ({
  updateInfo,
  onClose,
  onStatusMessage,
}) => {
  const [downloading, setDownloading] = useState<boolean>(false);
  const [progress, setProgress] = useState<number>(0);
  const [downloadedBytes, setDownloadedBytes] = useState<number>(0);
  const [totalBytes, setTotalBytes] = useState<number | undefined>(undefined);
  const [stage, setStage] = useState<PortableUpdateStage>("download");
  const [installing, setInstalling] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const isPortable = updateInfo.mode === "portable";

  const handleStartUpdate = async () => {
    setDownloading(true);
    setError(null);
    setStage("download");
    try {
      await installComiCutUpdate(
        updateInfo,
        (percent, downloaded, total, currentStage) => {
          setProgress(percent);
          setDownloadedBytes(downloaded);
          setTotalBytes(total);
          if (currentStage) {
            setStage(currentStage);
          }
          if (
            percent >= 100 ||
            currentStage === "extract" ||
            currentStage === "restart"
          ) {
            setInstalling(true);
          }
        }
      );
    } catch (err: any) {
      console.error("Gagal melakukan update:", err);
      setError(
        typeof err === "string"
          ? err
          : err?.message || "Gagal mengunduh atau memasang pembaruan."
      );
      setDownloading(false);
      setInstalling(false);
      if (onStatusMessage) {
        onStatusMessage(`Update gagal: ${err?.message || err}`);
      }
    }
  };

  const handleDismiss = async () => {
    if (downloading) return;
    await dismissComiCutUpdate(updateInfo);
    onClose();
  };

  const formatMb = (bytes: number) => {
    return (bytes / (1024 * 1024)).toFixed(1) + " MB";
  };

  const stageLabel = () => {
    if (!isPortable) {
      return installing
        ? "Memasang installer & me-restart aplikasi..."
        : "Mengunduh paket installer...";
    }
    switch (stage) {
      case "verify":
        return "Memverifikasi checksum SHA-256...";
      case "extract":
        return "Mengekstrak file baru ke folder portable...";
      case "restart":
        return "Memulai ulang ComiCut Portable...";
      default:
        return "Mengunduh paket portable (.zip)...";
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4 backdrop-blur-md animate-in fade-in duration-200"
      onClick={() => {
        if (!downloading) handleDismiss();
      }}
    >
      <div
        className="w-full max-w-md rounded-2xl border border-white/10 bg-[#121216] p-6 shadow-2xl space-y-5"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="grid h-11 w-11 shrink-0 place-items-center rounded-xl bg-gradient-to-br from-violet-600/20 to-fuchsia-600/20 border border-violet-500/30 text-violet-400">
              <Sparkles className="h-5 w-5" />
            </div>
            <div>
              <div className="flex items-center gap-2 flex-wrap">
                <h2 className="text-base font-bold text-white leading-tight">
                  Pembaruan Tersedia!
                </h2>
                <span className="rounded-full bg-violet-500/20 border border-violet-500/30 px-2 py-0.5 text-[10px] font-semibold text-violet-300">
                  v{updateInfo.version}
                </span>
                <span
                  className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-semibold border ${
                    isPortable
                      ? "bg-amber-500/15 border-amber-500/30 text-amber-300"
                      : "bg-sky-500/15 border-sky-500/30 text-sky-300"
                  }`}
                >
                  {isPortable ? (
                    <>
                      <FolderArchive className="w-3 h-3" />
                      <span>Portable (.zip)</span>
                    </>
                  ) : (
                    <>
                      <PackageCheck className="w-3 h-3" />
                      <span>Installer</span>
                    </>
                  )}
                </span>
              </div>
              <p className="mt-0.5 text-xs text-zinc-400">
                {isPortable
                  ? "Pembaruan khusus Portable akan menimpa file langsung di folder ini."
                  : "Versi baru ComiCut Studio siap diunduh dan dipasang."}
              </p>
            </div>
          </div>

          {!downloading && (
            <button
              type="button"
              onClick={handleDismiss}
              className="text-zinc-400 hover:text-white transition p-1"
              title="Tutup"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>

        {/* Version info badge & Release notes */}
        <div className="space-y-2">
          <div className="flex items-center justify-between rounded-xl bg-zinc-900/60 border border-white/[0.04] px-4 py-2 text-xs">
            <span className="text-zinc-500">Versi Saat Ini:</span>
            <span className="text-zinc-300 font-mono font-medium">
              v{updateInfo.currentVersion} ({isPortable ? "Portable" : "Installer"})
            </span>
          </div>

          {updateInfo.body && (
            <div className="rounded-xl bg-zinc-900/40 border border-white/[0.04] p-3 text-xs space-y-1 max-h-36 overflow-y-auto">
              <span className="block text-[10px] font-semibold tracking-wider text-zinc-500 uppercase">
                Catatan Rilis (Changelog):
              </span>
              <p className="text-zinc-300 leading-relaxed whitespace-pre-line text-[11px]">
                {updateInfo.body}
              </p>
            </div>
          )}
        </div>

        {/* Download & Install Progress */}
        {downloading && (
          <div className="space-y-2 rounded-xl bg-violet-950/20 border border-violet-500/20 p-4">
            <div className="flex items-center justify-between text-xs">
              <div className="flex items-center gap-2 text-violet-300 font-medium">
                {installing || stage !== "download" ? (
                  <Loader2 className="w-3.5 h-3.5 animate-spin text-fuchsia-400" />
                ) : (
                  <Download className="w-3.5 h-3.5 animate-pulse text-violet-400" />
                )}
                <span>{stageLabel()}</span>
              </div>
              <span className="font-mono font-semibold text-white">
                {progress}%
              </span>
            </div>

            {/* Progress Bar */}
            <div className="h-2 w-full overflow-hidden rounded-full bg-zinc-900">
              <div
                className="h-full bg-gradient-to-r from-violet-600 to-fuchsia-500 transition-all duration-300"
                style={{ width: `${progress}%` }}
              />
            </div>

            {totalBytes ? (
              <div className="text-[10px] text-zinc-400 text-right font-mono">
                {formatMb(downloadedBytes)} / {formatMb(totalBytes)}
              </div>
            ) : null}
          </div>
        )}

        {/* Error Message */}
        {error && (
          <div className="rounded-xl bg-rose-950/30 border border-rose-500/30 p-3 text-xs text-rose-300">
            {error}
          </div>
        )}

        {/* Action Buttons */}
        <div className="flex items-center justify-end gap-2 pt-2 border-t border-white/[0.06]">
          {!downloading && (
            <button
              type="button"
              onClick={handleDismiss}
              className="btn-ghost py-2 px-4 text-xs text-zinc-400 hover:text-white"
            >
              Nanti Saja
            </button>
          )}

          <button
            type="button"
            disabled={downloading}
            onClick={handleStartUpdate}
            className="btn-primary py-2 px-4 text-xs font-semibold inline-flex items-center gap-2 shadow-lg shadow-violet-950/50"
          >
            {downloading ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span>Memproses...</span>
              </>
            ) : (
              <>
                <ArrowUpCircle className="w-4 h-4" />
                <span>
                  {isPortable ? "Perbarui Portable" : "Perbarui Sekarang"}
                </span>
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
