use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb};
use crate::models::timeline::{EffectType, TimelineClip};

/// Menghasilkan 1 frame video 9:16 (misal 1080x1920) dengan efek Montagem
/// (Zoom Punch, Camera Shake, Vertical Pan Scroll, dan Impact Flash).
pub fn compose_vertical_frame(
    panel_img: &DynamicImage,
    clip: &TimelineClip,
    progress_0_to_1: f32,
    out_w: u32,
    out_h: u32,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let mut canvas = ImageBuffer::from_pixel(out_w, out_h, Rgb([10u8, 10u8, 14u8]));
    let (pw, ph) = panel_img.dimensions();
    if pw == 0 || ph == 0 {
        return canvas;
    }

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

    let target_w = ((out_w as f32) * zoom) as u32;
    let scale = target_w as f32 / pw as f32;
    let target_h = ((ph as f32) * scale) as u32;

    let resized = panel_img
        .resize_exact(
            target_w.max(1),
            target_h.max(1),
            image::imageops::FilterType::Triangle,
        )
        .to_rgb8();

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

    for cy in 0..out_h {
        let sy = cy as i32 - final_y_offset;
        if sy < 0 || sy >= resized.height() as i32 {
            continue;
        }
        for cx in 0..out_w {
            let sx = cx as i32 - x_offset;
            if sx < 0 || sx >= resized.width() as i32 {
                continue;
            }
            let p = resized.get_pixel(sx as u32, sy as u32);
            let r = p[0].saturating_add(flash_boost);
            let g = if red_only { p[1] } else { p[1].saturating_add(flash_boost) };
            let b = if red_only { p[2] } else { p[2].saturating_add(flash_boost) };
            canvas.put_pixel(cx, cy, Rgb([r, g, b]));
        }
    }

    canvas
}
