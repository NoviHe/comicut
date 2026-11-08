use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use std::fs;
use std::path::PathBuf;

use crate::core::comic_slicer::heuristics::analyze_panel_metrics;
use crate::models::panel::{BoundingBox, Panel};

/// Menyambung atau memindai strip gambar Webtoon secara vertikal dan memotong pada baris gutter
/// (ruang kosong putih/hitam seragam di antara panel) agar tidak membelah gambar karakter.
pub fn slice_vertical_webtoon(image_paths: &[PathBuf]) -> Result<Vec<Panel>> {
    let out_dir = std::env::temp_dir().join("comicut_cache").join("sliced_panels");
    fs::create_dir_all(&out_dir)?;

    let mut panels: Vec<Panel> = Vec::new();
    let mut panel_counter = 0usize;

    for path in image_paths {
        let Ok(img) = image::open(path) else {
            continue;
        };
        let (width, height) = img.dimensions();
        if width < 120 || height < 120 {
            continue;
        }

        let segments = find_vertical_panel_segments(&img);
        for (y_start, y_end) in segments {
            let seg_h = y_end.saturating_sub(y_start);
            if seg_h < 180 {
                continue;
            }

            let cropped = img.crop_imm(0, y_start, width, seg_h);
            let out_path = out_dir.join(format!("webtoon_panel_{:03}.png", panel_counter + 1));
            let _ = cropped.save(&out_path);

            let metrics = analyze_panel_metrics(&cropped, panel_counter, image_paths.len() * 2);

            panels.push(Panel {
                id: format!("panel-{}", panel_counter + 1),
                index: panel_counter,
                image_path: out_path.to_string_lossy().to_string(),
                preview_data_url: None,
                bbox: BoundingBox {
                    x: 0,
                    y: y_start,
                    width,
                    height: seg_h,
                },
                aspect_ratio: metrics.aspect_ratio,
                edge_density: metrics.edge_density,
                contrast_score: metrics.contrast_score,
                heuristic_score: metrics.heuristic_score,
                ai_epic_score: None,
                role: metrics.suggested_role,
                is_vertical_pan: metrics.is_vertical_pan,
                selected: metrics.heuristic_score >= 0.55,
                caption: Some(format!(
                    "Webtoon Panel #{} ({}x{}px)",
                    panel_counter + 1,
                    width,
                    seg_h
                )),
            });

            panel_counter += 1;
        }
    }

    Ok(panels)
}

fn find_vertical_panel_segments(img: &DynamicImage) -> Vec<(u32, u32)> {
    let gray = img.to_luma8();
    let (width, height) = gray.dimensions();
    let mut segments = Vec::new();

    let mut in_panel = false;
    let mut seg_start = 0u32;
    let mut gutter_run = 0u32;

    for y in 0..height {
        let is_gutter = is_uniform_gutter_row(&gray, y, width);
        if !is_gutter {
            if !in_panel {
                in_panel = true;
                seg_start = y;
            }
            gutter_run = 0;
        } else if in_panel {
            gutter_run += 1;
            if gutter_run >= 18 && y.saturating_sub(seg_start) >= 220 {
                let seg_end = y.saturating_sub(gutter_run / 2);
                segments.push((seg_start, seg_end));
                in_panel = false;
                gutter_run = 0;
            }
        }
    }

    if in_panel && height.saturating_sub(seg_start) >= 180 {
        segments.push((seg_start, height));
    }

    if segments.is_empty() {
        segments.push((0, height));
    }

    segments
}

fn is_uniform_gutter_row(gray: &image::GrayImage, y: u32, width: u32) -> bool {
    let step = (width / 32).max(1);
    let first = gray.get_pixel(0, y)[0] as i16;
    let mut max_diff = 0i16;

    for x in (0..width).step_by(step as usize) {
        let val = gray.get_pixel(x, y)[0] as i16;
        let diff = (val - first).abs();
        if diff > max_diff {
            max_diff = diff;
        }
        if max_diff > 10 {
            return false;
        }
    }
    first <= 18 || first >= 238
}
