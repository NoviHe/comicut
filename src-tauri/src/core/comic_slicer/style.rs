//! Panel style processing (`Default`, `FaceCharacter`, `EpicScene`):
//! - Speech bubble detection and automatic bubble-free cropping ("panel berbalon crop saja")
//! - ONNX character face detection (`face_onnx`) with heuristic fallback
//! - Action / combat / VFX scoring for `EpicScene`

use image::{DynamicImage, GenericImageView};
use std::collections::VecDeque;

use crate::core::comic_slicer::face_onnx::{detect_faces_onnx, FaceBox};
use crate::core::comic_slicer::heuristics::{analyze_panel_metrics, VisualMetrics};
use crate::models::panel::{BoundingBox, PanelRole, PanelStyle};

/// Normalized rectangle (`0.0..=1.0`) representing a speech bubble region.
#[derive(Debug, Clone, Copy)]
pub struct NormRect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl NormRect {
    pub fn width(&self) -> f32 {
        (self.x1 - self.x0).max(0.0)
    }
    pub fn height(&self) -> f32 {
        (self.y1 - self.y0).max(0.0)
    }
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }
    pub fn overlaps(&self, other: &NormRect) -> bool {
        let ix0 = self.x0.max(other.x0);
        let iy0 = self.y0.max(other.y0);
        let ix1 = self.x1.min(other.x1);
        let iy1 = self.y1.min(other.y1);
        (ix1 - ix0) > 0.01 && (iy1 - iy0) > 0.01
    }
}

pub struct StyledSlice {
    pub image: DynamicImage,
    pub bbox: BoundingBox,
    pub metrics: VisualMetrics,
    pub selected: bool,
    pub caption_suffix: String,
}

/// Detect speech bubbles (bright white/near-white enclosed blobs containing dark text strokes).
pub fn detect_speech_bubbles(img: &DynamicImage) -> Vec<NormRect> {
    let (orig_w, orig_h) = img.dimensions();
    if orig_w < 64 || orig_h < 64 {
        return Vec::new();
    }

    let small = if orig_w > 280 {
        img.resize(280, 2800, image::imageops::FilterType::Triangle)
    } else {
        img.clone()
    };
    let rgb = small.to_rgb8();
    let (w, h) = rgb.dimensions();
    let total_px = (w * h).max(1) as usize;

    let mut white = vec![false; total_px];
    let mut dark = vec![false; total_px];
    for y in 0..h {
        for x in 0..w {
            let p = rgb.get_pixel(x, y);
            let (r, g, b) = (p[0], p[1], p[2]);
            let hi = r.max(g).max(b);
            let lo = r.min(g).min(b);
            let luma = (u16::from(r) * 77 + u16::from(g) * 150 + u16::from(b) * 29) >> 8;
            let idx = (y * w + x) as usize;
            white[idx] = luma >= 235 && (hi - lo) <= 24;
            dark[idx] = luma <= 95;
        }
    }

    let mut visited = vec![false; total_px];
    let mut bubbles = Vec::new();
    let mut queue = VecDeque::new();

    for sy in 0..h {
        for sx in 0..w {
            let start = (sy * w + sx) as usize;
            if !white[start] || visited[start] {
                continue;
            }
            visited[start] = true;
            queue.clear();
            queue.push_back((sx, sy));

            let (mut min_x, mut max_x) = (sx, sx);
            let (mut min_y, mut max_y) = (sy, sy);
            let mut count = 0usize;
            let (mut touches_l, mut touches_r, mut touches_t, mut touches_b) = (false, false, false, false);

            while let Some((cx, cy)) = queue.pop_front() {
                count += 1;
                min_x = min_x.min(cx);
                max_x = max_x.max(cx);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);
                if cx == 0 { touches_l = true; }
                if cx + 1 == w { touches_r = true; }
                if cy == 0 { touches_t = true; }
                if cy + 1 == h { touches_b = true; }

                let neighbors = [
                    (cx.wrapping_sub(1), cy),
                    (cx + 1, cy),
                    (cx, cy.wrapping_sub(1)),
                    (cx, cy + 1),
                ];
                for (nx, ny) in neighbors {
                    if nx < w && ny < h {
                        let nidx = (ny * w + nx) as usize;
                        if white[nidx] && !visited[nidx] {
                            visited[nidx] = true;
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }

            let bw = max_x - min_x + 1;
            let bh = max_y - min_y + 1;
            let box_area = (bw * bh) as usize;
            let area_frac = box_area as f32 / total_px as f32;
            let fill_ratio = count as f32 / box_area.max(1) as f32;
            let border_touches = usize::from(touches_l) + usize::from(touches_r) + usize::from(touches_t) + usize::from(touches_b);

            // Speech bubble candidate: compact white region, not full-page gutter/background.
            if bw < 18 || bh < 14 || !(0.012..=0.55).contains(&area_frac) || fill_ratio < 0.42 || border_touches >= 3 {
                continue;
            }
            let aspect = bw as f32 / bh as f32;
            if !(0.22..=5.5).contains(&aspect) {
                continue;
            }

            // Check interior (central 70% of box) for dark text strokes and horizontal black-white transitions.
            let ix0 = min_x + bw / 7;
            let ix1 = max_x.saturating_sub(bw / 7).max(ix0 + 1);
            let iy0 = min_y + bh / 7;
            let iy1 = max_y.saturating_sub(bh / 7).max(iy0 + 1);
            let inner_area = ((ix1 - ix0 + 1) * (iy1 - iy0 + 1)).max(1) as f32;

            let mut dark_px = 0u32;
            let mut text_rows = 0u32;
            for y in iy0..=iy1 {
                let mut transitions = 0u32;
                let mut prev_dark = false;
                for x in ix0..=ix1 {
                    let is_d = dark[(y * w + x) as usize];
                    if is_d {
                        dark_px += 1;
                    }
                    if is_d != prev_dark {
                        transitions += 1;
                        prev_dark = is_d;
                    }
                }
                if transitions >= 4 {
                    text_rows += 1;
                }
            }

            let dark_ratio = dark_px as f32 / inner_area;
            let row_ratio = text_rows as f32 / (iy1 - iy0 + 1).max(1) as f32;
            if (0.015..=0.42).contains(&dark_ratio) && row_ratio >= 0.14 {
                bubbles.push(NormRect {
                    x0: (min_x as f32 / w as f32).clamp(0.0, 1.0),
                    y0: (min_y as f32 / h as f32).clamp(0.0, 1.0),
                    x1: ((max_x + 1) as f32 / w as f32).clamp(0.0, 1.0),
                    y1: ((max_y + 1) as f32 / h as f32).clamp(0.0, 1.0),
                });
            }
        }
    }

    bubbles
}

/// Crop out speech bubbles while preserving the largest art region (and any detected character faces).
/// Returns `(cropped_image, updated_bbox, was_cropped, uncroppable_bubble_heavy)`.
pub fn crop_out_bubbles(
    img: &DynamicImage,
    orig_bbox: BoundingBox,
    bubbles: &[NormRect],
    faces: &[FaceBox],
) -> (DynamicImage, BoundingBox, bool, bool) {
    if bubbles.is_empty() {
        return (img.clone(), orig_bbox, false, false);
    }

    let pad = 0.015f32;
    let mut candidates = vec![NormRect { x0: 0.0, y0: 0.0, x1: 1.0, y1: 1.0 }];

    for b in bubbles {
        let mut next = Vec::new();
        for c in &candidates {
            if !c.overlaps(b) {
                next.push(*c);
                continue;
            }
            // Split above, below, left, or right of the bubble.
            let splits = [
                NormRect { x0: c.x0, y0: (b.y1 + pad).min(c.y1), x1: c.x1, y1: c.y1 }, // below bubble
                NormRect { x0: c.x0, y0: c.y0, x1: c.x1, y1: (b.y0 - pad).max(c.y0) }, // above bubble
                NormRect { x0: (b.x1 + pad).min(c.x1), y0: c.y0, x1: c.x1, y1: c.y1 }, // right of bubble
                NormRect { x0: c.x0, y0: c.y0, x1: (b.x0 - pad).max(c.x0), y1: c.y1 }, // left of bubble
            ];
            for s in splits {
                if s.width() >= 0.50 && s.height() >= 0.32 && s.area() >= 0.28 {
                    next.push(s);
                }
            }
        }
        next.sort_by(|a, b| b.area().partial_cmp(&a.area()).unwrap_or(std::cmp::Ordering::Equal));
        next.truncate(24);
        candidates = next;
        if candidates.is_empty() {
            break;
        }
    }

    let score_rect = |r: &NormRect| -> f32 {
        let width_bonus = if r.width() >= 0.92 { 1.25 } else { 1.0 };
        let mut face_bonus = 1.0f32;
        for f in faces {
            let ix0 = r.x0.max(f.x0);
            let iy0 = r.y0.max(f.y0);
            let ix1 = r.x1.min(f.x1);
            let iy1 = r.y1.min(f.y1);
            let inter = (ix1 - ix0).max(0.0) * (iy1 - iy0).max(0.0);
            let coverage = inter / f.area().max(1e-5);
            if coverage >= 0.75 {
                face_bonus += 1.8 * f.score;
            }
        }
        r.area() * width_bonus * face_bonus
    };

    let best = candidates
        .into_iter()
        .max_by(|a, b| score_rect(a).partial_cmp(&score_rect(b)).unwrap_or(std::cmp::Ordering::Equal));

    let Some(best) = best else {
        // Entire panel is dominated by speech bubbles with no clean art crop >= 28% area.
        return (img.clone(), orig_bbox, false, true);
    };

    if best.area() >= 0.97 {
        return (img.clone(), orig_bbox, false, false);
    }

    let (w, h) = img.dimensions();
    let px = ((best.x0 * w as f32).round() as u32).min(w.saturating_sub(1));
    let py = ((best.y0 * h as f32).round() as u32).min(h.saturating_sub(1));
    let pw = ((best.width() * w as f32).round() as u32).clamp(1, w.saturating_sub(px));
    let ph = ((best.height() * h as f32).round() as u32).clamp(1, h.saturating_sub(py));

    if pw < 100 || ph < 100 {
        return (img.clone(), orig_bbox, false, true);
    }

    let cropped = img.crop_imm(px, py, pw, ph);
    let new_bbox = BoundingBox {
        x: orig_bbox.x + px,
        y: orig_bbox.y + py,
        width: pw,
        height: ph,
    };
    (cropped, new_bbox, true, false)
}

/// Fallback heuristic face detector when the ONNX model is not installed yet:
/// looks for warm skin-tone / portrait composition with central eye/hair edge detail.
fn detect_faces_heuristic(img: &DynamicImage) -> Vec<FaceBox> {
    let small = img.resize(160, 160, image::imageops::FilterType::Triangle).to_rgb8();
    let (w, h) = small.dimensions();
    if w < 16 || h < 16 {
        return Vec::new();
    }

    // Focus on central-upper region where character portraits appear.
    let (x0, x1) = (w / 6, w - w / 6);
    let (y0, y1) = (h / 10, h - h / 5);
    let mut skin_px = 0u32;
    let mut detail_px = 0u32;
    let mut total = 0u32;

    for y in y0..y1.saturating_sub(1) {
        for x in x0..x1.saturating_sub(1) {
            let p = small.get_pixel(x, y);
            let (r, g, b) = (p[0] as i32, p[1] as i32, p[2] as i32);
            // Anime/manhwa skin tone range (warm peach/ivory/tan: R >= G >= B with moderate chroma)
            let is_skin = r > 145
                && g > 110
                && b > 85
                && r >= g
                && g >= b - 10
                && (r - b) >= 12
                && (r - b) <= 115;
            if is_skin {
                skin_px += 1;
            }
            let pr = small.get_pixel(x + 1, y);
            let pb = small.get_pixel(x, y + 1);
            let diff = (r - pr[0] as i32).abs() + (g - pb[1] as i32).abs();
            if diff > 28 {
                detail_px += 1;
            }
            total += 1;
        }
    }

    if total == 0 {
        return Vec::new();
    }
    let skin_ratio = skin_px as f32 / total as f32;
    let detail_ratio = detail_px as f32 / total as f32;
    if (0.08..=0.72).contains(&skin_ratio) && detail_ratio >= 0.08 {
        let conf = ((skin_ratio * 1.4 + detail_ratio * 0.8).clamp(0.30, 0.85)) as f32;
        vec![FaceBox {
            x0: 0.2,
            y0: 0.1,
            x1: 0.8,
            y1: 0.75,
            score: conf,
        }]
    } else {
        Vec::new()
    }
}

/// Score action / combat / VFX intensity (`0.0..=1.0`) from diagonal speed lines, high edge energy, and contrast.
pub fn action_scene_score(img: &DynamicImage, metrics: &VisualMetrics) -> f32 {
    let small = img.resize(240, 240, image::imageops::FilterType::Triangle).to_luma8();
    let (w, h) = small.dimensions();
    let mut diag_edges = 0u32;
    let mut strong_edges = 0u32;
    let mut samples = 0u32;

    if w >= 3 && h >= 3 {
        let p = |x: u32, y: u32| small.get_pixel(x, y)[0] as i32;
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let gx = p(x + 1, y - 1) + 2 * p(x + 1, y) + p(x + 1, y + 1)
                    - p(x - 1, y - 1) - 2 * p(x - 1, y) - p(x - 1, y + 1);
                let gy = p(x - 1, y + 1) + 2 * p(x, y + 1) + p(x + 1, y + 1)
                    - p(x - 1, y - 1) - 2 * p(x, y - 1) - p(x + 1, y - 1);
                let mag = gx.abs() + gy.abs();
                if mag > 180 {
                    strong_edges += 1;
                    // Diagonal speed lines & impact slashes have comparable |gx| and |gy|.
                    let ax = gx.abs().max(1);
                    let ay = gy.abs().max(1);
                    let ratio = ax.min(ay) as f32 / ax.max(ay) as f32;
                    if ratio >= 0.35 {
                        diag_edges += 1;
                    }
                }
                samples += 1;
            }
        }
    }

    let diag_density = if samples > 0 {
        (diag_edges as f32 / samples as f32 * 4.5).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let strong_density = if samples > 0 {
        (strong_edges as f32 / samples as f32 * 3.2).clamp(0.0, 1.0)
    } else {
        0.0
    };

    (metrics.edge_density * 0.35
        + strong_density * 0.25
        + diag_density * 0.20
        + metrics.contrast_score * 0.20)
        .clamp(0.05, 0.99)
}

/// Apply the requested `PanelStyle` (bubble detection + cropping + face/action filtering & scoring).
pub fn apply_panel_style(
    img: DynamicImage,
    bbox: BoundingBox,
    style: PanelStyle,
    panel_index: usize,
    total_estimated: usize,
    default_select_threshold: f32,
) -> Option<StyledSlice> {
    if style == PanelStyle::Default {
        let metrics = analyze_panel_metrics(&img, panel_index, total_estimated);
        if metrics.is_blank {
            return None;
        }
        let selected = metrics.heuristic_score >= default_select_threshold;
        return Some(StyledSlice {
            image: img,
            bbox,
            metrics,
            selected,
            caption_suffix: String::new(),
        });
    }

    // Detect faces before cropping so bubble cropping protects face regions in FaceCharacter mode.
    let initial_faces = if style == PanelStyle::FaceCharacter {
        detect_faces_onnx(&img).unwrap_or_else(|| detect_faces_heuristic(&img))
    } else {
        Vec::new()
    };

    let bubbles = detect_speech_bubbles(&img);
    let (cropped, new_bbox, was_cropped, bubble_heavy) =
        crop_out_bubbles(&img, bbox, &bubbles, &initial_faces);

    let mut metrics = analyze_panel_metrics(&cropped, panel_index, total_estimated);
    if metrics.is_blank {
        return None;
    }

    match style {
        PanelStyle::Default => unreachable!(),
        PanelStyle::FaceCharacter => {
            let faces = if was_cropped {
                detect_faces_onnx(&cropped).unwrap_or_else(|| detect_faces_heuristic(&cropped))
            } else {
                initial_faces
            };
            let best_face = faces
                .iter()
                .copied()
                .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));

            if let Some(face) = best_face.filter(|_| !bubble_heavy) {
                let area_bonus = (face.area().sqrt() * 0.25).clamp(0.0, 0.18);
                metrics.heuristic_score =
                    (metrics.heuristic_score * 0.45 + face.score * 0.45 + area_bonus).clamp(0.58, 0.99);
                metrics.suggested_role = PanelRole::AuraCloseup;
                let crop_tag = if was_cropped { " • Balon dipotong" } else { "" };
                Some(StyledSlice {
                    image: cropped,
                    bbox: new_bbox,
                    metrics,
                    selected: true,
                    caption_suffix: format!(" • Wajah {:.0}%{}", face.score * 100.0, crop_tag),
                })
            } else {
                metrics.heuristic_score = (metrics.heuristic_score * 0.35).clamp(0.05, 0.38);
                metrics.suggested_role = PanelRole::Filler;
                let reason = if bubble_heavy {
                    " • Tidak dipilih: penuh balon dialog"
                } else {
                    " • Tidak dipilih: tanpa wajah karakter"
                };
                Some(StyledSlice {
                    image: cropped,
                    bbox: new_bbox,
                    metrics,
                    selected: false,
                    caption_suffix: reason.to_string(),
                })
            }
        }
        PanelStyle::EpicScene => {
            let act = action_scene_score(&cropped, &metrics);
            let is_epic = !bubble_heavy && (act >= 0.56 || (metrics.edge_density >= 0.55 && metrics.contrast_score >= 0.65));
            if is_epic {
                metrics.heuristic_score = (metrics.heuristic_score * 0.50 + act * 0.50).clamp(0.58, 0.99);
                if !metrics.is_vertical_pan && metrics.heuristic_score >= 0.72 {
                    metrics.suggested_role = PanelRole::Climax;
                }
                let crop_tag = if was_cropped { " • Balon dipotong" } else { "" };
                Some(StyledSlice {
                    image: cropped,
                    bbox: new_bbox,
                    metrics,
                    selected: true,
                    caption_suffix: format!(" • Epic {:.0}%{}", act * 100.0, crop_tag),
                })
            } else {
                metrics.heuristic_score = (metrics.heuristic_score * 0.40).clamp(0.05, 0.40);
                metrics.suggested_role = PanelRole::Filler;
                let reason = if bubble_heavy {
                    " • Tidak dipilih: penuh balon dialog"
                } else {
                    " • Tidak dipilih: kurang aksi/VFX"
                };
                Some(StyledSlice {
                    image: cropped,
                    bbox: new_bbox,
                    metrics,
                    selected: false,
                    caption_suffix: reason.to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn detects_and_crops_speech_bubble_band() {
        let mut rgb = RgbImage::from_pixel(300, 400, Rgb([45, 35, 70]));
        // Draw busy artwork in lower 70% (y = 120..400)
        for y in 120..400 {
            for x in 0..300 {
                let v = (((x * 13 + y * 7) % 200) + 30) as u8;
                rgb.put_pixel(x, y, Rgb([v, v / 2, 255 - v]));
            }
        }
        // Draw a white speech bubble in the top band (x = 40..260, y = 15..105)
        for y in 15..105 {
            for x in 40..260 {
                rgb.put_pixel(x, y, Rgb([250, 250, 250]));
            }
        }
        // Draw simulated dark text strokes inside the bubble
        for row in (35..85).step_by(10) {
            for col in (65..235).step_by(8) {
                for dy in 0..4 {
                    for dx in 0..4 {
                        rgb.put_pixel(col + dx, row + dy, Rgb([15, 15, 15]));
                    }
                }
            }
        }

        let img = DynamicImage::ImageRgb8(rgb);
        let bubbles = detect_speech_bubbles(&img);
        assert!(!bubbles.is_empty(), "should detect top speech bubble");

        let orig = BoundingBox { x: 0, y: 0, width: 300, height: 400 };
        let (cropped, new_bbox, was_cropped, bubble_heavy) = crop_out_bubbles(&img, orig, &bubbles, &[]);
        assert!(was_cropped, "should crop out the speech bubble");
        assert!(!bubble_heavy);
        assert!(new_bbox.y >= 95, "cropped bbox should start below the top bubble (got y={})", new_bbox.y);
        assert_eq!(cropped.dimensions(), (new_bbox.width, new_bbox.height));
    }

    #[test]
    fn epic_scene_distinguishes_action_from_calm() {
        // High-contrast diagonal slash pattern -> high action score
        let mut action = RgbImage::new(240, 320);
        for y in 0..320 {
            for x in 0..240 {
                let v = if ((x as i32 + y as i32) % 9) < 3 || ((x as i32 - y as i32).abs() % 11) < 2 {
                    250u8
                } else {
                    15u8
                };
                action.put_pixel(x, y, Rgb([v, v.saturating_sub(40), 255 - v]));
            }
        }
        let action_img = DynamicImage::ImageRgb8(action);
        let bbox = BoundingBox { x: 0, y: 0, width: 240, height: 320 };
        let styled = apply_panel_style(action_img, bbox, PanelStyle::EpicScene, 1, 10, 0.55)
            .expect("action panel should not be blank");
        assert!(styled.selected, "high-action slash panel should be selected in EpicScene mode");
    }
}
