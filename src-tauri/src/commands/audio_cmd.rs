use crate::core::audio_beat::analyze_track;
use crate::models::audio::BeatMap;

#[tauri::command]
pub async fn analyze_audio_beats(audio_path: String) -> Result<BeatMap, String> {
    // Copy into the cache so the webview (asset protocol scope = temp/comicut_cache) can play it in the preview.
    let src = std::path::PathBuf::from(&audio_path);
    let dir = std::env::temp_dir().join("comicut_cache").join("audio");
    let playable = if src.starts_with(&dir) {
        src
    } else {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let dst = dir.join(src.file_name().ok_or("Path audio tidak valid")?);
        std::fs::copy(&src, &dst).map_err(|e| format!("Gagal membaca file audio {}: {}", audio_path, e))?;
        dst
    };
    let p = playable.to_string_lossy().to_string();
    tokio::task::spawn_blocking(move || analyze_track(&p).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

/// Download audio via local `yt-dlp` (+ffmpeg) from a link (YouTube/TikTok/SoundCloud/...)
/// or, for plain text, the top YouTube search hit. Returns the mp3 path.
#[tauri::command]
pub async fn download_audio(source: String) -> Result<String, String> {
    let source = source.trim();
    if source.is_empty() {
        return Err("Link atau kata kunci kosong.".into());
    }
    // Trust boundary: never let user text become a yt-dlp option.
    if source.starts_with('-') {
        return Err("Input tidak valid.".into());
    }
    let target = if source.starts_with("http://") || source.starts_with("https://") {
        source.to_string()
    } else {
        format!("ytsearch1:{}", source)
    };

    let dir = std::env::temp_dir().join("comicut_cache").join("audio");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    println!("[audio] yt-dlp target={}", target);
    let mut cmd = tokio::process::Command::new(super::deps_cmd::tool("yt-dlp"));
    cmd.args([
        "-x",
        "--audio-format",
        "mp3",
        "--no-playlist",
        // YouTube needs a JS runtime for signature solving; node is the common one (ignored if absent).
        "--js-runtimes",
        "node",
        "--match-filter",
        "duration<=600",
        "--no-simulate",
        "--print",
        "after_move:filepath",
        "-o",
    ])
    .arg(dir.join("%(id)s.%(ext)s"))
    .arg("--")
    .arg(&target);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

    let out = cmd.output().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "yt-dlp belum terpasang. Klik Install di panel Status Dependensi.".to_string()
        } else {
            e.to_string()
        }
    })?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        println!("[audio] yt-dlp failed: {}", err);
        let last = err.lines().rev().find(|l| l.contains("ERROR")).unwrap_or("unknown error");
        let hint = if err.contains("403") || err.contains("Sign in") || err.contains("nsig") {
            " — update yt-dlp lewat panel Status Dependensi"
        } else {
            ""
        };
        return Err(format!("yt-dlp gagal: {}{}", last, hint));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let path = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .last()
        .ok_or("Tidak ada audio yang diunduh (mungkin melebihi 10 menit atau tidak ditemukan).")?
        .to_string();
    println!("[audio] downloaded {}", path);
    Ok(path)
}
