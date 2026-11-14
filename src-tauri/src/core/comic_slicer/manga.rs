use anyhow::Result;
use image::GenericImageView;
use std::fs;
use std::path::PathBuf;

use crate::core::comic_slicer::style::apply_panel_style;
use crate::models::panel::{BoundingBox, Panel, PanelStyle};

/// Memotong halaman Manga (RTL Jepang atau LTR Komik) menjadi panel-panel terurut.
pub fn slice_manga_pages(
    image_paths: &[PathBuf],
    right_to_left: bool,
    credit: &[bool],
    style: PanelStyle,
) -> Result<Vec<Panel>> {
    let out_dir = std::env::temp_dir().join("comicut_cache").join("sliced_manga");
    fs::create_dir_all(&out_dir)?;

    let mut panels = Vec::new();
    let mut panel_counter = 0usize;

    for (page_idx, path) in image_paths.iter().enumerate() {
        if credit.get(page_idx).copied().unwrap_or(false) {
            continue; // scanlator credit / ad page
        }
        let Ok(img) = image::open(path) else {
            continue;
        };
        let (width, height) = img.dimensions();
        if width < 150 || height < 150 {
            continue;
        }

        // Bagi halaman manga menjadi baris panel atas dan bawah (atau panel utuh jika spread)
        let row_heights = if height > width {
            vec![(0, height / 2), (height / 2, height)]
        } else {
            vec![(0, height)]
        };

        for (y0, y1) in row_heights {
            let h = y1.saturating_sub(y0);
            let cropped = img.crop_imm(0, y0, width, h);
            let orig_bbox = BoundingBox {
                x: 0,
                y: y0,
                width,
                height: h,
            };
            let Some(styled) = apply_panel_style(
                cropped,
                orig_bbox,
                style,
                panel_counter,
                image_paths.len() * 2,
                0.50,
            ) else {
                println!("[slicer] skipped blank/text-only segment (page {}, y {}..{})", page_idx + 1, y0, y1);
                continue;
            };
            let out_path = out_dir.join(format!("manga_panel_{:03}.png", panel_counter + 1));
            let _ = styled.image.save(&out_path);
            let direction_tag = if right_to_left { "RTL" } else { "LTR" };

            panels.push(Panel {
                id: format!("panel-{}", panel_counter + 1),
                index: panel_counter,
                image_path: out_path.to_string_lossy().to_string(),
                preview_data_url: None,
                bbox: styled.bbox.clone(),
                aspect_ratio: styled.metrics.aspect_ratio,
                edge_density: styled.metrics.edge_density,
                contrast_score: styled.metrics.contrast_score,
                heuristic_score: styled.metrics.heuristic_score,
                ai_epic_score: None,
                role: styled.metrics.suggested_role,
                is_vertical_pan: false,
                selected: styled.selected,
                caption: Some(format!(
                    "Manga {} Panel #{} ({}x{}px){}",
                    direction_tag,
                    panel_counter + 1,
                    styled.bbox.width,
                    styled.bbox.height,
                    styled.caption_suffix
                )),
            });

            panel_counter += 1;
        }
    }

    Ok(panels)
}
