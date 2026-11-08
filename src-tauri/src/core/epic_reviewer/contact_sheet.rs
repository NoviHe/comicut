use anyhow::Result;
use image::{ImageBuffer, Rgb};
use std::path::PathBuf;

use crate::models::panel::Panel;

/// Membuat gambar Contact Sheet (grid kumpulan kandidat panel) untuk menghemat token Vision LLM.
pub fn build_contact_sheet(panels: &[Panel]) -> Result<PathBuf> {
    let cols = 4u32;
    let cell_w = 240u32;
    let cell_h = 320u32;
    let rows = ((panels.len() as u32 + cols - 1) / cols).max(1);

    let mut sheet = ImageBuffer::from_pixel(cols * cell_w, rows * cell_h, Rgb([18u8, 18u8, 22u8]));

    for (idx, panel) in panels.iter().enumerate() {
        let Ok(img) = image::open(&panel.image_path) else {
            continue;
        };
        let thumb = img.resize_exact(cell_w - 8, cell_h - 8, image::imageops::FilterType::Triangle);
        let rgb = thumb.to_rgb8();

        let col = (idx as u32) % cols;
        let row = (idx as u32) / cols;
        let ox = col * cell_w + 4;
        let oy = row * cell_h + 4;

        for y in 0..rgb.height() {
            for x in 0..rgb.width() {
                sheet.put_pixel(ox + x, oy + y, *rgb.get_pixel(x, y));
            }
        }
    }

    let out_path = std::env::temp_dir()
        .join("comicut_cache")
        .join("contact_sheet.jpg");
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    sheet.save(&out_path)?;
    Ok(out_path)
}
