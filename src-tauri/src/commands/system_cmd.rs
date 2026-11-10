use serde::Serialize;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDiagnostics {
    pub app_name: String,
    pub app_version: String,
    pub os_name: String,
    pub os_build: String,
    pub kernel: String,
    pub cpu_model: String,
    pub architecture: String,
    pub cpu_cores_physical: u32,
    pub cpu_cores_logical: usize,
    pub total_memory_gb: u32,
    pub gpus: Vec<String>,
}

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
    /// Can be installed/updated from inside the app.
    pub installable: bool,
    /// The binary on PATH is the app-managed copy (%LOCALAPPDATA%\ComiCut\bin).
    pub managed: bool,
    /// The binary on PATH is the sidecar shipped inside the installer (next to ComiCut.exe).
    pub bundled: bool,
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
    let mut c = Command::new(super::deps_cmd::tool(program));
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

fn version_of(bin: &str, ver_args: &[&str]) -> Option<String> {
    find_in_path(bin)?;
    first_line(bin, ver_args).map(|l| {
        // "ffmpeg version 8.1.2-full_build ..." -> "8.1.2-full_build"; "deno 2.1.0 (...)" -> "2.1.0"
        let t: Vec<&str> = l.split_whitespace().collect();
        match t.as_slice() {
            ["ffmpeg", "version", v, ..] => v.to_string(),
            ["deno", v, ..] => v.to_string(),
            _ => l.trim_start_matches('v').to_string(),
        }
    })
}

/// Installed version of a known tool id (binary name == id), as resolved on PATH.
pub(crate) fn installed_version(id: &str) -> Option<String> {
    let args: &[&str] = if id == "ffmpeg" { &["-version"] } else { &["--version"] };
    version_of(id, args)
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
    let version = version_of(bin, ver_args);
    let outdated = id == "yt-dlp" && version.as_deref().is_some_and(ytdlp_outdated);
    let managed = path.as_deref().is_some_and(super::deps_cmd::is_managed);
    let bundled = path.as_deref().is_some_and(super::deps_cmd::is_bundled);
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
        installable: super::deps_cmd::is_installable(id),
        managed,
        bundled,
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

#[tauri::command]
pub fn window_minimize(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn window_toggle_maximize(window: tauri::Window) -> Result<(), String> {
    if window.is_maximized().unwrap_or(false) {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn window_close(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn app_restart(app: tauri::AppHandle) {
    app.restart();
}

#[tauri::command]
pub async fn get_device_diagnostics() -> Result<DeviceDiagnostics, String> {
    tokio::task::spawn_blocking(|| {
        let app_name = "ComiCut Studio".to_string();
        let app_version = env!("CARGO_PKG_VERSION").to_string();
        let architecture = std::env::consts::ARCH.to_string();

        #[cfg(windows)]
        {
            let script = r#"& {
                $os = Get-CimInstance Win32_OperatingSystem;
                $cpu = (Get-ItemProperty 'HKLM:\HARDWARE\DESCRIPTION\System\CentralProcessor\0' -ErrorAction SilentlyContinue).ProcessorNameString;
                if (-not $cpu) { $cpu = (Get-CimInstance Win32_Processor).Name };
                $cs = Get-CimInstance Win32_ComputerSystem;
                $gpus = (Get-CimInstance Win32_VideoController).Name -join ';;';
                $cores = (Get-CimInstance Win32_Processor).NumberOfCores;
                $logical = [System.Environment]::ProcessorCount;
                [PSCustomObject]@{
                    os = $os.Caption;
                    build = $os.BuildNumber;
                    cpu = $cpu.Trim();
                    mem = [math]::Round($cs.TotalPhysicalMemory / 1GB);
                    cores = $cores;
                    logical = $logical;
                    gpus = $gpus;
                } | ConvertTo-Json -Compress
            }"#;

            if let Ok(output) = quiet("powershell").args(["-NoProfile", "-NonInteractive", "-Command", script]).output() {
                if output.status.success() {
                    let out_str = String::from_utf8_lossy(&output.stdout);
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out_str) {
                        let os_raw = v["os"].as_str().unwrap_or("Windows 11").replace("Microsoft ", "");
                        let build = v["build"].as_str().unwrap_or("26200").to_string();
                        let cpu = v["cpu"].as_str().unwrap_or("AMD Ryzen 5").to_string();
                        let mem = v["mem"].as_u64().unwrap_or(16) as u32;
                        let cores = v["cores"].as_u64().unwrap_or(6) as u32;
                        let logical = v["logical"].as_u64().unwrap_or(12) as usize;
                        let gpus_str = v["gpus"].as_str().unwrap_or("");
                        let gpus: Vec<String> = gpus_str
                            .split(";;")
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();

                        return Ok(DeviceDiagnostics {
                            app_name,
                            app_version,
                            os_name: os_raw,
                            os_build: build.clone(),
                            kernel: build,
                            cpu_model: cpu,
                            architecture,
                            cpu_cores_physical: cores,
                            cpu_cores_logical: logical,
                            total_memory_gb: mem,
                            gpus,
                        });
                    }
                }
            }
        }

        Ok(DeviceDiagnostics {
            app_name,
            app_version,
            os_name: std::env::consts::OS.to_string(),
            os_build: "Unknown".into(),
            kernel: "Unknown".into(),
            cpu_model: std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "CPU".into()),
            architecture,
            cpu_cores_physical: 6,
            cpu_cores_logical: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(12),
            total_memory_gb: 16,
            gpus: vec!["NVIDIA GeForce RTX 2060".into()],
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn show_in_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
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
