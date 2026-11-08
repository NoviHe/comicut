use image::{DynamicImage, GenericImageView};
use crate::models::panel::PanelRole;

pub struct VisualMetrics {
    pub aspect_ratio: f32,
    pub edge_density: f32,
    pub contrast_score: f32,
    pub heuristic_score: f32,
    pub suggested_role: PanelRole,
    pub is_vertical_pan: bool,
    /// Almost no drawn content (blank gutter, plain text / info banner): not a real panel.
    pub is_blank: bool,
}

/// Local "epic-ness" score. Works on a <=320px-wide copy: Sobel edge density,
/// robust contrast (p5..p95, not min/max which is ~always 0..255), luma entropy,
/// colorfulness, and a penalty for flat/blank panels (dialogue, empty frames).
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

    let small = if w > 320 { img.resize(320, 4000, image::imageops::FilterType::Triangle) } else { img.clone() };
    let gray = small.to_luma8();
    let rgb = small.to_rgb8();
    let (sw, sh) = gray.dimensions();

    let mut hist = [0u32; 256];
    let mut edge_hits = 0u64;
    let mut samples = 0u64;
    if sw >= 3 && sh >= 3 {
        let p = |x: u32, y: u32| gray.get_pixel(x, y)[0] as i32;
        for y in 1..sh - 1 {
            for x in 1..sw - 1 {
                let gx = p(x + 1, y - 1) + 2 * p(x + 1, y) + p(x + 1, y + 1)
                    - p(x - 1, y - 1) - 2 * p(x - 1, y) - p(x - 1, y + 1);
                let gy = p(x - 1, y + 1) + 2 * p(x, y + 1) + p(x + 1, y + 1)
                    - p(x - 1, y - 1) - 2 * p(x, y - 1) - p(x + 1, y - 1);
                if gx.abs() + gy.abs() > 160 {
                    edge_hits += 1;
                }
                hist[p(x, y) as usize] += 1;
                samples += 1;
            }
        }
    }
    let edge_density = if samples > 0 {
        (edge_hits as f32 / samples as f32 * 3.0).clamp(0.05, 1.0)
    } else {
        0.5
    };

    // Percentile contrast + entropy from the luma histogram.
    let total: u32 = hist.iter().sum();
    let (mut p5, mut p95, mut acc) = (0usize, 255usize, 0u32);
    let mut entropy = 0.0f32;
    if total > 0 {
        let (lo, hi) = (total / 20, total - total / 20);
        let mut got_lo = false;
        for (v, &c) in hist.iter().enumerate() {
            acc += c;
            if !got_lo && acc > lo {
                p5 = v;
                got_lo = true;
            }
            if acc >= hi {
                p95 = v;
                break;
            }
        }
        for &c in hist.iter().filter(|&&c| c > 0) {
            let pr = c as f32 / total as f32;
            entropy -= pr * pr.ln();
        }
        entropy = (entropy / 256f32.ln()).clamp(0.0, 1.0);
    }
    let contrast_score = (p95.saturating_sub(p5) as f32 / 255.0).clamp(0.1, 1.0);

    // Fraction of 8x8 cells that are nearly flat: high = blank / speech-bubble page.
    let (mut flat, mut cells) = (0u32, 0u32);
    for cy in (0..sh.saturating_sub(7)).step_by(8) {
        for cx in (0..sw.saturating_sub(7)).step_by(8) {
            let (mut lo, mut hi) = (255u8, 0u8);
            for y in cy..cy + 8 {
                for x in cx..cx + 8 {
                    let v = gray.get_pixel(x, y)[0];
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
            cells += 1;
            if hi - lo < 8 {
                flat += 1;
            }
        }
    }
    let flat_ratio = if cells > 0 { flat as f32 / cells as f32 } else { 0.0 };

    // Hasler-Suesstrunk colorfulness (0 for B/W manga, which is fine: it cancels in ranking).
    let (mut n, mut s_rg, mut s_yb, mut q_rg, mut q_yb) = (0f32, 0f32, 0f32, 0f32, 0f32);
    for px in rgb.pixels().step_by(3) {
        let (r, g, b) = (px[0] as f32, px[1] as f32, px[2] as f32);
        let (rg, yb) = (r - g, 0.5 * (r + g) - b);
        n += 1.0;
        s_rg += rg;
        s_yb += yb;
        q_rg += rg * rg;
        q_yb += yb * yb;
    }
    let colorfulness = if n > 0.0 {
        let (m_rg, m_yb) = (s_rg / n, s_yb / n);
        let sd = ((q_rg / n - m_rg * m_rg).max(0.0) + (q_yb / n - m_yb * m_yb).max(0.0)).sqrt();
        ((sd + 0.3 * (m_rg * m_rg + m_yb * m_yb).sqrt()) / 100.0).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let size_bonus = if is_vertical_pan { 0.10 } else { 0.05 };
    let blank_penalty = (flat_ratio - 0.6).max(0.0) * 0.75;
    let heuristic_score = (edge_density * 0.34
        + contrast_score * 0.26
        + entropy * 0.16
        + colorfulness * 0.08
        + size_bonus
        - blank_penalty)
        .clamp(0.05, 0.99);

    let suggested_role = if panel_index == 0 {
        PanelRole::Hook
    } else if is_vertical_pan {
        PanelRole::VerticalPan
    } else if heuristic_score >= 0.75 {
        PanelRole::Climax
    } else if panel_index < total_estimated / 2 {
        PanelRole::BuildUp
    } else {
        PanelRole::AuraCloseup
    };

    println!(
        "[slicer] panel#{} {}x{} edge={:.2} contrast={:.2} entropy={:.2} color={:.2} flat={:.2} score={:.2}",
        panel_index + 1, w, h, edge_density, contrast_score, entropy, colorfulness, flat_ratio, heuristic_score
    );

    // shortcut: flat-cell ratio only; a minimalist all-white art panel could be dropped, lower 0.90 if so.
    let is_blank = flat_ratio >= 0.90;

    VisualMetrics {
        is_blank,
        aspect_ratio,
        edge_density,
        contrast_score,
        heuristic_score,
        suggested_role,
        is_vertical_pan,
    }
}
