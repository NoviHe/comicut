import React from "react";
import { ChevronLeft, Minus, Plus, Settings, Square, X } from "lucide-react";
import { ComiCutLogo } from "./ComiCutLogo";
import {
  closeWindow,
  minimizeWindow,
  toggleMaximizeWindow,
} from "../services/tauriCommands";

interface TopBarProps {
  canGoBack?: boolean;
  onBack?: () => void;
  onOpenSettings: () => void;
  hasActiveSession?: boolean;
  onNewSession?: () => void;
}

export const TopBar: React.FC<TopBarProps> = ({
  canGoBack = false,
  onBack,
  onOpenSettings,
  hasActiveSession = false,
  onNewSession,
}) => {
  return (
    <header
      data-tauri-drag-region
      onDoubleClick={() => void toggleMaximizeWindow()}
      className="relative z-30 flex h-14 shrink-0 select-none items-center border-b border-white/[0.06] bg-[#0b0b0e]"
    >
      {/* Left logo block (aligned to sidebar width w-[95px]) */}
      <div
        data-tauri-drag-region
        className="flex h-full w-[95px] shrink-0 items-center justify-center border-r border-white/[0.06]"
      >
        <div className="grid h-9 w-9 place-items-center rounded-xl bg-violet-950/40 border border-violet-500/25 shadow-lg shadow-violet-950/50">
          <ComiCutLogo className="h-5 w-5" />
        </div>
      </div>

      {/* Main title bar content */}
      <div
        data-tauri-drag-region
        className="flex flex-1 items-center justify-between px-5"
      >
        {/* Left side: Optional Back button + Brand title and tagline */}
        <div data-tauri-drag-region className="flex items-center gap-2.5">
          {canGoBack && onBack && (
            <button
              type="button"
              onClick={onBack}
              className="grid h-7 w-7 place-items-center rounded-lg border border-white/10 bg-white/5 text-zinc-300 transition hover:bg-white/10 hover:text-white active:scale-95"
              title="Kembali ke Home"
              aria-label="Kembali ke Home"
            >
              <ChevronLeft className="h-4 w-4" />
            </button>
          )}

          <div
            data-tauri-drag-region
            className="pointer-events-none flex flex-col justify-center"
          >
            <h1 className="text-sm font-bold tracking-tight text-white leading-tight">
              ComiCut Studio
            </h1>
            <p className="text-[11px] text-zinc-500 leading-tight">
              Webtoon / Manga → Montagem Reels &amp; Shorts
            </p>
          </div>
        </div>

        {/* Right side: New Session button (when active work exists) + Pengaturan + Seamless window controls */}
        <div className="flex items-center gap-2.5">
          {hasActiveSession && onNewSession && (
            <button
              type="button"
              onClick={onNewSession}
              className="flex items-center gap-1.5 rounded-lg border border-violet-500/30 bg-violet-500/10 px-2.5 py-1 text-xs font-semibold text-violet-300 transition hover:bg-violet-500/20 hover:border-violet-500/50 hover:text-white active:scale-95 shadow-sm"
              title="Reset dan mulai sesi baru"
            >
              <Plus className="h-3.5 w-3.5 text-violet-400 stroke-[2.5]" />
              <span>New Session</span>
            </button>
          )}

          <button
            type="button"
            onClick={onOpenSettings}
            className="btn-ghost"
            title="Pengaturan AI Vision"
          >
            <Settings className="h-3.5 w-3.5" />
            <span>Pengaturan</span>
          </button>

          {/* Integrated Frameless Window Controls */}
          <div className="flex items-center gap-1 border-l border-white/10 pl-2">
            {/* Minimize */}
            <button
              type="button"
              onClick={() => void minimizeWindow()}
              className="grid h-7 w-8 place-items-center rounded text-zinc-400 transition hover:bg-white/10 hover:text-white"
              title="Minimize"
              aria-label="Minimize"
            >
              <Minus className="h-3.5 w-3.5" />
            </button>

            {/* Maximize / Restore */}
            <button
              type="button"
              onClick={() => void toggleMaximizeWindow()}
              className="grid h-7 w-8 place-items-center rounded text-zinc-400 transition hover:bg-white/10 hover:text-white"
              title="Maximize"
              aria-label="Maximize"
            >
              <Square className="h-3 w-3" />
            </button>

            {/* Close */}
            <button
              type="button"
              onClick={() => void closeWindow()}
              className="grid h-7 w-8 place-items-center rounded text-zinc-400 transition hover:bg-rose-600 hover:text-white"
              title="Close"
              aria-label="Close"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      </div>
    </header>
  );
};
