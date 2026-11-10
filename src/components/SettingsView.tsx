import React, { useEffect, useRef, useState } from "react";
import {
  Check,
  ChevronDown,
  ChevronRight,
  Copy,
  Cpu,
  ExternalLink,
  Folder,
  FolderOpen,
  Languages,
  Loader2,
  RefreshCw,
  RotateCcw,
  Sparkles,
} from "lucide-react";
import {
  clearDefaultExportDir,
  DeviceDiagnostics,
  getDefaultExportDir,
  getDeviceDiagnostics,
  getSystemDefaultExportDir,
  setDefaultExportDir,
  showInFolder,
} from "../services/tauriCommands";
import { AppUpdateInfo, checkComiCutUpdate } from "../services/appUpdater";

interface SettingsCardProps {
  icon?: React.ReactNode;
  category: string;
  title: string;
  badge?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
}

export const SettingsCard: React.FC<SettingsCardProps> = ({
  icon,
  category,
  title,
  badge,
  children,
  className = "",
}) => {
  return (
    <div
      className={`rounded-2xl border border-white/[0.06] bg-[#111115]/90 shadow-lg backdrop-blur transition hover:border-white/10 relative ${className}`}
    >
      {/* Card Header */}
      <div className="flex items-center justify-between px-5 py-3.5 border-b border-white/[0.06] rounded-t-2xl">
        <div className="flex items-center gap-3 min-w-0">
          {icon && <div className="text-zinc-400 shrink-0">{icon}</div>}
          <div className="min-w-0">
            <span className="block text-[10px] font-semibold tracking-wider text-zinc-500 uppercase leading-none mb-1">
              {category}
            </span>
            <h2 className="text-sm font-bold text-white tracking-tight leading-tight">
              {title}
            </h2>
          </div>
        </div>
        {badge && <div className="shrink-0 ml-3">{badge}</div>}
      </div>

      {/* Card Body */}
      <div className="p-5 space-y-3">{children}</div>
    </div>
  );
};

const DEVICE_CACHE_KEY = "comicut:device_diagnostics";

interface SettingsViewProps {
  onCheckUpdate?: () => void;
  onUpdateFound?: (info: AppUpdateInfo) => void;
}

export const SettingsView: React.FC<SettingsViewProps> = ({
  onCheckUpdate,
  onUpdateFound,
}) => {
  const [selectedLang, setSelectedLang] = useState<string>("English");
  const [langDropdownOpen, setLangDropdownOpen] = useState<boolean>(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Updates & Versions state
  const [updatesExpanded, setUpdatesExpanded] = useState<boolean>(false);
  const [checkingUpdates, setCheckingUpdates] = useState<boolean>(false);
  const [updateStatus, setUpdateStatus] = useState<"uptodate" | "available" | "error">("uptodate");
  const [foundUpdate, setFoundUpdate] = useState<AppUpdateInfo | null>(null);

  const handleCheckUpdates = async () => {
    setCheckingUpdates(true);
    try {
      const info = await checkComiCutUpdate();
      if (info && info.available) {
        setUpdateStatus("available");
        setFoundUpdate(info);
        if (onUpdateFound) {
          onUpdateFound(info);
        }
      } else {
        setUpdateStatus("uptodate");
        setFoundUpdate(null);
      }
    } catch {
      setUpdateStatus("error");
    } finally {
      setCheckingUpdates(false);
    }
  };

  // Device diagnostics state - saved to localStorage so it only reads once unless refreshed
  const [deviceInfo, setDeviceInfo] = useState<DeviceDiagnostics | null>(() => {
    try {
      const cached = localStorage.getItem(DEVICE_CACHE_KEY);
      if (cached) {
        return JSON.parse(cached) as DeviceDiagnostics;
      }
    } catch (err) {
      console.warn("Failed to read cached device diagnostics:", err);
    }
    return null;
  });
  const [loadingDevice, setLoadingDevice] = useState<boolean>(false);
  const [deviceExpanded, setDeviceExpanded] = useState<boolean>(false);
  const [copied, setCopied] = useState<boolean>(false);

  // Default export path state
  const [exportExpanded, setExportExpanded] = useState<boolean>(false);
  const [exportDir, setExportDir] = useState<string | null>(() => getDefaultExportDir());
  const [systemDefaultDir, setSystemDefaultDir] = useState<string>("");

  useEffect(() => {
    void getSystemDefaultExportDir().then((dir) => {
      if (dir) setSystemDefaultDir(dir);
    });
  }, []);

  const handleSelectExportDir = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Pilih Folder Default Ekspor Video",
        defaultPath: exportDir || systemDefaultDir || undefined,
      });
      if (typeof selected === "string" && selected.trim()) {
        setExportDir(selected);
        setDefaultExportDir(selected);
      }
    } catch (err) {
      console.warn("Gagal membuka dialog folder:", err);
    }
  };

  const handleOpenExportDir = async () => {
    const target = exportDir || systemDefaultDir;
    if (!target) return;
    try {
      await showInFolder(target);
    } catch (err) {
      console.warn("Gagal membuka folder di explorer:", err);
    }
  };

  const handleResetExportDir = () => {
    clearDefaultExportDir();
    setExportDir(null);
  };

  const fetchDiagnostics = async (force: boolean = false) => {
    // If not forcing and cache exists, don't read hardware again
    if (!force) {
      try {
        const cached = localStorage.getItem(DEVICE_CACHE_KEY);
        if (cached) {
          setDeviceInfo(JSON.parse(cached) as DeviceDiagnostics);
          return;
        }
      } catch {
        // Fall through to query hardware
      }
    }

    setLoadingDevice(true);
    try {
      const data = await getDeviceDiagnostics();
      setDeviceInfo(data);
      try {
        localStorage.setItem(DEVICE_CACHE_KEY, JSON.stringify(data));
      } catch (e) {
        console.warn("Failed to cache device diagnostics:", e);
      }
    } catch (err) {
      console.warn("Failed to get diagnostics:", err);
    } finally {
      setLoadingDevice(false);
    }
  };

  useEffect(() => {
    // Only query on first launch if no saved device info exists
    if (!deviceInfo) {
      void fetchDiagnostics(false);
    }
  }, []);

  // Close dropdown menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setLangDropdownOpen(false);
      }
    };

    if (langDropdownOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [langDropdownOpen]);

  const handleCopyDiagnostics = () => {
    if (!deviceInfo) return;
    const text = [
      `Aplikasi: ${deviceInfo.appName} ${deviceInfo.appVersion} (${deviceInfo.architecture})`,
      `Sistem Operasi: ${deviceInfo.osName} (${deviceInfo.osBuild}) • kernel ${deviceInfo.kernel}`,
      `Prosesor: ${deviceInfo.cpuModel}`,
      `Arsitektur: ${deviceInfo.architecture}`,
      `Inti CPU: ${deviceInfo.cpuCoresPhysical} fisik / ${deviceInfo.cpuCoresLogical} logis`,
      `Memori: ${deviceInfo.totalMemoryGb} GB`,
      `GPU: ${deviceInfo.gpus.join(", ")}`,
    ].join("\n");

    void navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="relative z-10 flex flex-1 flex-col overflow-y-auto p-5 sm:p-6 space-y-4 w-full">
      {/* Page Title & Subtitle */}
      <div className="border-b border-white/[0.06] pb-3">
        <h1 className="text-xl font-bold tracking-tight text-white">
          Settings
        </h1>
        <p className="mt-0.5 text-xs text-zinc-400">
          YouTube acquisition settings and diagnostics.
        </p>
      </div>

      {/* Settings Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 w-full items-start">
        {/* Left Column: App Language Card */}
        <div>
          <SettingsCard
            icon={<Languages className="w-4 h-4 text-zinc-400" />}
            category="Language"
            title="App Language"
          >
            <p className="text-xs leading-relaxed text-zinc-400">
              Changes the language of the entire interface. Subtitle and AI output follow the language of your video, not this setting.
            </p>

            {/* Language Selector Dropdown */}
            <div ref={dropdownRef} className="relative inline-block pt-1 z-30">
              <button
                type="button"
                onClick={() => setLangDropdownOpen(!langDropdownOpen)}
                className="flex items-center gap-2 rounded-lg border border-white/10 bg-zinc-900/90 px-3 py-1.5 text-xs font-medium text-zinc-200 transition hover:bg-zinc-800 hover:border-white/20 active:scale-95 shadow-sm"
              >
                <Languages className="w-3.5 h-3.5 text-zinc-400" />
                <span className="text-zinc-500 uppercase text-[10px] font-bold">
                  Language
                </span>
                <span className="text-white font-medium">{selectedLang}</span>
                <ChevronDown className="w-3.5 h-3.5 text-zinc-400 ml-0.5" />
              </button>

              {langDropdownOpen && (
                <div className="absolute left-0 top-full mt-1.5 w-48 rounded-xl border border-white/10 bg-zinc-900/95 backdrop-blur-md p-1 shadow-2xl z-50">
                  {["English", "Bahasa Indonesia", "Japanese", "Korean"].map(
                    (lang) => (
                      <button
                        key={lang}
                        type="button"
                        onClick={() => {
                          setSelectedLang(lang);
                          setLangDropdownOpen(false);
                        }}
                        className={`w-full rounded-lg px-3 py-2 text-left text-xs transition ${
                          selectedLang === lang
                            ? "bg-violet-600/30 text-violet-200 font-semibold"
                            : "text-zinc-300 hover:bg-white/5"
                        }`}
                      >
                        {lang}
                      </button>
                    )
                  )}
                </div>
              )}
            </div>
          </SettingsCard>
        </div>

        {/* Right Column: Expandable System Rows & Device Diagnostics */}
        <div className="space-y-3">
          {/* Expandable Updates & Versions Card */}
          <div className="rounded-2xl border border-white/[0.06] bg-[#111115]/90 shadow-lg backdrop-blur overflow-hidden transition hover:border-white/15">
            {/* Header (Click to expand/collapse) */}
            <div
              onClick={() => setUpdatesExpanded(!updatesExpanded)}
              className="flex items-center justify-between px-5 py-3.5 cursor-pointer select-none"
            >
              <div className="flex items-center gap-3 min-w-0">
                {updatesExpanded ? (
                  <ChevronDown className="w-4 h-4 text-zinc-400 shrink-0 transition" />
                ) : (
                  <ChevronRight className="w-4 h-4 text-zinc-500 shrink-0 transition" />
                )}
                <div className="min-w-0">
                  <span className="block text-[10px] font-semibold tracking-wider text-zinc-500 uppercase leading-none mb-1">
                    System
                  </span>
                  <h3 className="text-sm font-semibold text-white truncate leading-tight">
                    Updates &amp; Versions
                  </h3>
                </div>
              </div>

              {checkingUpdates ? (
                <div className="flex items-center gap-1.5 rounded-full border border-zinc-700 bg-zinc-800/80 px-2.5 py-0.5 text-[10px] font-bold tracking-wider text-zinc-300 shrink-0">
                  <Loader2 className="w-3 h-3 animate-spin text-violet-400" />
                  <span>MEMERIKSA...</span>
                </div>
              ) : updateStatus === "available" ? (
                <div className="flex items-center gap-1.5 rounded-full border border-amber-500/40 bg-amber-500/10 px-2.5 py-0.5 text-[10px] font-bold tracking-wider text-amber-300 shrink-0">
                  <Sparkles className="w-3 h-3" />
                  <span>UPDATE TERSEDIA</span>
                </div>
              ) : (
                <div className="flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-0.5 text-[10px] font-bold tracking-wider text-emerald-400 shrink-0">
                  <Check className="w-3 h-3 stroke-[3]" />
                  <span>UP TO DATE</span>
                </div>
              )}
            </div>

            {/* Expanded Body */}
            {updatesExpanded && (
              <div className="border-t border-white/[0.06] p-4 space-y-3">
                <div className="space-y-1.5 text-xs">
                  <div className="flex items-center justify-between rounded-xl bg-zinc-900/40 border border-white/[0.03] px-4 py-2.5">
                    <span className="text-zinc-500">Versi Terpasang</span>
                    <span className="text-zinc-100 font-medium font-mono text-[11px]">
                      ComiCut Studio v0.1.0
                    </span>
                  </div>

                  <div className="flex items-center justify-between rounded-xl bg-zinc-900/40 border border-white/[0.03] px-4 py-2.5">
                    <span className="text-zinc-500">Kanal Pembaruan</span>
                    <span className="text-zinc-100 font-medium text-[11px]">
                      GitHub Releases (Official)
                    </span>
                  </div>

                  <div className="flex items-center justify-between rounded-xl bg-zinc-900/40 border border-white/[0.03] px-4 py-2.5">
                    <span className="text-zinc-500">Status Sistem</span>
                    <span className="text-zinc-100 font-medium text-[11px]">
                      {checkingUpdates
                        ? "Sedang memeriksa rilis terbaru..."
                        : updateStatus === "available"
                        ? `Tersedia versi v${foundUpdate?.version || ""}`
                        : "Versi Anda sudah paling baru"}
                    </span>
                  </div>
                </div>

                <div className="flex flex-wrap items-center justify-between gap-2 pt-2 border-t border-white/[0.04]">
                  <span className="text-[11px] text-zinc-500">
                    Pembaruan otomatis diperiksa saat aplikasi dibuka.
                  </span>

                  <div className="flex items-center gap-2">
                    <button
                      type="button"
                      disabled={checkingUpdates}
                      onClick={handleCheckUpdates}
                      className="btn-ghost py-1 px-3 text-xs inline-flex items-center gap-1.5"
                      title="Periksa apakah ada versi baru di GitHub"
                    >
                      <RefreshCw
                        className={`w-3.5 h-3.5 ${checkingUpdates ? "animate-spin text-violet-400" : ""}`}
                      />
                      <span>{checkingUpdates ? "Memeriksa..." : "Periksa Pembaruan"}</span>
                    </button>

                    {updateStatus === "available" && foundUpdate && (
                      <button
                        type="button"
                        onClick={() => onUpdateFound && onUpdateFound(foundUpdate)}
                        className="btn-primary py-1 px-3 text-xs inline-flex items-center gap-1.5 shadow-md shadow-violet-950/40"
                      >
                        <Sparkles className="w-3.5 h-3.5" />
                        <span>Pasang Update</span>
                      </button>
                    )}
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Expandable Default Export Path Card */}
          <div className="rounded-2xl border border-white/[0.06] bg-[#111115]/90 shadow-lg backdrop-blur overflow-hidden transition hover:border-white/15">
            {/* Header (Click to expand/collapse) */}
            <div
              onClick={() => setExportExpanded(!exportExpanded)}
              className="flex items-center justify-between px-5 py-3.5 cursor-pointer select-none"
            >
              <div className="flex items-center gap-3 min-w-0">
                {exportExpanded ? (
                  <ChevronDown className="w-4 h-4 text-zinc-400 shrink-0 transition" />
                ) : (
                  <ChevronRight className="w-4 h-4 text-zinc-500 shrink-0 transition" />
                )}
                <div className="min-w-0">
                  <span className="block text-[10px] font-semibold tracking-wider text-zinc-500 uppercase leading-none mb-1">
                    Render &amp; Export
                  </span>
                  <h3 className="text-sm font-semibold text-white truncate leading-tight">
                    Default Export Path
                  </h3>
                </div>
              </div>

              <div className="flex items-center gap-1.5 rounded-lg border border-white/10 bg-zinc-900/90 px-2 py-0.5 text-[10px] font-semibold text-zinc-300 shrink-0">
                <Folder className="w-3 h-3 text-zinc-400" />
                <span className="text-zinc-500 font-bold text-[9px]">STATUS</span>
                <span className="text-white">{exportDir ? "custom" : "set"}</span>
              </div>
            </div>

            {/* Expanded Body: Folder details & selection */}
            {exportExpanded && (
              <div className="border-t border-white/[0.06] p-4 space-y-3">
                <p className="text-xs leading-relaxed text-zinc-400">
                  Pilih folder default di komputer Anda untuk menyimpan hasil video render (Reels &amp; Shorts).
                  Nama file otomatis menggunakan format <code className="text-violet-300 font-mono text-[11px]">comicut_render_YYYY-MM-DD_HH-mm-ss.mp4</code> dengan tanggal dan timestamp agar tidak saling menimpa.
                </p>

                {/* Path display box */}
                <div className="flex items-center justify-between rounded-xl bg-zinc-900/50 border border-white/[0.06] px-4 py-2.5 gap-3">
                  <div className="flex items-center gap-2.5 min-w-0 flex-1">
                    <Folder className="w-4 h-4 text-violet-400 shrink-0" />
                    <span
                      className="text-xs font-mono text-zinc-200 truncate select-all"
                      title={exportDir || systemDefaultDir || "Folder Video Default"}
                    >
                      {exportDir || systemDefaultDir || "Memuat lokasi..."}
                    </span>
                  </div>
                  {exportDir && (
                    <span className="rounded bg-violet-500/20 px-1.5 py-0.5 text-[10px] font-semibold text-violet-300 shrink-0">
                      Khusus
                    </span>
                  )}
                </div>

                {/* Footer Buttons */}
                <div className="flex flex-wrap items-center justify-between gap-2 pt-2 border-t border-white/[0.04]">
                  <div className="flex items-center gap-1.5">
                    {exportDir && (
                      <button
                        type="button"
                        onClick={handleResetExportDir}
                        className="btn-ghost py-1 px-2.5 text-xs inline-flex items-center gap-1.5 text-zinc-400 hover:text-zinc-200"
                        title="Kembalikan ke folder default Videos"
                      >
                        <RotateCcw className="w-3.5 h-3.5" />
                        <span>Reset Default</span>
                      </button>
                    )}
                  </div>

                  <div className="flex items-center gap-1.5">
                    <button
                      type="button"
                      onClick={() => void handleOpenExportDir()}
                      className="btn-ghost py-1 px-2.5 text-xs inline-flex items-center gap-1.5"
                      title="Buka folder ini di File Explorer"
                    >
                      <ExternalLink className="w-3.5 h-3.5" />
                      <span>Buka Folder</span>
                    </button>

                    <button
                      type="button"
                      onClick={() => void handleSelectExportDir()}
                      className="btn-primary py-1 px-3 text-xs inline-flex items-center gap-1.5"
                      title="Pilih folder baru"
                    >
                      <FolderOpen className="w-3.5 h-3.5" />
                      <span>Pilih Folder...</span>
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Expandable Device Diagnostics Card */}
          <div className="rounded-2xl border border-white/[0.06] bg-[#111115]/90 shadow-lg backdrop-blur overflow-hidden transition hover:border-white/15">
            {/* Header (Click to expand/collapse) */}
            <div
              onClick={() => setDeviceExpanded(!deviceExpanded)}
              className="flex items-center justify-between px-5 py-3.5 cursor-pointer select-none"
            >
              <div className="flex items-center gap-3 min-w-0">
                {deviceExpanded ? (
                  <ChevronDown className="w-4 h-4 text-zinc-400 shrink-0 transition" />
                ) : (
                  <ChevronRight className="w-4 h-4 text-zinc-500 shrink-0 transition" />
                )}
                <div className="min-w-0">
                  <span className="block text-[10px] font-semibold tracking-wider text-zinc-500 uppercase leading-none mb-1">
                    Diagnostics
                  </span>
                  <h3 className="text-sm font-semibold text-white truncate leading-tight">
                    Informasi Perangkat
                  </h3>
                </div>
              </div>

              <div className="flex items-center gap-1.5 rounded-lg border border-white/10 bg-zinc-900/90 px-2 py-0.5 text-[10px] font-mono text-zinc-300 shrink-0">
                <Cpu className="w-3 h-3 text-zinc-400" />
                <span>
                  {deviceInfo
                    ? `${deviceInfo.architecture} ${deviceInfo.osName} (${deviceInfo.osBuild})`
                    : "Memuat..."}
                </span>
              </div>
            </div>

            {/* Expanded Body: Real Device Details */}
            {deviceExpanded && (
              <div className="border-t border-white/[0.06] p-4 space-y-2">
                {loadingDevice && !deviceInfo ? (
                  <div className="flex items-center justify-center p-6 text-zinc-400 gap-2 text-xs">
                    <Loader2 className="w-4 h-4 animate-spin text-violet-400" />
                    <span>Membaca spesifikasi perangkat...</span>
                  </div>
                ) : (
                  <>
                    {/* Rows */}
                    <div className="space-y-1.5 text-xs">
                      {/* Aplikasi */}
                      <div className="flex items-center justify-between rounded-xl bg-zinc-900/40 border border-white/[0.03] px-4 py-2.5">
                        <span className="text-zinc-500">Aplikasi</span>
                        <span className="text-zinc-100 font-medium font-mono text-[11px]">
                          {deviceInfo?.appName || "ComiCut Studio"}{" "}
                          {deviceInfo?.appVersion || "0.1.0"} (
                          {deviceInfo?.architecture || "x86_64"})
                        </span>
                      </div>

                      {/* Sistem Operasi */}
                      <div className="flex items-center justify-between rounded-xl bg-zinc-900/40 border border-white/[0.03] px-4 py-2.5">
                        <span className="text-zinc-500">Sistem Operasi</span>
                        <span className="text-zinc-100 font-medium">
                          {deviceInfo?.osName || "Windows 11"} (
                          {deviceInfo?.osBuild || "26200"}) • kernel{" "}
                          {deviceInfo?.kernel || "26200"}
                        </span>
                      </div>

                      {/* Prosesor */}
                      <div className="flex items-center justify-between rounded-xl bg-zinc-900/40 border border-white/[0.03] px-4 py-2.5">
                        <span className="text-zinc-500">Prosesor</span>
                        <span className="text-zinc-100 font-medium">
                          {deviceInfo?.cpuModel || "CPU"}
                        </span>
                      </div>

                      {/* Arsitektur */}
                      <div className="flex items-center justify-between rounded-xl bg-zinc-900/40 border border-white/[0.03] px-4 py-2.5">
                        <span className="text-zinc-500">Arsitektur</span>
                        <span className="text-zinc-100 font-medium font-mono text-[11px]">
                          {deviceInfo?.architecture || "x86_64"}
                        </span>
                      </div>

                      {/* Inti CPU */}
                      <div className="flex items-center justify-between rounded-xl bg-zinc-900/40 border border-white/[0.03] px-4 py-2.5">
                        <span className="text-zinc-500">Inti CPU</span>
                        <span className="text-zinc-100 font-medium">
                          {deviceInfo?.cpuCoresPhysical || 6} fisik /{" "}
                          {deviceInfo?.cpuCoresLogical || 12} logis
                        </span>
                      </div>

                      {/* Memori */}
                      <div className="flex items-center justify-between rounded-xl bg-zinc-900/40 border border-white/[0.03] px-4 py-2.5">
                        <span className="text-zinc-500">Memori</span>
                        <span className="text-zinc-100 font-medium font-mono text-[11px]">
                          {deviceInfo?.totalMemoryGb || 16} GB
                        </span>
                      </div>

                      {/* GPU */}
                      <div className="flex items-center justify-between rounded-xl bg-zinc-900/40 border border-white/[0.03] px-4 py-2.5">
                        <span className="text-zinc-500 shrink-0 mr-4">GPU</span>
                        <span className="text-zinc-100 font-medium text-right truncate">
                          {deviceInfo?.gpus && deviceInfo.gpus.length > 0
                            ? deviceInfo.gpus.join(", ")
                            : "Tidak terdeteksi"}
                        </span>
                      </div>
                    </div>

                    {/* Footer Info & Action Buttons */}
                    <div className="flex flex-wrap items-center justify-between gap-2 pt-3 border-t border-white/[0.04]">
                      <span className="text-[11px] text-zinc-500">
                        Sertakan informasi ini saat melaporkan masalah render.
                      </span>

                      <div className="flex items-center gap-1.5">
                        <button
                          type="button"
                          disabled={loadingDevice}
                          onClick={() => void fetchDiagnostics(true)}
                          className="btn-ghost py-1 px-2.5 text-xs inline-flex items-center gap-1.5"
                          title="Perbarui informasi perangkat"
                        >
                          <RefreshCw
                            className={`w-3.5 h-3.5 ${
                              loadingDevice ? "animate-spin text-violet-400" : ""
                            }`}
                          />
                          <span>Muat ulang</span>
                        </button>

                        <button
                          type="button"
                          onClick={handleCopyDiagnostics}
                          className="btn-ghost py-1 px-2.5 text-xs inline-flex items-center gap-1.5"
                          title="Salin rincian informasi perangkat ke clipboard"
                        >
                          {copied ? (
                            <>
                              <Check className="w-3.5 h-3.5 text-emerald-400" />
                              <span className="text-emerald-400">Tersalin!</span>
                            </>
                          ) : (
                            <>
                              <Copy className="w-3.5 h-3.5" />
                              <span>Salin</span>
                            </>
                          )}
                        </button>
                      </div>
                    </div>
                  </>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
