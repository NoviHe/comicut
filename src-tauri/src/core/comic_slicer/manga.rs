use anyhow::Result;
use image::GenericImageView;
use std::fs;
use std::path::PathBuf;

use crate::core::comic_slicer::heuristics::analyze_panel_metrics;
use crate::models::panel::{BoundingBox, Panel};

/// Memotong halaman Manga (RTL Jepang atau LTR Komik) menjadi panel-panel terurut.
pub fn slice_manga_pages(image_paths: &[PathBuf], right_to_left: bool) -> Result<Vec<Panel>> {
    let out_dir = std::env::temp_dir().join("comicut_cache").join("sliced_manga");
    fs::create_dir_all(&out_dir)?;

    let mut panels = Vec::new();
    let mut panel_counter = 0usize;

    for path in image_paths {
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
            let out_path = out_dir.join(format!("manga_panel_{:03}.png", panel_counter + 1));
            let _ = cropped.save(&out_path);

            let metrics = analyze_panel_metrics(&cropped, panel_counter, image_paths.len() * 2);
            let direction_tag = if right_to_left { "RTL" } else { "LTR" };

            panels.push(Panel {
                id: format!("panel-{}", panel_counter + 1),
                index: panel_counter,
                image_path: out_path.to_string_lossy().to_string(),
                preview_data_url: None,
                bbox: BoundingBox {
                    x: 0,
                    y: y0,
                    width,
                    height: h,
                },
                aspect_ratio: metrics.aspect_ratio,
                edge_density: metrics.edge_density,
                contrast_score: metrics.contrast_score,
                heuristic_score: metrics.heuristic_score,
                ai_epic_score: None,
                role: metrics.suggested_role,
                is_vertical_pan: false,
                selected: metrics.heuristic_score >= 0.50,
                caption: Some(format!(
                    "Manga {} Panel #{} ({}x{}px)",
                    direction_tag,
                    panel_counter + 1,
                    width,
                    h
                )),
            });

            panel_counter += 1;
        }
    }

    Ok(panels)
}
