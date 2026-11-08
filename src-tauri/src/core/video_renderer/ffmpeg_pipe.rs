use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::core::video_renderer::effects::compose_vertical_frame;
use crate::core::video_renderer::encoder::{self, Encoder};
use crate::models::audio::BeatMap;
use crate::models::panel::Panel;
use crate::models::timeline::{RenderConfig, TimelineClip};

/// Merender frame demi frame di Rust lalu mengirimkannya via stdin pipe ke FFmpeg
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

    let mut loaded_images = HashMap::new();
    for p in panels {
        if let Ok(img) = image::open(&p.image_path) {
            loaded_images.insert(p.id.clone(), img);
        }
    }

    // Cek ketersediaan binary ffmpeg di sistem
    let ffmpeg_check = Command::new("ffmpeg").arg("-version").output();
    if ffmpeg_check.is_err() {
        return Err(anyhow!(
            "Binary FFmpeg belum ditemukan di PATH sistem. Pasang FFmpeg (atau taruh di src-tauri/binaries/) untuk mengekspor file MP4 ke {}",
            config.output_path
        ));
    }

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
        "[render] {}x{} @{}fps clips={} audio={} out={}",
        width, height, fps, timeline.len(),
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
            Ok(()) => {
                println!("[render] done -> {} ({})", config.output_path, enc.label());
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

#[allow(clippy::too_many_arguments)]
fn encode_with(
    enc: Encoder,
    loaded_images: &HashMap<String, image::DynamicImage>,
    beat_map: &BeatMap,
    timeline: &[TimelineClip],
    config: &RenderConfig,
    width: u32,
    height: u32,
    fps: u32,
    has_audio: bool,
) -> Result<()> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-hide_banner", "-f", "rawvideo", "-pixel_format", "rgb24", "-video_size"])
        .arg(format!("{}x{}", width, height))
        .args(["-framerate"])
        .arg(fps.to_string())
        .args(["-i", "-"]);
    if has_audio {
        cmd.arg("-i").arg(&beat_map.audio_path).args(["-map", "0:v", "-map", "1:a", "-c:a", "aac", "-b:a", "192k", "-shortest"]);
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

    let mut write_err: Option<std::io::Error> = None;
    if let Some(mut stdin) = child.stdin.take() {
        let default_img = image::DynamicImage::new_rgb8(800, 1200);
        'clips: for clip in timeline {
            let panel_img = loaded_images.get(&clip.panel_id).unwrap_or(&default_img);
            let total_frames = ((clip.duration_sec * fps as f32).round() as u32).max(1);
            for f in 0..total_frames {
                let progress = f as f32 / total_frames as f32;
                let frame = compose_vertical_frame(panel_img, clip, progress, width, height);
                if let Err(e) = stdin.write_all(frame.as_raw()) {
                    write_err = Some(e);
                    break 'clips;
                }
            }
        }
    } // stdin dropped -> EOF for ffmpeg

    let status = child.wait()?;
    let log = stderr_thread.join().unwrap_or_default();
    if write_err.is_some() || !status.success() {
        let tail: Vec<&str> = log.lines().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
        println!("[render] ffmpeg failed: {}", log);
        return Err(anyhow!("FFmpeg gagal ({}): {}", status, tail.join(" | ")));
    }

    Ok(())
}
