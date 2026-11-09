//! In-app installer / updater for external tools (FFmpeg, yt-dlp, Deno).
//!
//! FFmpeg ships inside the installer as a Tauri sidecar (next to ComiCut.exe). yt-dlp and Deno
//! — and newer FFmpeg builds — are downloaded on demand from their official release channels
//! into an app-private folder (`%LOCALAPPDATA%\ComiCut\bin`), verified against the publisher's
//! SHA-256 checksum. Nothing is installed system-wide. Resolution order: see [`tool`].

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

const UA: &str = concat!("ComiCut/", env!("CARGO_PKG_VERSION"));
const PROGRESS_EVENT: &str = "dep-progress";

enum Kind {
    /// The download itself is the executable.
    Exe,
    /// A zip archive; the listed files are extracted (by basename) from anywhere inside it.
    Zip,
}

struct Spec {
    name: &'static str,
    kind: Kind,
    url: &'static str,
    checksum_url: &'static str,
    /// For multi-file checksum lists (`<hash>  <file>`), the file name to look up.
    checksum_entry: Option<&'static str>,
    files: &'static [&'static str],
}

fn spec(id: &str) -> Option<Spec> {
    Some(match id {
        "ffmpeg" => Spec {
            name: "FFmpeg",
            kind: Kind::Zip,
            url: "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
            checksum_url: "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip.sha256",
            checksum_entry: None,
            files: &["ffmpeg.exe", "ffprobe.exe"],
        },
        "yt-dlp" => Spec {
            name: "yt-dlp",
            kind: Kind::Exe,
            url: "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
            checksum_url: "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS",
            checksum_entry: Some("yt-dlp.exe"),
            files: &["yt-dlp.exe"],
        },
        "deno" => Spec {
            name: "Deno",
            kind: Kind::Zip,
            url: "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip",
            checksum_url: "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip.sha256sum",
            checksum_entry: None,
            files: &["deno.exe"],
        },
        _ => return None,
    })
}

pub fn is_installable(id: &str) -> bool {
    spec(id).is_some()
}

pub fn managed_bin_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("ComiCut")
        .join("bin")
}

pub fn is_managed(path: &Path) -> bool {
    path.parent().is_some_and(|p| p == managed_bin_dir())
}

/// Folder of ComiCut.exe — where Tauri places bundled sidecars (FFmpeg).
fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(Path::to_path_buf)
}

pub fn is_bundled(path: &Path) -> bool {
    path.parent().is_some_and(|p| Some(p.to_path_buf()) == exe_dir())
}

/// Resolve a tool to an absolute path using the app's priority order:
/// managed (updated from the UI) > bundled sidecar > system PATH.
///
/// Always use this instead of a bare `Command::new("ffmpeg")`: on Windows, Rust searches the
/// exe's own folder *before* PATH, which would make the bundled copy shadow a newer managed one.
pub fn tool(bin: &str) -> PathBuf {
    let exts: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ".bat", ""] } else { &[""] };
    std::env::var_os("PATH")
        .and_then(|p| {
            std::env::split_paths(&p)
                .flat_map(|d| exts.iter().map(move |e| d.join(format!("{bin}{e}"))))
                .find(|p| p.is_file())
        })
        .unwrap_or_else(|| PathBuf::from(bin))
}

/// PATH = [managed bin, exe dir (bundled sidecars), ...system PATH], and clean leftovers from
/// previous updates. Child processes (e.g. yt-dlp looking for ffmpeg/deno) inherit this order.
/// Must be called once at startup, before any other threads are spawned.
pub fn init_managed_path() {
    let dir = managed_bin_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.ends_with(".old") || n.ends_with(".new") || n.ends_with(".download") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let mut front = vec![dir];
    front.extend(exe_dir());
    let mut paths = front.clone();
    if let Some(p) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&p).filter(|x| !front.contains(x)));
    }
    if let Ok(joined) = std::env::join_paths(paths) {
        std::env::set_var("PATH", joined);
    }
    std::thread::spawn(prune_stale_managed_ffmpeg);
}

/// After a ComiCut update the bundled FFmpeg may be newer than a copy updated earlier from the
/// UI; drop the managed copy then so it doesn't shadow the bundled one.
fn prune_stale_managed_ffmpeg() {
    let (Some(exe), managed) = (exe_dir(), managed_bin_dir()) else { return };
    let ver = |p: PathBuf| -> Option<String> {
        let mut c = std::process::Command::new(p);
        c.arg("-version").stdin(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let out = c.output().ok()?;
        let line = String::from_utf8_lossy(&out.stdout).lines().next()?.to_string();
        line.split_whitespace().nth(2).map(str::to_string)
    };
    let (bundled, local) = (exe.join("ffmpeg.exe"), managed.join("ffmpeg.exe"));
    if !bundled.is_file() || !local.is_file() {
        return;
    }
    if let (Some(b), Some(l)) = (ver(bundled), ver(local)) {
        // Bundled is same or newer -> managed copy is redundant.
        if is_newer(&l, &b) == Some(false) {
            for f in ["ffmpeg.exe", "ffprobe.exe"] {
                let _ = std::fs::remove_file(managed.join(f));
            }
        }
    }
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(UA)
        .connect_timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- version check

/// "v2.9.8" / "9.0.2-essentials_build-www.gyan.dev" / "2026.10.26" -> numeric parts.
fn parse_ver(s: &str) -> Vec<u64> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let end = s.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(s.len());
    s[..end].split('.').filter_map(|p| p.parse().ok()).collect()
}

/// `Some(true)` if `latest` is newer than `installed`; `None` if `installed` isn't comparable
/// (e.g. FFmpeg git builds like "N-12345-g...").
fn is_newer(latest: &str, installed: &str) -> Option<bool> {
    let (l, i) = (parse_ver(latest), parse_ver(installed));
    if l.is_empty() || i.is_empty() {
        return None;
    }
    let n = l.len().max(i.len());
    let pad = |v: &Vec<u64>, k: usize| v.get(k).copied().unwrap_or(0);
    for k in 0..n {
        if pad(&l, k) != pad(&i, k) {
            return Some(pad(&l, k) > pad(&i, k));
        }
    }
    Some(false)
}

async fn latest_version(client: &reqwest::Client, id: &str) -> Result<String, String> {
    let get_text = |url: &'static str| async move {
        client
            .get(url)
            .timeout(Duration::from_secs(20))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())
    };
    let v = match id {
        "ffmpeg" => get_text("https://www.gyan.dev/ffmpeg/builds/release-version").await?,
        "deno" => get_text("https://dl.deno.land/release-latest.txt").await?,
        "yt-dlp" => {
            let body = get_text("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest").await?;
            let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
            json["tag_name"].as_str().ok_or("tag_name tidak ada")?.to_string()
        }
        _ => return Err("tidak didukung".into()),
    };
    let v = v.trim().trim_start_matches('v').to_string();
    if parse_ver(&v).is_empty() {
        return Err(format!("format versi tidak dikenal: {v}"));
    }
    Ok(v)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepUpdate {
    pub id: String,
    pub installed: Option<String>,
    pub latest: Option<String>,
    pub update_available: bool,
    pub error: Option<String>,
}

/// Query the official release channels for the newest version of every installable tool.
#[tauri::command]
pub async fn check_dependency_updates() -> Result<Vec<DepUpdate>, String> {
    let client = client()?;
    let ids = ["ffmpeg", "yt-dlp", "deno"];
    let latest = futures_join(&client, &ids).await;
    let installed = tokio::task::spawn_blocking(move || {
        ids.iter().map(|id| super::system_cmd::installed_version(id)).collect::<Vec<_>>()
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(ids
        .iter()
        .zip(latest)
        .zip(installed)
        .map(|((id, latest), installed)| {
            let (latest, error) = match latest {
                Ok(v) => (Some(v), None),
                Err(e) => (None, Some(e)),
            };
            let update_available = match (&latest, &installed) {
                (Some(l), Some(i)) => is_newer(l, i).unwrap_or(false),
                _ => false,
            };
            DepUpdate { id: id.to_string(), installed, latest, update_available, error }
        })
        .collect())
}

async fn futures_join(client: &reqwest::Client, ids: &[&str; 3]) -> Vec<Result<String, String>> {
    let (a, b, c) = tokio::join!(
        latest_version(client, ids[0]),
        latest_version(client, ids[1]),
        latest_version(client, ids[2])
    );
    vec![a, b, c]
}

// ---------------------------------------------------------------- install / update

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    id: &'a str,
    /// "download" | "verify" | "install" | "done"
    stage: &'a str,
    downloaded: u64,
    total: Option<u64>,
}

static BUSY: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct BusyGuard(String);
impl BusyGuard {
    fn acquire(id: &str) -> Result<Self, String> {
        let mut busy = BUSY.lock().unwrap();
        if busy.iter().any(|b| b == id) {
            return Err(format!("{id} sedang diinstal."));
        }
        busy.push(id.to_string());
        Ok(Self(id.to_string()))
    }
}
impl Drop for BusyGuard {
    fn drop(&mut self) {
        BUSY.lock().unwrap().retain(|b| b != &self.0);
    }
}

fn emit(app: &AppHandle, id: &str, stage: &str, downloaded: u64, total: Option<u64>) {
    let _ = app.emit(PROGRESS_EVENT, Progress { id, stage, downloaded, total });
}

/// Stream `url` to `dest`, emitting progress, and return the SHA-256 (lowercase hex).
async fn download(app: &AppHandle, client: &reqwest::Client, id: &str, url: &str, dest: &Path) -> Result<String, String> {
    let mut resp = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Gagal mengunduh: {e}"))?;
    let total = resp.content_length();
    let mut file = tokio::fs::File::create(dest).await.map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let (mut done, mut last_emit) = (0u64, 0u64);
    emit(app, id, "download", 0, total);
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("Unduhan terputus: {e}"))? {
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
        if done - last_emit >= 512 * 1024 {
            last_emit = done;
            emit(app, id, "download", done, total);
        }
    }
    file.flush().await.map_err(|e| e.to_string())?;
    emit(app, id, "download", done, total);
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Extract the expected hash from a checksum file. Handles bare hashes, `sha256sum`-style
/// lists (`<hash>  <file>`), and PowerShell `Get-FileHash` output.
fn parse_checksum(text: &str, entry: Option<&str>) -> Option<String> {
    let is_hash = |t: &str| t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit());
    let scope: Vec<&str> = match entry {
        Some(name) => text
            .lines()
            .filter(|l| l.split_whitespace().last().map(|f| f.trim_start_matches('*')) == Some(name))
            .collect(),
        None => vec![text],
    };
    scope
        .iter()
        .flat_map(|s| s.split(|c: char| !c.is_ascii_hexdigit()))
        .find(|t| is_hash(t))
        .map(|t| t.to_ascii_lowercase())
}

/// Atomically-ish swap `staged` into `target`. A running exe can't be overwritten on Windows,
/// but it can be renamed — so move the old one aside (cleaned up on next start).
fn replace(staged: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        let old = target.with_extension("exe.old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(target, &old)
            .map_err(|e| format!("Tidak bisa mengganti {} (sedang dipakai?): {e}", target.display()))?;
    }
    std::fs::rename(staged, target).map_err(|e| e.to_string())
}

fn extract(zip_path: &Path, dir: &Path, files: &[&str]) -> Result<(), String> {
    let f = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(f).map_err(|e| format!("Arsip rusak: {e}"))?;
    let mut found = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        // Only the basename is used for the output path (no zip-slip).
        let base = Path::new(entry.name())
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let Some(want) = files.iter().find(|w| w.eq_ignore_ascii_case(&base)) else { continue };
        if found.contains(want) {
            continue;
        }
        let staged = dir.join(format!("{want}.new"));
        let mut out = std::fs::File::create(&staged).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        drop(out);
        replace(&staged, &dir.join(want))?;
        found.push(*want);
    }
    if found.len() < files.len() {
        return Err(format!("File tidak ditemukan di arsip: {:?}", files));
    }
    Ok(())
}

/// Download, verify (SHA-256) and install the latest official build of `id` into the
/// managed bin dir. Used for both first install and updates.
#[tauri::command]
pub async fn install_dependency(app: AppHandle, id: String) -> Result<String, String> {
    let spec = spec(&id).ok_or_else(|| format!("{id} tidak bisa diinstal otomatis."))?;
    let _guard = BusyGuard::acquire(&id)?;
    let client = client()?;
    let dir = managed_bin_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!("{id}.download"));

    let result: Result<(), String> = async {
        let actual = download(&app, &client, &id, spec.url, &tmp).await?;

        emit(&app, &id, "verify", 0, None);
        let sums = client
            .get(spec.checksum_url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("Gagal mengambil checksum: {e}"))?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        let expected = parse_checksum(&sums, spec.checksum_entry).ok_or("Checksum tidak ditemukan.")?;
        if expected != actual {
            return Err("Verifikasi SHA-256 gagal — file tidak sesuai rilis resmi, dibatalkan.".into());
        }

        emit(&app, &id, "install", 0, None);
        let (tmp2, dir2) = (tmp.clone(), dir.clone());
        match spec.kind {
            Kind::Exe => replace(&tmp2, &dir2.join(spec.files[0])),
            Kind::Zip => tokio::task::spawn_blocking(move || extract(&tmp2, &dir2, spec.files))
                .await
                .map_err(|e| e.to_string())?,
        }
    }
    .await;

    let _ = std::fs::remove_file(&tmp);
    result?;
    emit(&app, &id, "done", 0, None);

    let id2 = id.clone();
    let version = tokio::task::spawn_blocking(move || super::system_cmd::installed_version(&id2))
        .await
        .ok()
        .flatten();
    Ok(format!("{} {} terpasang.", spec.name, version.unwrap_or_default()).replace("  ", " "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(parse_ver("v2.9.8"), vec![2, 9, 8]);
        assert_eq!(parse_ver("9.0.2-essentials_build-www.gyan.dev"), vec![9, 0, 2]);
        assert_eq!(is_newer("9.0.2", "8.1.2-full_build"), Some(true));
        assert_eq!(is_newer("2026.10.26", "2026.10.26"), Some(false));
        assert_eq!(is_newer("2.9.8", "2.10.0"), Some(false));
        assert_eq!(is_newer("9.0", "N-12345-gabc"), None);
    }

    #[test]
    fn checksums() {
        let h = "f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d";
        let list = format!("{}  yt-dlp_x86.exe\n{h}  yt-dlp.exe\n", "0".repeat(64));
        assert_eq!(parse_checksum(&list, Some("yt-dlp.exe")).as_deref(), Some(h));
        let ps = format!("\nAlgorithm : SHA256\nHash      : {}\nPath      : C:\\a\\x.zip\n", h.to_uppercase());
        assert_eq!(parse_checksum(&ps, None).as_deref(), Some(h));
        assert_eq!(parse_checksum(h, None).as_deref(), Some(h));
    }
}
