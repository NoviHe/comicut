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

    // Decay kurva eksponensial untuk efek hentakan bass di awal klip
    let beat_decay = (1.0 - progress_0_to_1 * 3.2).clamp(0.0, 1.0);

    let zoom = match clip.effect {
        EffectType::ZoomPunch => 1.0 + (clip.zoom_scale - 1.0) * beat_decay,
        EffectType::SlowPushIn => 1.0 + 0.08 * progress_0_to_1,
        _ => 1.04,
    };

    let (shake_x, shake_y) = if clip.effect == EffectType::CameraShake && beat_decay > 0.0 {
        let sx = ((progress_0_to_1 * 90.0).sin() * clip.shake_amplitude * beat_decay) as i32;
        let sy = ((progress_0_to_1 * 120.0).cos() * clip.shake_amplitude * beat_decay) as i32;
        (sx, sy)
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

    let flash_boost: u8 = if clip.effect == EffectType::ImpactFlashWhite && beat_decay > 0.55 {
        ((beat_decay - 0.55) * 140.0) as u8
    } else {
        0
    };

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
            let g = p[1].saturating_add(flash_boost);
            let b = p[2].saturating_add(flash_boost);
            canvas.put_pixel(cx, cy, Rgb([r, g, b]));
        }
    }

    canvas
}
