import React from "react";
import { CircleHelp, Folder, Home, Settings } from "lucide-react";
import { ComiCutLogo } from "./ComiCutLogo";

export type NavTab = "home" | "projects" | "templates" | "help" | "settings";

interface SidebarNavProps {
  active: NavTab;
  onSelect: (tab: NavTab) => void;
  showLogo?: boolean;
}

/** Dashed frame with horizontal lines inside for the "Templates" icon. */
const TemplatesIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg
    viewBox="0 0 24 24"
    className={className}
    fill="none"
    stroke="currentColor"
    strokeWidth="1.8"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M8 3H6a3 3 0 0 0-3 3v2" />
    <path d="M16 3h2a3 3 0 0 1 3 3v2" />
    <path d="M8 21H6a3 3 0 0 1-3-3v-2" />
    <path d="M16 21h2a3 3 0 0 0 3-3v-2" />
    <line x1="8" y1="9" x2="16" y2="9" />
    <line x1="8" y1="12" x2="16" y2="12" />
    <line x1="8" y1="15" x2="13" y2="15" />
  </svg>
);

const TOP_ITEMS: { id: NavTab; label: string; Icon: React.FC<{ className?: string }> }[] = [
  { id: "home", label: "Home", Icon: Home },
  { id: "projects", label: "Projects", Icon: Folder },
  { id: "templates", label: "Templates", Icon: TemplatesIcon },
];

const BOTTOM_ITEMS: { id: NavTab; label: string; Icon: React.FC<{ className?: string }> }[] = [
  { id: "help", label: "Help", Icon: CircleHelp },
  { id: "settings", label: "Settings", Icon: Settings },
];

export const SidebarNav: React.FC<SidebarNavProps> = ({ active, onSelect, showLogo = false }) => {
  const renderItem = ({ id, label, Icon }: (typeof TOP_ITEMS)[number]) => {
    const isActive = active === id;
    return (
      <button
        key={id}
        type="button"
        onClick={() => onSelect(id)}
        className={`flex w-[76px] flex-col items-center justify-center gap-1.5 rounded-xl py-2.5 transition ${
          isActive
            ? "bg-zinc-800/90 text-white shadow-inner shadow-white/[0.04]"
            : "text-zinc-400 hover:bg-white/[0.04] hover:text-zinc-200"
        }`}
      >
        <Icon className={`h-5 w-5 ${isActive ? "text-white" : "text-zinc-400"}`} />
        <span className={`text-[11px] leading-none ${isActive ? "font-semibold text-white" : "font-normal"}`}>
          {label}
        </span>
      </button>
    );
  };

  return (
    <aside className="relative z-20 flex h-full w-[95px] shrink-0 flex-col items-center border-r border-white/[0.06] bg-[#0b0b0e] py-3">
      {/* Optional Brand Logo */}
      {showLogo && (
        <div className="mb-4 grid h-10 w-10 place-items-center rounded-xl bg-violet-950/40 border border-violet-500/25 shadow-lg shadow-violet-950/50">
          <ComiCutLogo className="h-6 w-6" />
        </div>
      )}

      {/* Main Navigation */}
      <nav className="flex flex-col items-center gap-2">
        {TOP_ITEMS.map(renderItem)}
      </nav>

      {/* Bottom Navigation + Profile */}
      <div className="mt-auto flex flex-col items-center gap-2">
        {BOTTOM_ITEMS.map(renderItem)}

        <button
          type="button"
          title="Profil"
          className="mt-2 grid h-10 w-10 place-items-center rounded-full bg-gradient-to-b from-zinc-500 to-zinc-700 text-sm font-bold text-zinc-950 shadow-md transition hover:brightness-110"
        >
          N
        </button>
      </div>
    </aside>
  );
};
