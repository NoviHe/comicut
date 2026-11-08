use image::{DynamicImage, GenericImageView};
use crate::models::panel::PanelRole;

pub struct VisualMetrics {
    pub aspect_ratio: f32,
    pub edge_density: f32,
    pub contrast_score: f32,
    pub heuristic_score: f32,
    pub suggested_role: PanelRole,
    pub is_vertical_pan: bool,
}

pub fn analyze_panel_metrics(
    img: &DynamicImage,
    panel_index: usize,
    total_estimated: usize,
) -> VisualMetrics {
    let (w, h) = img.dimensions();
    let width = w.max(1) as f32;
    let height = h.max(1) as f32;
    let aspect_ratio = width / height;
    let is_vertical_pan = height / width >= 1.85;

    let gray = img.to_luma8();
    let step = 4usize;
    let mut edge_hits = 0u64;
    let mut samples = 0u64;
    let mut min_luma = 255u8;
    let mut max_luma = 0u8;

    for y in (1..h.saturating_sub(1)).step_by(step) {
        for x in (1..w.saturating_sub(1)).step_by(step) {
            let c = gray.get_pixel(x, y)[0];
            let r = gray.get_pixel(x + 1, y)[0];
            let b = gray.get_pixel(x, y + 1)[0];

            min_luma = min_luma.min(c);
            max_luma = max_luma.max(c);

            let dx = (c as i16 - r as i16).unsigned_abs();
            let dy = (c as i16 - b as i16).unsigned_abs();
            if dx + dy > 36 {
                edge_hits += 1;
            }
            samples += 1;
        }
    }

    let edge_density = if samples > 0 {
        ((edge_hits as f32 / samples as f32) * 2.2).clamp(0.1, 1.0)
    } else {
        0.5
    };

    let contrast_score = ((max_luma.saturating_sub(min_luma)) as f32 / 255.0).clamp(0.2, 1.0);
    let size_bonus = if is_vertical_pan { 0.18 } else { 0.08 };
    let heuristic_score =
        (edge_density * 0.45 + contrast_score * 0.37 + size_bonus).clamp(0.15, 0.99);

    let suggested_role = if panel_index == 0 {
        PanelRole::Hook
    } else if is_vertical_pan {
        PanelRole::VerticalPan
    } else if heuristic_score >= 0.80 {
        PanelRole::Climax
    } else if panel_index < total_estimated / 2 {
        PanelRole::BuildUp
    } else {
        PanelRole::AuraCloseup
    };

    VisualMetrics {
        aspect_ratio,
        edge_density,
        contrast_score,
        heuristic_score,
        suggested_role,
        is_vertical_pan,
    }
}
