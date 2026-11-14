//! Portable mode detection & self-updater for ComiCut Portable (.zip) builds.
//!
//! Installer builds (NSIS `-setup.exe` / `.msi`) use `tauri-plugin-updater` (`latest.json`).
//! Portable builds (`.zip`) use this module (`portable.json` / GitHub Releases `-portable.zip`)
//! so updates extract in-place inside the portable folder instead of launching the NSIS installer.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

const UA: &str = concat!("ComiCut/", env!("CARGO_PKG_VERSION"));
const PORTABLE_PROGRESS_EVENT: &str = "portable-update-progress";

const PORTABLE_MANIFEST_URL: &str =
    "https://github.com/NoviHe/comicut/releases/latest/download/portable.json";
const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/NoviHe/comicut/releases/latest";
const INSTALLER_MANIFEST_URL: &str =
    "https://github.com/NoviHe/comicut/releases/latest/download/latest.json";

static UPDATE_BUSY: Mutex<bool> = Mutex::new(false);

struct UpdateBusyGuard;
impl UpdateBusyGuard {
    fn acquire() -> Result<Self, String> {
        let mut busy = UPDATE_BUSY.lock().unwrap();
        if *busy {
            return Err("Proses update portable sedang berjalan.".into());
        }
        *busy = true;
        Ok(Self)
    }
}
impl Drop for UpdateBusyGuard {
    fn drop(&mut self) {
        *UPDATE_BUSY.lock().unwrap() = false;
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstallModeInfo {
    pub is_portable: bool,
    /// "portable" | "installer"
    pub mode: String,
    pub current_version: String,
    pub exe_dir: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PortableUpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub version: String,
    pub date: Option<String>,
    pub body: Option<String>,
    pub download_url: String,
    pub sha256: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PortableProgress<'a> {
    /// "download" | "verify" | "extract" | "restart"
    stage: &'a str,
    downloaded: u64,
    total: Option<u64>,
    percent: u8,
}

#[derive(Deserialize)]
struct ManifestFile {
    version: String,
    notes: Option<String>,
    pub_date: Option<String>,
    platforms: Option< serde_json::Value >,
}

/// Determines whether the application in `exe_dir` is running as a Portable build
/// or an Installed (NSIS/MSI) build.
pub fn detect_portable_mode(exe_dir: &Path) -> bool {
    // 1. Explicit portable marker shipped inside ComiCut_*_x64-portable.zip
    if exe_dir.join("portable.txt").is_file() || exe_dir.join(".portable").is_file() {
        return true;
    }

    // 2. NSIS installer always places `uninstall.exe` alongside `ComiCut.exe`
    if exe_dir.join("uninstall.exe").is_file() {
        return false;
    }

    let path_lower = exe_dir.to_string_lossy().replace('/', "\\").to_ascii_lowercase();

    // 3. Standard Windows system/per-user installer directories (MSI / NSIS)
    if path_lower.contains("\\program files\\")
        || path_lower.contains("\\program files (x86)\\")
        || path_lower.contains("\\appdata\\local\\programs\\")
        || path_lower.contains("\\appdata\\local\\comicut")
        || path_lower.contains("\\windowsapps\\")
    {
        return false;
    }

    // 4. Local Cargo dev/release build folder without `portable.txt` marker -> default to installer mode
    if path_lower.ends_with("\\target\\debug") || path_lower.ends_with("\\target\\release") {
        return false;
    }

    // Standalone folder extracted anywhere else on disk -> Portable
    true
}

#[tauri::command]
pub fn get_install_mode() -> InstallModeInfo {
    let dir = super::deps_cmd::exe_dir().unwrap_or_default();
    let is_portable = detect_portable_mode(&dir);
    InstallModeInfo {
        is_portable,
        mode: if is_portable { "portable".into() } else { "installer".into() },
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        exe_dir: dir.to_string_lossy().to_string(),
    }
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(UA)
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())
}

fn parse_first_sha256(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_hexdigit())
        .find(|t| t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|t| t.to_ascii_lowercase())
}

/// Derive a portable `.zip` URL from an installer `.exe`/`.msi` URL when needed.
fn derive_portable_url(installer_url: &str, version: &str) -> String {
    let clean_ver = version.trim_start_matches(['v', 'V']);
    if let Some((prefix, _)) = installer_url.rsplit_once('/') {
        return format!("{prefix}/ComiCut_{clean_ver}_x64-portable.zip");
    }
    installer_url.replace("-setup.exe", "-portable.zip")
}

async fn fetch_portable_manifest(client: &reqwest::Client) -> Result<PortableUpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    // Tier 1: Dedicated `portable.json` manifest in GitHub Releases
    if let Ok(resp) = client
        .get(PORTABLE_MANIFEST_URL)
        .timeout(Duration::from_secs(12))
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(manifest) = resp.json::<ManifestFile>().await {
                let ver = manifest.version.trim().trim_start_matches(['v', 'V']).to_string();
                let plat = manifest
                    .platforms
                    .as_ref()
                    .and_then(|p| p.get("windows-x86_64").or_else(|| p.get("windows")));
                let url = plat
                    .and_then(|w| w.get("url"))
                    .and_then(|u| u.as_str())
                    .map(str::to_string);
                let sha256 = plat
                    .and_then(|w| w.get("sha256"))
                    .and_then(|s| s.as_str())
                    .and_then(parse_first_sha256);

                if let Some(download_url) = url {
                    let available =
                        super::deps_cmd::is_newer(&ver, &current_version).unwrap_or(false);
                    return Ok(PortableUpdateInfo {
                        available,
                        current_version,
                        version: ver,
                        date: manifest.pub_date,
                        body: manifest.notes,
                        download_url,
                        sha256,
                    });
                }
            }
        }
    }

    // Tier 2: GitHub Releases API (`/releases/latest`) looking for `*-portable.zip` asset
    if let Ok(resp) = client
        .get(GITHUB_LATEST_RELEASE_API)
        .timeout(Duration::from_secs(12))
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let tag = json["tag_name"].as_str().unwrap_or_default();
                let ver = tag.trim().trim_start_matches(['v', 'V']).to_string();
                if !ver.is_empty() {
                    let body = json["body"].as_str().map(str::to_string);
                    let date = json["published_at"].as_str().map(str::to_string);
                    let assets = json["assets"].as_array().cloned().unwrap_or_default();

                    let zip_asset = assets.iter().find(|a| {
                        let name = a["name"].as_str().unwrap_or_default().to_ascii_lowercase();
                        name.ends_with("-portable.zip")
                            || (name.contains("portable") && name.ends_with(".zip"))
                    });

                    let sha_asset = assets.iter().find(|a| {
                        let name = a["name"].as_str().unwrap_or_default().to_ascii_lowercase();
                        name.ends_with("-portable.zip.sha256")
                            || (name.contains("portable") && name.ends_with(".sha256"))
                    });

                    let download_url = zip_asset
                        .and_then(|a| a["browser_download_url"].as_str())
                        .map(str::to_string)
                        .unwrap_or_else(|| {
                            format!(
                                "https://github.com/NoviHe/comicut/releases/download/{tag}/ComiCut_{ver}_x64-portable.zip"
                            )
                        });

                    let mut sha256 = None;
                    if let Some(sha_url) =
                        sha_asset.and_then(|a| a["browser_download_url"].as_str())
                    {
                        if let Ok(r) = client.get(sha_url).timeout(Duration::from_secs(10)).send().await {
                            if let Ok(txt) = r.text().await {
                                sha256 = parse_first_sha256(&txt);
                            }
                        }
                    }

                    let available =
                        super::deps_cmd::is_newer(&ver, &current_version).unwrap_or(false);
                    return Ok(PortableUpdateInfo {
                        available,
                        current_version,
                        version: ver,
                        date,
                        body,
                        download_url,
                        sha256,
                    });
                }
            }
        }
    }

    // Tier 3: Fallback to `latest.json` and derive the portable `.zip` URL in the same release
    let resp = client
        .get(INSTALLER_MANIFEST_URL)
        .timeout(Duration::from_secs(12))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Gagal memeriksa rilis portable: {e}"))?;

    let manifest = resp
        .json::<ManifestFile>()
        .await
        .map_err(|e| format!("Format manifest rilis tidak valid: {e}"))?;

    let ver = manifest.version.trim().trim_start_matches(['v', 'V']).to_string();
    let plat = manifest
        .platforms
        .as_ref()
        .and_then(|p| p.get("windows-x86_64").or_else(|| p.get("windows")));

    let installer_url = plat
        .and_then(|w| w.get("url"))
        .and_then(|u| u.as_str())
        .unwrap_or("https://github.com/NoviHe/comicut/releases/latest/download/ComiCut_0.1.0_x64-setup.exe");

    let download_url = plat
        .and_then(|w| w.get("portable_url"))
        .and_then(|u| u.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| derive_portable_url(installer_url, &ver));

    let sha256 = plat
        .and_then(|w| w.get("portable_sha256"))
        .and_then(|s| s.as_str())
        .and_then(parse_first_sha256);

    let available = super::deps_cmd::is_newer(&ver, &current_version).unwrap_or(false);
    Ok(PortableUpdateInfo {
        available,
        current_version,
        version: ver,
        date: manifest.pub_date,
        body: manifest.notes,
        download_url,
        sha256,
    })
}

#[tauri::command]
pub async fn check_portable_update() -> Result<Option<PortableUpdateInfo>, String> {
    let client = http_client()?;
    let info = fetch_portable_manifest(&client).await?;
    if info.available {
        Ok(Some(info))
    } else {
        Ok(None)
    }
}

fn emit_progress(app: &AppHandle, stage: &str, downloaded: u64, total: Option<u64>, percent: u8) {
    let _ = app.emit(
        PORTABLE_PROGRESS_EVENT,
        PortableProgress {
            stage,
            downloaded,
            total,
            percent,
        },
    );
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Atomically swap `staged` into `target` on Windows, moving any running/existing target
/// to `<target>.old` so the running executable and sidecars can be replaced in-place.
fn swap_file(staged: &Path, target: &Path) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if target.exists() {
        let mut old_name = target
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".into());
        old_name.push_str(".old");
        let old_path = target.with_file_name(old_name);
        let _ = std::fs::remove_file(&old_path);
        std::fs::rename(target, &old_path).map_err(|e| {
            format!(
                "Tidak bisa memindahkan file lama {} (pastikan tidak dikunci proses lain): {e}",
                target.display()
            )
        })?;
    }
    std::fs::rename(staged, target).map_err(|e| {
        format!(
            "Gagal menempatkan file baru {}: {e}",
            target.display()
        )
    })
}

/// Sanitize a zip entry path and strip a shared top-level folder if present.
fn normalize_entry_relpath(entry_name: &str, strip_prefix: Option<&str>) -> Option<PathBuf> {
    let trimmed = match strip_prefix {
        Some(prefix) => entry_name.strip_prefix(prefix).unwrap_or(entry_name),
        None => entry_name,
    };
    let mut out = PathBuf::new();
    for comp in Path::new(trimmed).components() {
        if let Component::Normal(part) = comp {
            out.push(part);
        } else {
            return None;
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Detect if all entries in the archive live under a single top-level directory (e.g. `ComiCut_0.1.0_x64-portable/`).
fn detect_common_top_dir(names: &[String]) -> Option<String> {
    let mut prefix: Option<String> = None;
    for name in names {
        let norm = name.replace('\\', "/");
        let (first, rest) = norm.split_once('/')?;
        if first.is_empty() || rest.is_empty() {
            return None;
        }
        let candidate = format!("{first}/");
        match &prefix {
            None => prefix = Some(candidate),
            Some(p) if p != &candidate => return None,
            _ => {}
        }
    }
    prefix
}

fn extract_portable_zip(zip_path: &Path, exe_dir: &Path, current_exe: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| format!("Gagal membuka paket update portable: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Arsip portable rusak: {e}"))?;

    let mut file_names = Vec::new();
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            if !entry.is_dir() {
                file_names.push(entry.name().replace('\\', "/"));
            }
        }
    }
    if file_names.is_empty() {
        return Err("Arsip update portable kosong.".into());
    }

    let common_prefix = detect_common_top_dir(&file_names);
    let mut updated_main_exe = false;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let norm_name = entry.name().replace('\\', "/");
        let Some(rel_path) = normalize_entry_relpath(&norm_name, common_prefix.as_deref()) else {
            continue;
        };

        let is_root_exe = rel_path.parent().is_none_or(|p| p.as_os_str().is_empty())
            && rel_path
                .file_name()
                .map(|n| n.to_string_lossy().eq_ignore_ascii_case("comicut.exe"))
                .unwrap_or(false);

        let target_path = if is_root_exe {
            updated_main_exe = true;
            current_exe.to_path_buf()
        } else {
            exe_dir.join(&rel_path)
        };

        let mut staged_name = target_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "update.bin".into());
        staged_name.push_str(".new");
        let staged_path = target_path.with_file_name(staged_name);

        if let Some(parent) = staged_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let mut out = std::fs::File::create(&staged_path)
            .map_err(|e| format!("Tidak bisa menulis ke folder portable ({}): {e}", staged_path.display()))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        drop(out);

        swap_file(&staged_path, &target_path)?;
    }

    if !updated_main_exe {
        return Err("File ComiCut.exe tidak ditemukan di dalam arsip update portable.".into());
    }

    // Ensure portable marker stays present after update
    let marker = exe_dir.join("portable.txt");
    if !marker.exists() {
        let _ = std::fs::write(
            &marker,
            "ComiCut Portable Mode\nDeleting this file is safe; portable mode is kept as long as uninstall.exe is absent.\n",
        );
    }

    Ok(())
}

/// Download the portable `.zip`, verify SHA-256 (if available), extract in-place over the
/// current portable folder, and relaunch the updated `ComiCut.exe`.
#[tauri::command]
pub async fn install_portable_update(
    app: AppHandle,
    download_url: String,
    sha256: Option<String>,
) -> Result<(), String> {
    let _guard = UpdateBusyGuard::acquire()?;
    let orig_exe = std::env::current_exe()
        .map_err(|e| format!("Gagal membaca lokasi aplikasi saat ini: {e}"))?;
    let exe_dir = orig_exe
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Folder aplikasi portable tidak ditemukan.".to_string())?;

    let client = http_client()?;
    let tmp_zip = exe_dir.join("ComiCut-portable-update.zip.download");

    let run: Result<(), String> = async {
        let mut resp = client
            .get(&download_url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("Gagal mengunduh paket portable: {e}"))?;

        let total = resp.content_length();
        let mut file = tokio::fs::File::create(&tmp_zip).await.map_err(|e| {
            format!(
                "Tidak bisa menulis di folder portable ({}). Pastikan folder tidak Read-Only: {e}",
                exe_dir.display()
            )
        })?;

        let mut hasher = Sha256::new();
        let (mut done, mut last_emit) = (0u64, 0u64);
        emit_progress(&app, "download", 0, total, 0);

        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| format!("Unduhan update portable terputus: {e}"))?
        {
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            done += chunk.len() as u64;
            if done - last_emit >= 256 * 1024 {
                last_emit = done;
                let pct = total
                    .filter(|&t| t > 0)
                    .map(|t| ((done * 95) / t).min(95) as u8)
                    .unwrap_or(50);
                emit_progress(&app, "download", done, total, pct);
            }
        }
        file.flush().await.map_err(|e| e.to_string())?;
        drop(file);
        let actual_sha = hex_lower(&hasher.finalize());

        emit_progress(&app, "verify", done, total, 96);

        // Verify SHA-256 against manifest or companion `.sha256` file if present
        let mut expected_sha = sha256.and_then(|s| parse_first_sha256(&s));
        if expected_sha.is_none() {
            let sha_url = format!("{download_url}.sha256");
            if let Ok(r) = client.get(&sha_url).timeout(Duration::from_secs(10)).send().await {
                if r.status().is_success() {
                    if let Ok(txt) = r.text().await {
                        expected_sha = parse_first_sha256(&txt);
                    }
                }
            }
        }
        if let Some(expected) = expected_sha {
            if expected != actual_sha {
                return Err(
                    "Verifikasi SHA-256 paket portable gagal — file unduhan rusak atau tidak sesuai rilis resmi."
                        .into(),
                );
            }
        }

        emit_progress(&app, "extract", done, total, 98);
        let (zip_clone, dir_clone, exe_clone) =
            (tmp_zip.clone(), exe_dir.clone(), orig_exe.clone());
        tokio::task::spawn_blocking(move || extract_portable_zip(&zip_clone, &dir_clone, &exe_clone))
            .await
            .map_err(|e| e.to_string())??;

        emit_progress(&app, "restart", done, total, 100);
        Ok(())
    }
    .await;

    let _ = std::fs::remove_file(&tmp_zip);
    run?;

    // Relaunch the newly extracted executable at `orig_exe` and close the current process
    let relaunch_exe = orig_exe.clone();
    let app_clone = app.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(350)).await;
        let _ = std::process::Command::new(&relaunch_exe)
            .current_dir(relaunch_exe.parent().unwrap_or_else(|| Path::new(".")))
            .spawn();
        app_clone.exit(0);
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_portable_url() {
        let installer = "https://github.com/NoviHe/comicut/releases/download/v0.2.0/ComiCut_0.2.0_x64-setup.exe";
        assert_eq!(
            derive_portable_url(installer, "0.2.0"),
            "https://github.com/NoviHe/comicut/releases/download/v0.2.0/ComiCut_0.2.0_x64-portable.zip"
        );
    }

    #[test]
    fn test_common_top_dir() {
        let wrapped = vec![
            "ComiCut_0.1.0_x64-portable/ComiCut.exe".to_string(),
            "ComiCut_0.1.0_x64-portable/ffmpeg.exe".to_string(),
        ];
        assert_eq!(
            detect_common_top_dir(&wrapped).as_deref(),
            Some("ComiCut_0.1.0_x64-portable/")
        );

        let flat = vec!["ComiCut.exe".to_string(), "ffmpeg.exe".to_string()];
        assert_eq!(detect_common_top_dir(&flat), None);
    }

    #[test]
    fn test_detect_portable_mode_paths() {
        assert!(!detect_portable_mode(Path::new(
            r"C:\Program Files\ComiCut"
        )));
        assert!(!detect_portable_mode(Path::new(
            r"C:\Users\Admin\AppData\Local\Programs\ComiCut"
        )));
        assert!(!detect_portable_mode(Path::new(
            r"D:\Laravel\comicut\src-tauri\target\release"
        )));
        assert!(detect_portable_mode(Path::new(
            r"D:\PortableApps\ComiCut"
        )));
    }
}
