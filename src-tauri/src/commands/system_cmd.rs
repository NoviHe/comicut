use serde::Serialize;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepStatus {
    pub id: &'static str,
    pub name: &'static str,
    pub purpose: &'static str,
    pub required: bool,
    pub found: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub outdated: bool,
    pub install_hint: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub cpu: String,
    pub threads: usize,
    pub gpu_encoder: Option<String>,
    /// Detected video adapters, e.g. "NVIDIA GeForce RTX 3060".
    pub gpus: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepReport {
    pub deps: Vec<DepStatus>,
    pub system: SystemInfo,
}

fn quiet(program: &str) -> Command {
    let mut c = Command::new(program);
    c.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let exts: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ".bat", ""] } else { &[""] };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .flat_map(|d| exts.iter().map(move |e| d.join(format!("{name}{e}"))))
        .find(|p| p.is_file())
}

fn first_line(program: &str, args: &[&str]) -> Option<String> {
    let out = quiet(program).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout).lines().next().map(|l| l.trim().to_string())
}

// Days since 1970-01-01 (Hinnant's civil-from-days inverse).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// yt-dlp is date-versioned (YYYY.MM.DD); YouTube breaks old builds, so >90 days old counts as outdated.
fn ytdlp_outdated(version: &str) -> bool {
    let p: Vec<i64> = version.split('.').take(3).filter_map(|x| x.parse().ok()).collect();
    if p.len() < 3 {
        return false;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 / 86400)
        .unwrap_or(0);
    now - days_from_civil(p[0], p[1], p[2]) > 90
}

fn dep(
    id: &'static str,
    name: &'static str,
    purpose: &'static str,
    required: bool,
    bin: &str,
    ver_args: &[&str],
    install_hint: &'static str,
) -> DepStatus {
    let path = find_in_path(bin);
    let version = path.as_ref().and_then(|_| first_line(bin, ver_args)).map(|l| {
        // "ffmpeg version 8.1.2-full_build ..." -> "8.1.2-full_build"; "deno 2.1.0 (...)" -> "2.1.0"
        let t: Vec<&str> = l.split_whitespace().collect();
        match t.as_slice() {
            ["ffmpeg", "version", v, ..] => v.to_string(),
            ["deno", v, ..] => v.to_string(),
            _ => l,
        }
    });
    let outdated = id == "yt-dlp" && version.as_deref().is_some_and(ytdlp_outdated);
    DepStatus {
        id,
        name,
        purpose,
        required,
        found: path.is_some() && version.is_some(),
        version,
        path: path.map(|p| p.to_string_lossy().to_string()),
        outdated,
        install_hint,
    }
}

#[tauri::command]
pub async fn check_dependencies() -> Result<DepReport, String> {
    tokio::task::spawn_blocking(|| {
        let deps = vec![
            dep("ffmpeg", "FFmpeg", "Render video MP4 & ekstrak audio", true, "ffmpeg", &["-version"], "winget install Gyan.FFmpeg"),
            dep("yt-dlp", "yt-dlp", "Unduh audio dari link / pencarian", false, "yt-dlp", &["--version"], "pip install -U \"yt-dlp[default]\""),
            dep("node", "Node.js", "JS runtime untuk yt-dlp (YouTube)", false, "node", &["--version"], "winget install OpenJS.NodeJS.LTS"),
            dep("deno", "Deno", "Alternatif JS runtime untuk yt-dlp", false, "deno", &["--version"], "winget install DenoLand.Deno"),
        ];
        let system = SystemInfo {
            cpu: std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "CPU tidak diketahui".into()),
            threads: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
            gpu_encoder: if find_in_path("ffmpeg").is_some() {
                let g = crate::core::video_renderer::encoder::detect();
                (g.encoder != crate::core::video_renderer::encoder::Encoder::X264).then(|| g.encoder.label().to_string())
            } else {
                None
            },
            gpus: crate::core::video_renderer::encoder::detect().names.clone(),
        };
        DepReport { deps, system }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Update yt-dlp: pip first (matches how it is usually installed), then its built-in updater.
#[tauri::command]
pub async fn update_ytdlp() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        for py in ["python", "py"] {
            if let Ok(o) = quiet(py).args(["-m", "pip", "install", "-U", "yt-dlp[default]"]).output() {
                if o.status.success() {
                    return Ok("yt-dlp diperbarui via pip.".to_string());
                }
            }
        }
        let o = quiet("yt-dlp").arg("-U").output().map_err(|e| format!("Gagal menjalankan yt-dlp: {e}"))?;
        let text = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
        if o.status.success() {
            Ok(text.lines().last().unwrap_or("selesai").to_string())
        } else {
            Err(format!("Update gagal: {}", text.lines().last().unwrap_or("unknown")))
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_math() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11017);
        assert!(ytdlp_outdated("2020.01.01"));
        assert!(!ytdlp_outdated("2999.01.01"));
        assert!(!ytdlp_outdated("garbage"));
    }
}
