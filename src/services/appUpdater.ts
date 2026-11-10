import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isTauri } from "./tauriCommands";

export type UpdateDistributionMode = "portable" | "installer";

export interface InstallModeInfo {
  isPortable: boolean;
  mode: UpdateDistributionMode;
  currentVersion: string;
  exeDir: string;
}

export interface AppUpdateInfo {
  available: boolean;
  mode: UpdateDistributionMode;
  currentVersion: string;
  version: string;
  date?: string;
  body?: string;
  downloadUrl?: string;
  sha256?: string;
  rawUpdate?: any;
}

export type PortableUpdateStage = "download" | "verify" | "extract" | "restart";

interface PortableProgressPayload {
  stage: PortableUpdateStage;
  downloaded: number;
  total?: number;
  percent: number;
}

export type DownloadProgressCallback = (
  percent: number,
  downloadedBytes: number,
  totalBytes?: number,
  stage?: PortableUpdateStage
) => void;

/**
 * Queries Rust backend to check whether ComiCut is running as a Portable (.zip)
 * or Installed (NSIS/MSI) build.
 */
export async function getInstallMode(): Promise<InstallModeInfo> {
  if (!isTauri()) {
    return {
      isPortable: false,
      mode: "installer",
      currentVersion: "0.1.1",
      exeDir: "",
    };
  }
  try {
    return await invoke<InstallModeInfo>("get_install_mode");
  } catch {
    return {
      isPortable: false,
      mode: "installer",
      currentVersion: "0.1.1",
      exeDir: "",
    };
  }
}

/**
 * Checks GitHub releases endpoint for a new ComiCut version.
 * - Portable mode: checks `portable.json` / `-portable.zip` release asset.
 * - Installer mode: checks `latest.json` via `@tauri-apps/plugin-updater`.
 */
export async function checkComiCutUpdate(): Promise<AppUpdateInfo | null> {
  if (!isTauri()) {
    return null;
  }

  try {
    const modeInfo = await getInstallMode();

    if (modeInfo.isPortable) {
      const portableUpdate = await invoke<{
        available: boolean;
        currentVersion: string;
        version: string;
        date?: string;
        body?: string;
        downloadUrl: string;
        sha256?: string;
      } | null>("check_portable_update");

      if (!portableUpdate || !portableUpdate.available) {
        return null;
      }

      return {
        available: true,
        mode: "portable",
        currentVersion: portableUpdate.currentVersion,
        version: portableUpdate.version,
        date: portableUpdate.date,
        body: portableUpdate.body,
        downloadUrl: portableUpdate.downloadUrl,
        sha256: portableUpdate.sha256,
      };
    }

    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check({
      timeout: 10000,
    });

    if (!update) {
      return null;
    }

    return {
      available: true,
      mode: "installer",
      currentVersion: update.currentVersion,
      version: update.version,
      date: update.date,
      body: update.body,
      rawUpdate: update,
    };
  } catch (err: any) {
    console.warn("Update check failed:", err);
    return null;
  }
}

/**
 * Downloads and installs the update according to the active mode:
 * - Portable mode: downloads `-portable.zip`, verifies SHA-256, extracts in-place over the portable folder, and restarts.
 * - Installer mode: runs the signed NSIS/MSI updater package.
 */
export async function installComiCutUpdate(
  updateInfo: AppUpdateInfo,
  onProgress: DownloadProgressCallback
): Promise<void> {
  if (updateInfo.mode === "portable") {
    if (!updateInfo.downloadUrl) {
      throw new Error("URL unduhan paket portable tidak tersedia.");
    }

    const unlisten = await listen<PortableProgressPayload>(
      "portable-update-progress",
      (event) => {
        const p = event.payload;
        onProgress(p.percent, p.downloaded, p.total, p.stage);
      }
    );

    try {
      await invoke("install_portable_update", {
        downloadUrl: updateInfo.downloadUrl,
        sha256: updateInfo.sha256 ?? null,
      });
    } finally {
      unlisten();
    }
    return;
  }

  if (!updateInfo.rawUpdate) {
    throw new Error("No update handle available.");
  }

  const update = updateInfo.rawUpdate;
  let totalBytes = 0;
  let downloadedBytes = 0;

  await update.downloadAndInstall((event: any) => {
    if (event.event === "Started") {
      totalBytes = event.data?.contentLength || 0;
      onProgress(0, 0, totalBytes, "download");
    } else if (event.event === "Progress") {
      downloadedBytes += event.data?.chunkLength || 0;
      const percent =
        totalBytes > 0
          ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100))
          : 50;
      onProgress(percent, downloadedBytes, totalBytes, "download");
    } else if (event.event === "Finished") {
      onProgress(100, totalBytes, totalBytes, "restart");
    }
  });
}

/**
 * Dismisses the update and frees backend resource.
 */
export async function dismissComiCutUpdate(
  updateInfo: AppUpdateInfo
): Promise<void> {
  if (updateInfo.rawUpdate) {
    try {
      await updateInfo.rawUpdate.close();
    } catch {
      // Ignored
    }
  }
}
