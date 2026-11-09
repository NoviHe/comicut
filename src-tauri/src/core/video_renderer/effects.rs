use std::collections::HashMap;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb, RgbImage};
use rayon::prelude::*;
use crate::models::timeline::{EffectType, TimelineClip};

#[derive(Clone, Copy)]
struct FrameParams {
    target_w: u32,
    target_h: u32,
    x_offset: i32,
    final_y_offset: i32,
    flash_boost: u8,
    red_only: bool,
}

fn frame_params(
    pw: u32,
    ph: u32,
    clip: &TimelineClip,
    progress_0_to_1: f32,
    out_w: u32,
    out_h: u32,
) -> FrameParams {
    // Eye-friendly motion (keep in sync with src/components/VideoPreview.tsx):
    // - impact eases out over the first 55% of the cut (squared), never a hard snap back;
    // - zoom punch capped at +8%, shake capped at 8px of 1080 and at ~7Hz (no strobing jitter);
    // - base zoom 1.03 on shake/flash so moving the frame never reveals the dark edge.
    let beat_decay = {
        let d = (1.0 - progress_0_to_1 / 0.55).clamp(0.0, 1.0);
        d * d
    };
    let t = progress_0_to_1 * clip.duration_sec;

    let zoom = match clip.effect {
        EffectType::ZoomPunch => 1.0 + (clip.zoom_scale - 1.0).clamp(0.0, 0.08) * beat_decay,
        EffectType::SlowPushIn => {
            let s = progress_0_to_1;
            1.0 + 0.05 * (s * s * (3.0 - 2.0 * s)) // smoothstep ease-in-out
        }
        _ => 1.03,
    };

    let (shake_x, shake_y) = if clip.effect == EffectType::CameraShake {
        let amp = clip.shake_amplitude.clamp(0.0, 8.0) * beat_decay;
        let w = 2.0 * std::f32::consts::PI * 7.0;
        ((((t * w).sin()) * amp) as i32, (((t * w * 0.8 + 1.3).cos()) * amp * 0.6) as i32)
    } else {
        (0, 0)
    };

    let target_w = (((out_w as f32) * zoom) as u32).max(1);
    let scale = target_w as f32 / pw as f32;
    let target_h = (((ph as f32) * scale) as u32).max(1);

    let y_offset: i32 = if clip.effect == EffectType::VerticalPanScroll && target_h > out_h {
        let scrollable = (target_h - out_h) as f32;
        -(scrollable * progress_0_to_1) as i32
    } else {
        (out_h as i32 - target_h as i32) / 2
    };

    let x_offset: i32 = (out_w as i32 - target_w as i32) / 2 + shake_x;
    let final_y_offset = y_offset + shake_y;

    // Soft flash: fades out smoothly and peaks at ~14% brightness (was a hard bright pop);
    // photosensitivity guideline: avoid large bright flashes.
    let flash_boost: u8 = match clip.effect {
        EffectType::ImpactFlashWhite | EffectType::ImpactFlashRed => (36.0 * beat_decay) as u8,
        _ => 0,
    };
    let red_only = clip.effect == EffectType::ImpactFlashRed;

    FrameParams {
        target_w,
        target_h,
        x_offset,
        final_y_offset,
        flash_boost,
        red_only,
    }
}

/// Blit a pre-resized panel into a vertical 9:16 RGB24 frame buffer using row-slice operations.
fn blit_resized_to_raw(resized: &RgbImage, p: &FrameParams, out_w: u32, out_h: u32) -> Vec<u8> {
    let rw = resized.width() as i32;
    let rh = resized.height() as i32;
    let ow = out_w as i32;
    let oh = out_h as i32;

    let cy_start = p.final_y_offset.max(0).min(oh);
    let cy_end = (p.final_y_offset + rh).max(0).min(oh);
    let cx_start = p.x_offset.max(0).min(ow);
    let cx_end = (p.x_offset + rw).max(0).min(ow);

    let total_bytes = (out_w * out_h * 3) as usize;
    let covers_all = cy_start == 0 && cy_end == oh && cx_start == 0 && cx_end == ow;
    let mut raw = vec![0u8; total_bytes];
    if !covers_all {
        for px in raw.chunks_exact_mut(3) {
            px[0] = 10;
            px[1] = 10;
            px[2] = 14;
        }
    }

    if cy_start >= cy_end || cx_start >= cx_end {
        return raw;
    }

    let sx_start = (cx_start - p.x_offset) as usize * 3;
    let copy_bytes = (cx_end - cx_start) as usize * 3;
    let dst_x_start = cx_start as usize * 3;
    let src_stride = rw as usize * 3;
    let dst_stride = ow as usize * 3;
    let src_raw = resized.as_raw();

    for cy in cy_start..cy_end {
        let sy = (cy - p.final_y_offset) as usize;
        let src_off = sy * src_stride + sx_start;
        let dst_off = cy as usize * dst_stride + dst_x_start;
        let src_row = &src_raw[src_off..src_off + copy_bytes];
        let dst_row = &mut raw[dst_off..dst_off + copy_bytes];

        if p.flash_boost == 0 {
            dst_row.copy_from_slice(src_row);
        } else if !p.red_only {
            let boost = p.flash_boost;
            for (d, &s) in dst_row.iter_mut().zip(src_row.iter()) {
                *d = s.saturating_add(boost);
            }
        } else {
            let boost = p.flash_boost;
            for (d, s) in dst_row.chunks_exact_mut(3).zip(src_row.chunks_exact(3)) {
                d[0] = s[0].saturating_add(boost);
                d[1] = s[1];
                d[2] = s[2];
            }
        }
    }

    raw
}

/// Menghasilkan 1 frame video 9:16 (misal 1080x1920) dengan efek Montagem
/// (Zoom Punch, Camera Shake, Vertical Pan Scroll, dan Impact Flash).
pub fn compose_vertical_frame(
    panel_img: &DynamicImage,
    clip: &TimelineClip,
    progress_0_to_1: f32,
    out_w: u32,
    out_h: u32,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let (pw, ph) = panel_img.dimensions();
    if pw == 0 || ph == 0 {
        return ImageBuffer::from_pixel(out_w, out_h, Rgb([10u8, 10u8, 14u8]));
    }
    let p = frame_params(pw, ph, clip, progress_0_to_1, out_w, out_h);
    let resized = panel_img
        .resize_exact(p.target_w, p.target_h, image::imageops::FilterType::Triangle)
        .to_rgb8();
    let raw = blit_resized_to_raw(&resized, &p, out_w, out_h);
    ImageBuffer::from_raw(out_w, out_h, raw).expect("valid frame dimensions")
}

/// Render all frames of a single clip in parallel:
/// 1. Deduplicates `(target_w, target_h)` so constant-zoom clips only resize once (instead of 60x/sec).
/// 2. Computes all unique resizes in parallel across CPU cores via `rayon`.
/// 3. Streams composed RGB24 frames in chunks to `on_frame` in exact frame order.
pub fn render_clip_frames<F>(
    panel_rgb: &RgbImage,
    clip: &TimelineClip,
    total_frames: u32,
    out_w: u32,
    out_h: u32,
    mut on_frame: F,
) -> std::io::Result<()>
where
    F: FnMut(Vec<u8>) -> std::io::Result<()>,
{
    let (pw, ph) = panel_rgb.dimensions();
    if pw == 0 || ph == 0 {
        let blank = ImageBuffer::from_pixel(out_w, out_h, Rgb([10u8, 10u8, 14u8])).into_raw();
        for _ in 0..total_frames {
            on_frame(blank.clone())?;
        }
        return Ok(());
    }

    let params: Vec<FrameParams> = (0..total_frames)
        .map(|f| {
            let progress = f as f32 / total_frames as f32;
            frame_params(pw, ph, clip, progress, out_w, out_h)
        })
        .collect();

    // Collect unique (target_w, target_h) sizes needed by this clip and resize each once in parallel.
    let mut unique_dims: Vec<(u32, u32)> = params.iter().map(|p| (p.target_w, p.target_h)).collect();
    unique_dims.sort_unstable();
    unique_dims.dedup();

    let resized_cache: HashMap<(u32, u32), RgbImage> = unique_dims
        .into_par_iter()
        .map(|(tw, th)| {
            let img = image::imageops::resize(
                panel_rgb,
                tw,
                th,
                image::imageops::FilterType::Triangle,
            );
            ((tw, th), img)
        })
        .collect();

    // Compose frames in parallel batches of 16 to bound memory (~99 MB @ 1080p) while keeping all cores busy.
    for chunk in params.chunks(16) {
        let frames: Vec<Vec<u8>> = chunk
            .par_iter()
            .map(|p| {
                let resized = &resized_cache[&(p.target_w, p.target_h)];
                blit_resized_to_raw(resized, p, out_w, out_h)
            })
            .collect();
        for frame in frames {
            on_frame(frame)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_blit_matches_pixel_loop() {
        let mut src = RgbImage::new(240, 360);
        for (x, y, px) in src.enumerate_pixels_mut() {
            *px = Rgb([(x % 255) as u8, (y % 255) as u8, ((x + y) % 255) as u8]);
        }
        let dyn_img = DynamicImage::ImageRgb8(src.clone());
        for effect in [
            EffectType::ZoomPunch,
            EffectType::CameraShake,
            EffectType::VerticalPanScroll,
            EffectType::ImpactFlashWhite,
            EffectType::ImpactFlashRed,
            EffectType::SlowPushIn,
        ] {
            let clip = TimelineClip {
                id: "c1".into(),
                panel_id: "p1".into(),
                panel_index: 0,
                role: crate::models::panel::PanelRole::Climax,
                start_sec: 0.0,
                end_sec: 0.5,
                duration_sec: 0.5,
                effect,
                beat_intensity: 0.9,
                zoom_scale: 1.08,
                shake_amplitude: 6.0,
            };
            let mut batch = Vec::new();
            render_clip_frames(&src, &clip, 4, 180, 320, |f| {
                batch.push(f);
                Ok(())
            })
            .unwrap();
            for (i, raw) in batch.into_iter().enumerate() {
                let single = compose_vertical_frame(&dyn_img, &clip, i as f32 / 4.0, 180, 320);
                assert_eq!(raw, single.into_raw());
            }
        }
    }
}
