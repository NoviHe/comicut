import React, { useState, useEffect } from "react";
import {
  Globe,
  FolderOpen,
  Sparkles,
  SlidersHorizontal,
  CheckCircle2,
  Loader2,
  ExternalLink,
  Layers,
} from "lucide-react";
import {
  ComicFormat,
  Panel,
  PlatformKind,
  PlatformPreset,
} from "../types/project";
import {
  DEFAULT_PLATFORM_PRESETS,
  getPlatformPresets,
  importLocalChapter,
  scrapeComicChapter,
} from "../services/tauriCommands";

interface ChapterImporterProps {
  onPanelsLoaded: (panels: Panel[], chapterTitle: string) => void;
  onStatusMessage: (msg: string) => void;
}

export const ChapterImporter: React.FC<ChapterImporterProps> = ({
  onPanelsLoaded,
  onStatusMessage,
}) => {
  const [mode, setMode] = useState<"url" | "local">("url");
  const [presets, setPresets] = useState<PlatformPreset[]>(
    DEFAULT_PLATFORM_PRESETS
  );
  const [selectedPlatform, setSelectedPlatform] =
    useState<PlatformKind>("VoraToon");
  const [url, setUrl] = useState<string>("https://voratoon.web.id/");
  const [comicFormat, setComicFormat] =
    useState<ComicFormat>("WebtoonVertical");
  const [customSelector, setCustomSelector] = useState<string>(
    '#readerarea img, .reading-content img, .chapter-area img, img[class*="chapterPage"]'
  );
  const [customReferer, setCustomReferer] = useState<string>(
    "https://voratoon.web.id/"
  );
  const [showAdvanced, setShowAdvanced] = useState<boolean>(false);
  const [localFolderPath, setLocalFolderPath] = useState<string>("");
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [savedCustomDomains, setSavedCustomDomains] = useState<string[]>([
    "https://voratoon.web.id/",
    "https://www.webtoons.com/",
    "https://mangadex.org/",
  ]);

  useEffect(() => {
    getPlatformPresets().then((list) => {
      if (list && list.length > 0) setPresets(list);
    });
  }, []);

  const handleSelectPlatform = (platformId: PlatformKind) => {
    setSelectedPlatform(platformId);
    const preset = presets.find((p) => p.id === platformId);
    if (preset) {
      setComicFormat(preset.defaultFormat);
      setCustomSelector(preset.imageSelector);
      setCustomReferer(preset.refererHeader);
      if (platformId === "VoraToon" && !url.includes("voratoon")) {
        setUrl("https://voratoon.web.id/");
      } else if (platformId === "Webtoon" && !url.includes("webtoons.com")) {
        setUrl(preset.exampleUrl);
      } else if (platformId === "MangaDex" && !url.includes("mangadex.org")) {
        setUrl(preset.exampleUrl);
      }
    }
  };

  const handleUrlChange = (newUrl: string) => {
    setUrl(newUrl);
    const lower = newUrl.toLowerCase();
    if (lower.includes("webtoons.com")) {
      setSelectedPlatform("Webtoon");
      setComicFormat("WebtoonVertical");
      setCustomReferer("https://www.webtoons.com/");
    } else if (lower.includes("mangadex.org")) {
      setSelectedPlatform("MangaDex");
      setComicFormat("MangaRTL");
      setCustomReferer("https://mangadex.org/");
    } else if (lower.includes("voratoon.web.id")) {
      setSelectedPlatform("VoraToon");
      setComicFormat("WebtoonVertical");
      setCustomReferer("https://voratoon.web.id/");
    } else if (newUrl.startsWith("http")) {
      setSelectedPlatform("Custom");
      try {
        const parsed = new URL(newUrl);
        setCustomReferer(parsed.origin + "/");
      } catch {
        // ignore invalid partial URL
      }
    }
  };

  const handleScrapeUrl = async () => {
    if (!url.trim()) return;
    setIsLoading(true);
    onStatusMessage(`Mengambil & memotong panel dari ${url}...`);
    try {
      const result = await scrapeComicChapter({
        url: url.trim(),
        platform: selectedPlatform,
        comicFormat,
        customSelector: customSelector.trim() || undefined,
        customReferer: customReferer.trim() || undefined,
      });

      try {
        const origin = new URL(url.trim()).origin + "/";
        if (!savedCustomDomains.includes(origin)) {
          setSavedCustomDomains((prev) => [origin, ...prev]);
        }
      } catch {
        // ignore
      }

      onPanelsLoaded(result.extractedPanels, result.chapterTitle);
      onStatusMessage(
        `Berhasil mengekstrak ${result.extractedPanels.length} panel dari ${result.sourcePlatform}!`
      );
    } catch (err: any) {
      onStatusMessage(`Gagal scrape URL: ${err?.message || String(err)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleImportLocal = async () => {
    if (!localFolderPath.trim()) return;
    setIsLoading(true);
    onStatusMessage(`Memproses gambar komik dari folder ${localFolderPath}...`);
    try {
      const panels = await importLocalChapter(
        localFolderPath.trim(),
        comicFormat
      );
      onPanelsLoaded(panels, localFolderPath.split(/[\\/]/).pop() || "Chapter");
      onStatusMessage(`Berhasil memotong ${panels.length} panel dari folder!`);
    } catch (err: any) {
      onStatusMessage(`Gagal import lokal: ${err?.message || String(err)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleLoadSimulationPanels = () => {
    const samplePanels: Panel[] = [
      {
        id: "panel-1",
        index: 0,
        imagePath: "sample://voratoon-ch1-p1.png",
        bbox: { x: 0, y: 0, width: 800, height: 1120 },
        aspectRatio: 0.71,
        edgeDensity: 0.42,
        contrastScore: 0.68,
        heuristicScore: 0.74,
        aiEpicScore: 0.82,
        role: "Hook",
        isVerticalPan: false,
        selected: true,
        caption: "Opening Hook — Karakter utama menatap musuh di kegelapan",
      },
      {
        id: "panel-2",
        index: 1,
        imagePath: "sample://voratoon-ch1-p2.png",
        bbox: { x: 0, y: 1150, width: 800, height: 960 },
        aspectRatio: 0.83,
        edgeDensity: 0.51,
        contrastScore: 0.72,
        heuristicScore: 0.78,
        aiEpicScore: 0.85,
        role: "BuildUp",
        isVerticalPan: false,
        selected: true,
        caption: "Build-up — Efek aura mulai menyelimuti pedang",
      },
      {
        id: "panel-3",
        index: 2,
        imagePath: "sample://voratoon-ch1-p3.png",
        bbox: { x: 0, y: 2140, width: 800, height: 1050 },
        aspectRatio: 0.76,
        edgeDensity: 0.64,
        contrastScore: 0.85,
        heuristicScore: 0.88,
        aiEpicScore: 0.93,
        role: "AuraCloseup",
        isVerticalPan: false,
        selected: true,
        caption: "Pre-Drop Closeup — Mata menyala tepat sebelum Bass Drop",
      },
      {
        id: "panel-4",
        index: 3,
        imagePath: "sample://voratoon-ch1-p4.png",
        bbox: { x: 0, y: 3220, width: 800, height: 2400 },
        aspectRatio: 0.33,
        edgeDensity: 0.88,
        contrastScore: 0.94,
        heuristicScore: 0.97,
        aiEpicScore: 0.99,
        role: "Climax",
        isVerticalPan: true,
        selected: true,
        caption:
          "MAIN DROP CLIMAX — Long Vertical Action Panel (Auto Camera Scroll + Shake)",
      },
      {
        id: "panel-5",
        index: 4,
        imagePath: "sample://voratoon-ch1-p5.png",
        bbox: { x: 0, y: 5650, width: 800, height: 1280 },
        aspectRatio: 0.62,
        edgeDensity: 0.79,
        contrastScore: 0.91,
        heuristicScore: 0.92,
        aiEpicScore: 0.95,
        role: "Climax",
        isVerticalPan: false,
        selected: true,
        caption: "Beat Hit #2 — Tebasan penuh dengan garis kecepatan tinggi",
      },
      {
        id: "panel-6",
        index: 5,
        imagePath: "sample://voratoon-ch1-p6.png",
        bbox: { x: 0, y: 6960, width: 800, height: 1180 },
        aspectRatio: 0.67,
        edgeDensity: 0.73,
        contrastScore: 0.89,
        heuristicScore: 0.89,
        aiEpicScore: 0.91,
        role: "Climax",
        isVerticalPan: false,
        selected: true,
        caption: "Beat Hit #3 — Ledakan impact & siluet kemenangan",
      },
    ];
    onPanelsLoaded(samplePanels, "VoraToon — Sample Manhwa Chapter 01");
    onStatusMessage(
      "Berhasil memuat 6 panel simulasi (Webtoon Vertical Slicer + Epic Roles)."
    );
  };

  return (
    <div className="card space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="card-title">
          <span className="step">1</span>
          <Layers className="w-4 h-4 text-violet-400" />
          Sumber Komik
        </h2>
        <div className="flex bg-zinc-950 p-0.5 rounded-lg border border-zinc-800 text-xs">
          <button
            onClick={() => setMode("url")}
            className={`px-2.5 py-1 rounded-md flex items-center gap-1.5 transition ${
              mode === "url"
                ? "bg-violet-600 text-white font-medium"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <Globe className="w-3.5 h-3.5" />
            URL Web
          </button>
          <button
            onClick={() => setMode("local")}
            className={`px-2.5 py-1 rounded-md flex items-center gap-1.5 transition ${
              mode === "local"
                ? "bg-violet-600 text-white font-medium"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <FolderOpen className="w-3.5 h-3.5" />
            Folder Lokal
          </button>
        </div>
      </div>

      {/* Pilihan Platform Preset */}
      {mode === "url" && (
        <div className="space-y-3">
          <div>
            <label className="block text-[11px] uppercase tracking-wider text-zinc-400 mb-1.5">
              Pilih Platform Default atau Input Web Lain
            </label>
            <div className="grid grid-cols-2 gap-1.5">
              {presets.map((preset) => {
                const active = selectedPlatform === preset.id;
                return (
                  <button
                    key={preset.id}
                    type="button"
                    onClick={() => handleSelectPlatform(preset.id)}
                    className={`text-left p-2 rounded-lg border transition flex flex-col justify-between ${
                      active
                        ? "bg-violet-950/50 border-violet-500 text-zinc-100"
                        : "bg-zinc-950/60 border-zinc-800/80 text-zinc-400 hover:border-zinc-700 hover:text-zinc-200"
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-semibold">
                        {preset.name}
                      </span>
                      {active && (
                        <CheckCircle2 className="w-3.5 h-3.5 text-violet-400 shrink-0" />
                      )}
                    </div>
                    <span className="text-[10px] text-zinc-500 truncate mt-0.5">
                      {preset.baseUrl}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Input URL Chapter */}
          <div>
            <label className="block text-xs text-zinc-300 mb-1">
              URL Chapter Komik (Webtoon / MangaDex / VoraToon / Web Lain)
            </label>
            <div className="flex gap-2">
              <input
                type="url"
                value={url}
                onChange={(e) => handleUrlChange(e.target.value)}
                placeholder="Contoh: https://voratoon.web.id/judul-chapter-1/"
                className="field flex-1"
              />
            </div>
            {/* Quick domain chips */}
            <div className="flex flex-wrap items-center gap-1.5 mt-1.5">
              <span className="text-[10px] text-zinc-500">Contoh Cepat:</span>
              {savedCustomDomains.map((domain) => (
                <button
                  key={domain}
                  type="button"
                  onClick={() => handleUrlChange(domain)}
                  className="text-[10px] bg-zinc-950 hover:bg-zinc-800 text-zinc-400 hover:text-violet-300 border border-zinc-800 px-2 py-0.5 rounded-full flex items-center gap-1 transition"
                >
                  <ExternalLink className="w-2.5 h-2.5" />
                  {domain.replace(/^https?:\/\//, "").replace(/\/$/, "")}
                </button>
              ))}
            </div>
          </div>

          {/* Opsi Advanced Custom Scraper */}
          <div>
            <button
              type="button"
              onClick={() => setShowAdvanced(!showAdvanced)}
              className="text-[11px] text-violet-400 hover:text-violet-300 flex items-center gap-1"
            >
              <SlidersHorizontal className="w-3 h-3" />
              {showAdvanced
                ? "Sembunyikan Konfigurasi Selector Web Custom"
                : "Atur CSS Selector / Referer (Untuk Web Komik Custom)"}
            </button>

            {showAdvanced && (
              <div className="mt-2 p-2.5 bg-zinc-950/90 border border-zinc-800 rounded-lg space-y-2">
                <div>
                  <label className="block text-[11px] text-zinc-400 mb-0.5">
                    CSS Selector Gambar Chapter (Otomatis fallback ke{" "}
                    <code>ts_reader</code> &amp; <code>#readerarea img</code>)
                  </label>
                  <input
                    type="text"
                    value={customSelector}
                    onChange={(e) => setCustomSelector(e.target.value)}
                    className="w-full bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-xs text-zinc-200 font-mono"
                  />
                </div>
                <div>
                  <label className="block text-[11px] text-zinc-400 mb-0.5">
                    Custom Referer Header (Bypass proteksi hotlink CDN)
                  </label>
                  <input
                    type="text"
                    value={customReferer}
                    onChange={(e) => setCustomReferer(e.target.value)}
                    className="w-full bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-xs text-zinc-200 font-mono"
                  />
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Mode Folder Lokal */}
      {mode === "local" && (
        <div className="space-y-2">
          <label className="block text-xs text-zinc-300">
            Path Folder Chapter Lokal (Berisi Gambar .jpg / .png / .webp)
          </label>
          <input
            type="text"
            value={localFolderPath}
            onChange={(e) => setLocalFolderPath(e.target.value)}
            placeholder="D:\Comics\SoloLeveling\Chapter_01"
            className="field"
          />
        </div>
      )}

      {/* Pilihan Tipe Pemotongan Komik */}
      <div>
        <label className="block text-[11px] uppercase tracking-wider text-zinc-400 mb-1.5">
          Mode Pemotongan Panel (Slicer Engine)
        </label>
        <div className="grid grid-cols-3 gap-1.5 text-xs">
          <button
            type="button"
            onClick={() => setComicFormat("WebtoonVertical")}
            className={`py-1.5 px-2 rounded-lg border text-center transition ${
              comicFormat === "WebtoonVertical"
                ? "bg-violet-600/20 border-violet-500 text-violet-200 font-medium"
                : "bg-zinc-950 border-zinc-800 text-zinc-400"
            }`}
          >
            Webtoon / Manhwa (Vertikal)
          </button>
          <button
            type="button"
            onClick={() => setComicFormat("MangaRTL")}
            className={`py-1.5 px-2 rounded-lg border text-center transition ${
              comicFormat === "MangaRTL"
                ? "bg-violet-600/20 border-violet-500 text-violet-200 font-medium"
                : "bg-zinc-950 border-zinc-800 text-zinc-400"
            }`}
          >
            Manga Jepang (Kanan-Kiri)
          </button>
          <button
            type="button"
            onClick={() => setComicFormat("MangaLTR")}
            className={`py-1.5 px-2 rounded-lg border text-center transition ${
              comicFormat === "MangaLTR"
                ? "bg-violet-600/20 border-violet-500 text-violet-200 font-medium"
                : "bg-zinc-950 border-zinc-800 text-zinc-400"
            }`}
          >
            Komik Barat (Kiri-Kanan)
          </button>
        </div>
      </div>

      {/* Action Buttons */}
      <div className="flex gap-2 pt-1">
        <button
          type="button"
          disabled={isLoading}
          onClick={mode === "url" ? handleScrapeUrl : handleImportLocal}
          className="flex-1 btn-primary flex-1 py-2.5 flex items-center justify-center gap-2 transition shadow-lg shadow-violet-950/50"
        >
          {isLoading ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Memproses & Memotong Panel...
            </>
          ) : (
            <>
              <Globe className="w-4 h-4" />
              {mode === "url"
                ? "Scrape & Potong Panel Otomatis"
                : "Import & Potong Folder Lokal"}
            </>
          )}
        </button>

        <button
          type="button"
          onClick={handleLoadSimulationPanels}
          title="Muat contoh panel manhwa VoraToon untuk mencoba timeline & efek Montagem"
          className="btn-ghost flex items-center gap-1.5 transition"
        >
          <Sparkles className="w-3.5 h-3.5 text-amber-400" />
          Simulasi
        </button>
      </div>
    </div>
  );
};
