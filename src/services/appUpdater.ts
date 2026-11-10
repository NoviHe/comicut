import { isTauri } from "./tauriCommands";

export interface AppUpdateInfo {
  available: boolean;
  currentVersion: string;
  version: string;
  date?: string;
  body?: string;
  rawUpdate?: any;
}

export type DownloadProgressCallback = (percent: number, downloadedBytes: number, totalBytes?: number) => void;

/**
 * Checks GitHub releases endpoint for a new ComiCut version.
 * Returns AppUpdateInfo if an update is found, or null if up-to-date.
 */
export async function checkComiCutUpdate(): Promise<AppUpdateInfo | null> {
  if (!isTauri()) {
    return null;
  }

  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check({
      timeout: 10000,
    });

    if (!update) {
      return null;
    }

    return {
      available: true,
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
 * Downloads and installs the update, with progress callback.
 */
export async function installComiCutUpdate(
  updateInfo: AppUpdateInfo,
  onProgress: DownloadProgressCallback
): Promise<void> {
  if (!updateInfo.rawUpdate) {
    throw new Error("No update handle available.");
  }

  const update = updateInfo.rawUpdate;
  let totalBytes = 0;
  let downloadedBytes = 0;

  await update.downloadAndInstall((event: any) => {
    if (event.event === "Started") {
      totalBytes = event.data?.contentLength || 0;
      onProgress(0, 0, totalBytes);
    } else if (event.event === "Progress") {
      downloadedBytes += event.data?.chunkLength || 0;
      const percent = totalBytes > 0 ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100)) : 50;
      onProgress(percent, downloadedBytes, totalBytes);
    } else if (event.event === "Finished") {
      onProgress(100, totalBytes, totalBytes);
    }
  });
}

/**
 * Dismisses the update and frees backend resource.
 */
export async function dismissComiCutUpdate(updateInfo: AppUpdateInfo): Promise<void> {
  if (updateInfo.rawUpdate) {
    try {
      await updateInfo.rawUpdate.close();
    } catch {
      // Ignored
    }
  }
}
