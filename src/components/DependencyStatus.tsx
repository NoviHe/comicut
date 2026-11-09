import React, { useCallback, useEffect, useRef, useState } from "react";
import { CloudDownload, Cpu, Download, Loader2, RefreshCw } from "lucide-react";
import {
  checkDependencies,
  checkDependencyUpdates,
  DepProgress,
  DepReport,
  DepStatus,
  DepUpdate,
  installDependency,
  onDepProgress,
} from "../services/tauriCommands";

const AUTO_KEY = "comicut.autoUpdateDeps";
const LAST_KEY = "comicut.depsLastCheck";

const badge = (d: DepStatus, u?: DepUpdate) =>
  !d.found
    ? d.required
      ? { text: "TIDAK ADA", cls: "bg-rose-500/15 text-rose-300 border-rose-500/30" }
      : { text: "OPSIONAL", cls: "bg-zinc-500/15 text-zinc-300 border-zinc-500/30" }
    : u?.updateAvailable
    ? { text: "UPDATE", cls: "bg-sky-500/15 text-sky-300 border-sky-500/30" }
    : d.outdated && !u?.latest
    ? { text: "USANG", cls: "bg-amber-500/15 text-amber-300 border-amber-500/30" }
    : { text: "READY", cls: "bg-emerald-500/15 text-emerald-300 border-emerald-500/30" };

const mb = (n: number) => (n / 1048576).toFixed(1);

const progressLabel = (p: DepProgress) => {
  if (p.stage === "verify") return "Memverifikasi SHA-256…";
  if (p.stage === "install") return "Memasang…";
  if (p.stage === "done") return "Selesai";
  return p.total
    ? `Mengunduh ${Math.floor((p.downloaded / p.total) * 100)}% (${mb(p.downloaded)}/${mb(p.total)} MB)`
    : `Mengunduh ${mb(p.downloaded)} MB`;
};

export const DependencyStatus: React.FC<{ onStatusMessage: (m: string) => void }> = ({
  onStatusMessage,
}) => {
  const [report, setReport] = useState<DepReport | null>(null);
  const [updates, setUpdates] = useState<Record<string, DepUpdate>>({});
  const [progress, setProgress] = useState<Record<string, DepProgress | undefined>>({});
  const [installing, setInstalling] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState("");
  const [autoUpdate, setAutoUpdate] = useState(() => localStorage.getItem(AUTO_KEY) === "1");
  const [lastCheck, setLastCheck] = useState(() => localStorage.getItem(LAST_KEY) ?? "");
  const started = useRef(false);

  useEffect(() => {
    const un = onDepProgress((p) => setProgress((prev) => ({ ...prev, [p.id]: p })));
    return () => {
      void un.then((f) => f());
    };
  }, []);

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

  const checkUpdates = useCallback(async (): Promise<DepUpdate[]> => {
    setChecking(true);
    try {
      const list = await checkDependencyUpdates();
      setUpdates(Object.fromEntries(list.map((u) => [u.id, u])));
      const now = new Date().toLocaleString();
      localStorage.setItem(LAST_KEY, now);
      setLastCheck(now);
      return list;
    } catch (e: any) {
      onStatusMessage(`Cek update gagal: ${e?.message || String(e)}`);
      return [];
    } finally {
      setChecking(false);
    }
  }, [onStatusMessage]);

  const install = useCallback(
    async (ids: string[]) => {
      for (const id of ids) {
        setInstalling(id);
        onStatusMessage(`Mengunduh ${id}...`);
        try {
          onStatusMessage(await installDependency(id));
        } catch (e: any) {
          onStatusMessage(`Gagal memasang ${id}: ${e?.message || String(e)}`);
        } finally {
          setProgress((prev) => ({ ...prev, [id]: undefined }));
        }
      }
      setInstalling(null);
      await refresh();
      await checkUpdates();
    },
    [onStatusMessage, refresh, checkUpdates],
  );

  // On open: local scan, then online update check (+ optional auto-update).
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    void (async () => {
      await refresh();
      const list = await checkUpdates();
      const pending = list.filter((u) => u.updateAvailable).map((u) => u.id);
      if (localStorage.getItem(AUTO_KEY) === "1" && pending.length) {
        onStatusMessage(`Auto-update dependensi: ${pending.join(", ")}`);
        await install(pending);
      }
    })();
  }, [refresh, checkUpdates, install, onStatusMessage]);

  const toggleAuto = (v: boolean) => {
    setAutoUpdate(v);
    localStorage.setItem(AUTO_KEY, v ? "1" : "0");
  };

  const pending = Object.values(updates).filter((u) => u.updateAvailable).map((u) => u.id);
  const ytdlp = report?.deps.find((d) => d.id === "yt-dlp");
  const jsRuntime = report?.deps.some((d) => (d.id === "node" || d.id === "deno") && d.found);
  const locked = busy || installing !== null;

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
        <div className="flex flex-wrap items-center gap-1.5">
          {pending.length > 0 && (
            <button
              type="button"
              disabled={locked}
              onClick={() => install(pending)}
              className="btn bg-sky-500 text-zinc-950 hover:bg-sky-400"
            >
              <Download className="w-3.5 h-3.5" />
              Update Semua ({pending.length})
            </button>
          )}
          <button type="button" disabled={locked || checking} onClick={() => checkUpdates()} className="btn-ghost">
            {checking ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <CloudDownload className="w-3.5 h-3.5" />}
            Cek Update
          </button>
          <button type="button" disabled={locked} onClick={refresh} className="btn-ghost">
            {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <RefreshCw className="w-3.5 h-3.5" />}
            Cek Ulang
          </button>
        </div>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] text-zinc-500">
        <label className="flex cursor-pointer items-center gap-1.5">
          <input type="checkbox" checked={autoUpdate} onChange={(e) => toggleAuto(e.target.checked)} />
          Update otomatis saat aplikasi dibuka
        </label>
        {lastCheck && <span>Cek update terakhir: {lastCheck}</span>}
      </div>

      {error && <p className="text-[11px] text-rose-300">{error}</p>}

      <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
        {report?.deps.map((d) => {
          const u = updates[d.id];
          const b = badge(d, u);
          const p = progress[d.id];
          const pct = p?.stage === "download" && p.total ? (p.downloaded / p.total) * 100 : p ? 100 : 0;
          return (
            <div key={d.id} className="rounded-xl border border-white/[0.06] bg-zinc-950/60 p-3">
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                  <div className="text-xs font-semibold text-zinc-100">
                    {d.name}
                    {d.managed && (
                      <span className="ml-1.5 text-[9px] font-normal text-sky-400/80" title={d.path}>
                        • diperbarui di aplikasi
                      </span>
                    )}
                    {d.bundled && (
                      <span className="ml-1.5 text-[9px] font-normal text-emerald-400/80" title={d.path}>
                        • bawaan aplikasi
                      </span>
                    )}
                  </div>
                  <div className="truncate font-mono text-[10px] text-zinc-500" title={d.path}>
                    {d.found ? d.version ?? "terpasang" : "tidak ditemukan"}
                    {u?.updateAvailable && <span className="text-sky-300"> → {u.latest}</span>}
                    {d.found && u?.latest && !u.updateAvailable && <span className="text-emerald-400/70"> (terbaru)</span>}
                  </div>
                </div>
                <span className={`shrink-0 rounded-md border px-1.5 py-0.5 text-[10px] font-bold ${b.cls}`}>
                  {b.text}
                </span>
              </div>
              <p className="mt-1.5 text-[10px] text-zinc-500">{d.purpose}</p>

              {p ? (
                <div className="mt-2 space-y-1">
                  <div className="h-1.5 overflow-hidden rounded bg-white/10">
                    <div
                      className={`h-full bg-sky-400 transition-all ${p.stage === "download" && !p.total ? "animate-pulse" : ""}`}
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                  <div className="text-[10px] text-sky-300">{progressLabel(p)}</div>
                </div>
              ) : d.installable && (!d.found || u?.updateAvailable) ? (
                <button
                  type="button"
                  disabled={locked}
                  onClick={() => install([d.id])}
                  className="btn mt-2 w-full justify-center bg-sky-500/90 py-1 text-[11px] text-zinc-950 hover:bg-sky-400"
                >
                  <Download className="w-3 h-3" />
                  {d.found ? `Update ke ${u?.latest}` : "Install"}
                </button>
              ) : (
                !d.installable &&
                !d.found && (
                  <>
                    <code className="mt-1.5 block select-text break-all rounded bg-black/40 px-1.5 py-1 text-[10px] text-amber-200/90">
                      {d.installHint}
                    </code>
                    {d.id === "node" && (
                      <p className="mt-1 text-[10px] text-zinc-500">Tidak perlu jika Deno sudah terpasang.</p>
                    )}
                  </>
                )
              )}
              {u?.error && d.installable && (
                <p className="mt-1 text-[10px] text-rose-300/80" title={u.error}>
                  Cek update gagal (offline?)
                </p>
              )}
            </div>
          );
        })}
      </div>

      {report && ytdlp?.found && !jsRuntime && (
        <p className="text-[11px] text-amber-300/90">
          yt-dlp terpasang tapi tidak ada JS runtime (Node.js / Deno). Unduhan YouTube bisa gagal.{" "}
          <button
            type="button"
            disabled={locked}
            onClick={() => install(["deno"])}
            className="underline hover:text-amber-200"
          >
            Install Deno
          </button>
        </p>
      )}
    </div>
  );
};
