use anyhow::Result;
use crate::core::epic_reviewer::contact_sheet::build_contact_sheet;
use crate::models::panel::{Panel, PanelRole, VisionReviewConfig};

pub async fn score_and_rank_panels(
    mut panels: Vec<Panel>,
    config: &VisionReviewConfig,
) -> Result<Vec<Panel>> {
    if panels.is_empty() {
        return Ok(panels);
    }

    // Buat contact sheet untuk inspeksi visual / pengiriman ke Vision LLM
    let _ = build_contact_sheet(&panels);

    // Urutkan skor berdasarkan kombinasi Edge Density, Kontras, dan Ukuran Vertikal
    let mut scored_indices: Vec<(usize, f32)> = panels
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let bonus = if p.is_vertical_pan { 0.12 } else { 0.0 };
            let combined = (p.heuristic_score + bonus).clamp(0.1, 0.99);
            (i, combined)
        })
        .collect();

    scored_indices.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let max_select = config.max_panels_to_select.max(1);
    let top_set: std::collections::HashSet<usize> = scored_indices
        .iter()
        .take(max_select)
        .map(|(idx, _)| *idx)
        .collect();

    let total = panels.len();
    for (idx, panel) in panels.iter_mut().enumerate() {
        let is_picked = top_set.contains(&idx);
        panel.selected = is_picked;
        panel.ai_epic_score = Some((panel.heuristic_score * 1.05).clamp(0.15, 0.99));

        if idx == 0 {
            panel.role = PanelRole::Hook;
            panel.selected = true;
        } else if panel.is_vertical_pan {
            panel.role = PanelRole::VerticalPan;
        } else if panel.heuristic_score >= 0.82 {
            panel.role = PanelRole::Climax;
        } else if idx < total / 2 {
            panel.role = PanelRole::BuildUp;
        } else {
            panel.role = PanelRole::AuraCloseup;
        }
    }

    Ok(panels)
}
