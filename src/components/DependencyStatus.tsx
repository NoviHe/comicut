import React, { useCallback, useEffect, useState } from "react";
import { Cpu, Loader2, RefreshCw } from "lucide-react";
import {
  checkDependencies,
  DepReport,
  DepStatus,
  updateYtdlp,
} from "../services/tauriCommands";

const badge = (d: DepStatus) =>
  !d.found
    ? d.required
      ? { text: "TIDAK ADA", cls: "bg-rose-500/15 text-rose-300 border-rose-500/30" }
      : { text: "OPSIONAL", cls: "bg-zinc-500/15 text-zinc-300 border-zinc-500/30" }
    : d.outdated
    ? { text: "USANG", cls: "bg-amber-500/15 text-amber-300 border-amber-500/30" }
    : { text: "READY", cls: "bg-emerald-500/15 text-emerald-300 border-emerald-500/30" };

export const DependencyStatus: React.FC<{ onStatusMessage: (m: string) => void }> = ({
  onStatusMessage,
}) => {
  const [report, setReport] = useState<DepReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    setBusy(true);
    setError("");
    try {
      setReport(await checkDependencies());
    } catch (e: any) {
      setError(e?.message || String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const ytdlp = report?.deps.find((d) => d.id === "yt-dlp");
  const jsRuntime = report?.deps.some((d) => (d.id === "node" || d.id === "deno") && d.found);

  const doUpdate = async () => {
    setBusy(true);
    onStatusMessage("Memperbarui yt-dlp...");
    try {
      onStatusMessage(await updateYtdlp());
    } catch (e: any) {
      onStatusMessage(e?.message || String(e));
    }
    await refresh();
  };

  return (
    <div className="card space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="card-title">
            <Cpu className="w-4 h-4 text-sky-400" />
            Status Dependensi
          </h2>
          {report && (
            <p className="mt-1 text-[11px] text-zinc-500">
              CPU: {report.system.cpu} • {report.system.threads} thread
              {report.system.gpus.length > 0 && <> • GPU: {report.system.gpus.join(", ")}</>}
              {" "}• Encoder render:{" "}
              <b className={report.system.gpuEncoder ? "text-emerald-300" : "text-zinc-400"}>
                {report.system.gpuEncoder ?? "CPU x264 (GPU tidak bisa dipakai)"}
              </b>
            </p>
          )}
        </div>
        <div className="flex gap-1.5">
          {ytdlp?.found && ytdlp.outdated && (
            <button type="button" disabled={busy} onClick={doUpdate} className="btn bg-amber-500 text-zinc-950 hover:bg-amber-400">
              Update yt-dlp
            </button>
          )}
          <button type="button" disabled={busy} onClick={refresh} className="btn-ghost">
            {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <RefreshCw className="w-3.5 h-3.5" />}
            Cek Ulang
          </button>
        </div>
      </div>

      {error && <p className="text-[11px] text-rose-300">{error}</p>}

      <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
        {report?.deps.map((d) => {
          const b = badge(d);
          return (
            <div key={d.id} className="rounded-xl border border-white/[0.06] bg-zinc-950/60 p-3">
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                  <div className="text-xs font-semibold text-zinc-100">{d.name}</div>
                  <div className="truncate font-mono text-[10px] text-zinc-500" title={d.path}>
                    {d.found ? d.version ?? "terpasang" : "tidak ditemukan"}
                  </div>
                </div>
                <span className={`shrink-0 rounded-md border px-1.5 py-0.5 text-[10px] font-bold ${b.cls}`}>
                  {b.text}
                </span>
              </div>
              <p className="mt-1.5 text-[10px] text-zinc-500">{d.purpose}</p>
              {(!d.found || d.outdated) && (
                <code className="mt-1.5 block select-text break-all rounded bg-black/40 px-1.5 py-1 text-[10px] text-amber-200/90">
                  {d.installHint}
                </code>
              )}
            </div>
          );
        })}
      </div>

      {report && ytdlp?.found && !jsRuntime && (
        <p className="text-[11px] text-amber-300/90">
          yt-dlp terpasang tapi tidak ada JS runtime (Node.js / Deno). Unduhan YouTube bisa gagal.
        </p>
      )}
    </div>
  );
};
