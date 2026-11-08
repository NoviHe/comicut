use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::core::video_renderer::effects::compose_vertical_frame;
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

    let mut child = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "rawvideo",
            "-pixel_format",
            "rgb24",
            "-video_size",
            &format!("{}x{}", width, height),
            "-framerate",
            &fps.to_string(),
            "-i",
            "-",
            "-i",
            &beat_map.audio_path,
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-preset",
            "veryfast",
            "-shortest",
            &config.output_path,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        let default_img = image::DynamicImage::new_rgb8(800, 1200);

        for clip in timeline {
            let panel_img = loaded_images.get(&clip.panel_id).unwrap_or(&default_img);
            let total_frames = ((clip.duration_sec * fps as f32).round() as u32).max(1);

            for f in 0..total_frames {
                let progress = f as f32 / total_frames as f32;
                let frame = compose_vertical_frame(panel_img, clip, progress, width, height);
                stdin.write_all(frame.as_raw())?;
            }
        }
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("FFmpeg selesai dengan kode status non-zero"));
    }

    Ok(config.output_path.clone())
}
