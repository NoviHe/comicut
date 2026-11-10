use anyhow::{anyhow, Result};
use image::RgbImage;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Instant;

use crate::commands::deps_cmd::tool;
use crate::core::video_renderer::effects::render_clip_frames;
use crate::core::video_renderer::encoder::{self, Encoder};
use crate::models::audio::BeatMap;
use crate::models::panel::Panel;
use crate::models::timeline::{RenderConfig, TimelineClip};

/// Merender frame demi frame di Rust secara paralel lalu mengirimkannya via stdin pipe ke FFmpeg
/// untuk menghasilkan video MP4 9:16 (1080x1920) 60fps lengkap dengan musik Montagem.
pub fn render_timeline_to_mp4(
    panels: &[Panel],
    beat_map: &BeatMap,
    timeline: &[TimelineClip],
    config: &RenderConfig,
) -> Result<String> {
    if timeline.is_empty() {
        return Err(anyhow!("Timeline masih kosong. Sinkronkan panel ke beat terlebih dahulu."));
    }

    let t0 = Instant::now();

    // Cek ketersediaan binary ffmpeg di sistem
    let ffmpeg_check = Command::new(tool("ffmpeg")).arg("-version").output();
    if ffmpeg_check.is_err() {
        return Err(anyhow!(
            "FFmpeg tidak ditemukan (bawaan aplikasi hilang?). Pasang ulang lewat panel Status Dependensi untuk mengekspor MP4 ke {}",
            config.output_path
        ));
    }

    // Hanya muat gambar panel yang benar-benar dipakai di timeline, secara paralel.
    let used_ids: HashSet<&str> = timeline.iter().map(|c| c.panel_id.as_str()).collect();
    let loaded_images: HashMap<String, RgbImage> = panels
        .par_iter()
        .filter(|p| used_ids.contains(p.id.as_str()))
        .filter_map(|p| image::open(&p.image_path).ok().map(|img| (p.id.clone(), img.into_rgb8())))
        .collect();

    let fps = config.fps.max(24);
    let width = config.width.max(720);
    let height = config.height.max(1280);

    // ffmpeg does not create folders: a missing output dir kills it instantly (os error 232 on our pipe).
    if let Some(parent) = std::path::Path::new(&config.output_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Tidak bisa membuat folder output {}: {}", parent.display(), e))?;
        }
    }

    // Sample/preset beat maps have no real file (sample://...); render silent instead of failing.
    let has_audio = !beat_map.audio_path.starts_with("sample://")
        && std::path::Path::new(&beat_map.audio_path).is_file();
    println!(
        "[render] {}x{} @{}fps clips={} panels_loaded={}/{} audio={} out={}",
        width,
        height,
        fps,
        timeline.len(),
        loaded_images.len(),
        panels.len(),
        if has_audio { beat_map.audio_path.as_str() } else { "(none)" },
        config.output_path
    );

    // GPU encoder when it really works on this machine, CPU x264 otherwise (also if the GPU run fails midway).
    let gpu = encoder::detect().encoder;
    let mut tried = vec![gpu];
    if gpu != Encoder::X264 {
        tried.push(Encoder::X264);
    }
    let mut last_err = anyhow!("render gagal");
    for enc in tried {
        println!("[render] encoder: {}", enc.label());
        match encode_with(enc, &loaded_images, beat_map, timeline, config, width, height, fps, has_audio) {
            Ok(frames) => {
                let secs = t0.elapsed().as_secs_f32().max(0.001);
                println!(
                    "[render] done {} frames in {:.2}s ({:.0} fps) -> {} ({})",
                    frames,
                    secs,
                    frames as f32 / secs,
                    config.output_path,
                    enc.label()
                );
                return Ok(config.output_path.clone());
            }
            Err(e) => {
                println!("[render] {} failed: {}", enc.label(), e);
                last_err = e;
            }
        }
    }
    Err(last_err)
}

struct TempFileGuard(std::path::PathBuf);
impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn resolve_font_path(font_name: &str) -> Option<String> {
    let win_fonts = std::path::Path::new("C:\\Windows\\Fonts");
    let file = match font_name.to_lowercase().as_str() {
        "impact" => "impact.ttf",
        "arial black" | "ariblk" => "ariblk.ttf",
        "arial bold" | "arial" => "arialbd.ttf",
        "verdana bold" | "verdana" => "verdanab.ttf",
        "trebuchet ms bold" | "trebuchet ms" | "trebuc" => "trebucbd.ttf",
        "segoe ui black" | "seguibl" => "seguibl.ttf",
        "segoe ui bold" | "segoe ui" => "segoeuib.ttf",
        "tahoma bold" | "tahoma" => "tahomabd.ttf",
        _ => "impact.ttf",
    };
    let path = win_fonts.join(file);
    if path.is_file() {
        let s = path.to_string_lossy().replace('\\', "/");
        let escaped = s.replace(':', "\\:");
        Some(escaped)
    } else {
        None
    }
}

fn build_watermark_filter(
    wm: &crate::models::timeline::WatermarkConfig,
) -> Option<(String, TempFileGuard)> {
    if !wm.enabled || wm.text.trim().is_empty() {
        return None;
    }

    let id = format!(
        "comicut_wm_{}_{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let temp_file = std::env::temp_dir().join(id);
    if std::fs::write(&temp_file, wm.text.trim()).is_err() {
        return None;
    }

    let font_file_arg = match resolve_font_path(&wm.font) {
        Some(escaped) => format!(":fontfile='{}'", escaped),
        None => format!(":font='{}'", wm.font.replace('\'', "")),
    };

    let tf_escaped = temp_file
        .to_string_lossy()
        .replace('\\', "/")
        .replace(':', "\\:");
    let font_size = wm.font_size.unwrap_or(48).max(12).min(180);
    let opacity = wm.opacity.clamp(0.05, 1.0);
    let pos_x = (wm.position_x / 100.0).clamp(0.0, 1.0);
    let pos_y = (wm.position_y / 100.0).clamp(0.0, 1.0);

    let filter = format!(
        "drawtext=textfile='{}':fontsize={}:fontcolor=white@{:.2}:bordercolor=black@{:.2}:borderw=3:x=(w-text_w)*{:.4}:y=(h-text_h)*{:.4}{}",
        tf_escaped,
        font_size,
        opacity,
        opacity,
        pos_x,
        pos_y,
        font_file_arg
    );

    Some((filter, TempFileGuard(temp_file)))
}

#[allow(clippy::too_many_arguments)]
fn encode_with(
    enc: Encoder,
    loaded_images: &HashMap<String, RgbImage>,
    beat_map: &BeatMap,
    timeline: &[TimelineClip],
    config: &RenderConfig,
    width: u32,
    height: u32,
    fps: u32,
    has_audio: bool,
) -> Result<u64> {
    let (wm_filter, _wm_guard) = config
        .watermark
        .as_ref()
        .and_then(build_watermark_filter)
        .map(|(f, g)| (Some(f), Some(g)))
        .unwrap_or((None, None));

    let mut cmd = Command::new(tool("ffmpeg"));
    cmd.args(["-y", "-hide_banner", "-f", "rawvideo", "-pixel_format", "rgb24", "-video_size"])
        .arg(format!("{}x{}", width, height))
        .args(["-framerate"])
        .arg(fps.to_string())
        .args(["-i", "-"]);
    if has_audio {
        cmd.arg("-i").arg(&beat_map.audio_path).args(["-map", "0:v", "-map", "1:a", "-c:a", "aac", "-b:a", "192k", "-shortest"]);
    }
    if let Some(ref filter) = wm_filter {
        cmd.args(["-vf", filter]);
    }
    cmd.args(["-pix_fmt", "yuv420p"])
        .args(enc.args())
        .args(["-movflags", "+faststart"])
        .arg(&config.output_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = cmd.spawn()?;

    // Drain stderr continuously (full pipe would deadlock ffmpeg) and keep it for error reports.
    let mut stderr = child.stderr.take().ok_or_else(|| anyhow!("stderr ffmpeg tidak tersedia"))?;
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr, &mut buf);
        buf
    });

    // Dedicated writer thread so CPU frame composition overlaps with FFmpeg pipe I/O.
    let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("stdin ffmpeg tidak tersedia"))?;
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(16);
    let writer_thread = std::thread::spawn(move || -> std::io::Result<()> {
        for frame in rx {
            stdin.write_all(&frame)?;
        }
        stdin.flush()
    });

    let default_img = RgbImage::new(800, 1200);
    let mut total_written: u64 = 0;
    for clip in timeline {
        let panel_img = loaded_images.get(&clip.panel_id).unwrap_or(&default_img);
        let total_frames = ((clip.duration_sec * fps as f32).round() as u32).max(1);
        let send_res = render_clip_frames(panel_img, clip, total_frames, width, height, |frame| {
            total_written += 1;
            tx.send(frame)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "ffmpeg pipe closed"))
        });
        if send_res.is_err() {
            break;
        }
    }
    drop(tx); // Close channel -> writer finishes and drops stdin -> EOF for ffmpeg

    let write_res = writer_thread.join().unwrap_or_else(|_| {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "writer thread panicked"))
    });
    let status = child.wait()?;
    let log = stderr_thread.join().unwrap_or_default();
    if write_res.is_err() || !status.success() {
        let tail: Vec<&str> = log.lines().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
        println!("[render] ffmpeg failed: {}", log);
        return Err(anyhow!("FFmpeg gagal ({}): {}", status, tail.join(" | ")));
    }

    Ok(total_written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::timeline::WatermarkConfig;

    #[test]
    fn test_watermark_filter_generation() {
        let wm = WatermarkConfig {
            enabled: true,
            text: "@ComiCutTest".into(),
            font: "Impact".into(),
            font_size: Some(52),
            position_x: 50.0,
            position_y: 82.0,
            opacity: 0.7,
        };

        let res = build_watermark_filter(&wm);
        assert!(res.is_some());
        let (filter_str, guard) = res.unwrap();
        assert!(filter_str.contains("fontsize=52"));
        assert!(filter_str.contains("fontcolor=white@0.70"));
        assert!(filter_str.contains("x=(w-text_w)*0.5000"));
        assert!(filter_str.contains("y=(h-text_h)*0.8200"));
        assert!(guard.0.is_file());
        drop(guard);
    }

    #[test]
    fn test_watermark_filter_disabled_or_empty() {
        let wm_disabled = WatermarkConfig {
            enabled: false,
            text: "@ComiCut".into(),
            font: "Impact".into(),
            font_size: None,
            position_x: 50.0,
            position_y: 82.0,
            opacity: 0.7,
        };
        assert!(build_watermark_filter(&wm_disabled).is_none());

        let wm_empty = WatermarkConfig {
            enabled: true,
            text: "   ".into(),
            font: "Impact".into(),
            font_size: None,
            position_x: 50.0,
            position_y: 82.0,
            opacity: 0.7,
        };
        assert!(build_watermark_filter(&wm_empty).is_none());
    }
}
