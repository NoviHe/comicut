//! ONNX anime / comic character face detector (`deepghs/anime_face_detection` YOLOv8n).
//!
//! Loads `%LOCALAPPDATA%\ComiCut\bin\anime-face.onnx` (managed via Dependency Status) once into a
//! lazy `tract-onnx` runnable plan. Falls back to `None` if the model file is not installed yet.

use image::{DynamicImage, GenericImageView};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;
use tract_onnx::prelude::*;

/// Normalized bounding box (`0.0..=1.0` relative to the image dimensions).
#[derive(Debug, Clone, Copy)]
pub struct FaceBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub score: f32,
}

impl FaceBox {
    pub fn area(&self) -> f32 {
        (self.x1 - self.x0).max(0.0) * (self.y1 - self.y0).max(0.0)
    }
}

type Plan = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

struct CachedModel {
    path: PathBuf,
    modified: Option<SystemTime>,
    plan: Plan,
}

static MODEL: Mutex<Option<CachedModel>> = Mutex::new(None);

const INPUT_SIZE: u32 = 640;
/// Optimal F1 confidence threshold from `deepghs/anime_face_detection` `face_detect_v1.4_n/threshold.json`.
const CONF_THRESHOLD: f32 = 0.28;
const NMS_IOU_THRESHOLD: f32 = 0.45;

fn load_or_get_plan() -> Option<Plan> {
    let path = crate::commands::deps_cmd::face_model_path()?;
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();

    let mut guard = MODEL.lock().ok()?;
    if let Some(cached) = guard.as_ref() {
        if cached.path == path && cached.modified == modified {
            return Some(cached.plan.clone());
        }
    }

    let mut model = tract_onnx::onnx()
        .with_ignore_output_shapes(true)
        .model_for_path(&path)
        .map_err(|e| println!("[onnx] model_for_path err: {e:?}"))
        .ok()?;
    // Clear any intermediate value_info symbolic shapes (e.g. Sym(batch)) so concrete input shapes propagate cleanly.
    for node in &mut model.nodes {
        for output in &mut node.outputs {
            output.fact = InferenceFact::default();
        }
    }
    let model = model
        .with_input_fact(
            0,
            InferenceFact::dt_shape(f32::datum_type(), tvec!(1, 3, INPUT_SIZE as usize, INPUT_SIZE as usize)),
        )
        .map_err(|e| println!("[onnx] with_input_fact err: {e:?}"))
        .ok()?
        .into_optimized()
        .map_err(|e| println!("[onnx] into_optimized err: {e:?}"))
        .ok()?
        .into_runnable()
        .map_err(|e| println!("[onnx] into_runnable err: {e:?}"))
        .ok()?;

    *guard = Some(CachedModel {
        path,
        modified,
        plan: model.clone(),
    });
    Some(model)
}

/// Detect character faces using the ONNX model. Returns `None` if the ONNX model is not installed.
pub fn detect_faces_onnx(img: &DynamicImage) -> Option<Vec<FaceBox>> {
    let plan = load_or_get_plan()?;
    let (orig_w, orig_h) = img.dimensions();
    if orig_w == 0 || orig_h == 0 {
        return Some(Vec::new());
    }

    // Letterbox resize into 640x640 (gray 114 padding) to preserve aspect ratio.
    let scale = (INPUT_SIZE as f32 / orig_w as f32).min(INPUT_SIZE as f32 / orig_h as f32);
    let new_w = ((orig_w as f32 * scale).round() as u32).clamp(1, INPUT_SIZE);
    let new_h = ((orig_h as f32 * scale).round() as u32).clamp(1, INPUT_SIZE);
    let pad_x = (INPUT_SIZE - new_w) / 2;
    let pad_y = (INPUT_SIZE - new_h) / 2;

    let resized = img
        .resize_exact(new_w, new_h, image::imageops::FilterType::Triangle)
        .to_rgb8();

    let side = INPUT_SIZE as usize;
    let plane = side * side;
    let mut input_data = vec![114.0f32 / 255.0; 3 * plane];
    for y in 0..new_h as usize {
        let dst_y = y + pad_y as usize;
        for x in 0..new_w as usize {
            let dst_x = x + pad_x as usize;
            let px = resized.get_pixel(x as u32, y as u32);
            let idx = dst_y * side + dst_x;
            input_data[idx] = px[0] as f32 / 255.0;
            input_data[plane + idx] = px[1] as f32 / 255.0;
            input_data[2 * plane + idx] = px[2] as f32 / 255.0;
        }
    }

    let tensor = tract_ndarray::Array4::from_shape_vec((1, 3, side, side), input_data)
        .ok()?
        .into_tensor();
    let result = plan.run(tvec!(tensor.into())).ok()?;
    let output = result[0].to_array_view::<f32>().ok()?;
    let shape = output.shape();
    // YOLOv8 output shape: [1, 5, 8400] where 5 = [cx, cy, w, h, conf]
    if shape.len() != 3 || shape[1] < 5 {
        return Some(Vec::new());
    }
    let anchors = shape[2];
    let mut candidates: Vec<FaceBox> = Vec::new();
    for i in 0..anchors {
        let score = output[[0, 4, i]];
        if score < CONF_THRESHOLD {
            continue;
        }
        let cx = output[[0, 0, i]];
        let cy = output[[0, 1, i]];
        let w = output[[0, 2, i]];
        let h = output[[0, 3, i]];

        let x0 = ((cx - 0.5 * w - pad_x as f32) / new_w as f32).clamp(0.0, 1.0);
        let y0 = ((cy - 0.5 * h - pad_y as f32) / new_h as f32).clamp(0.0, 1.0);
        let x1 = ((cx + 0.5 * w - pad_x as f32) / new_w as f32).clamp(0.0, 1.0);
        let y1 = ((cy + 0.5 * h - pad_y as f32) / new_h as f32).clamp(0.0, 1.0);
        if x1 - x0 > 0.03 && y1 - y0 > 0.03 {
            candidates.push(FaceBox { x0, y0, x1, y1, score });
        }
    }

    Some(nms(candidates))
}

fn iou(a: &FaceBox, b: &FaceBox) -> f32 {
    let ix0 = a.x0.max(b.x0);
    let iy0 = a.y0.max(b.y0);
    let ix1 = a.x1.min(b.x1);
    let iy1 = a.y1.min(b.y1);
    let inter = (ix1 - ix0).max(0.0) * (iy1 - iy0).max(0.0);
    let union = a.area() + b.area() - inter;
    if union <= 0.0 { 0.0 } else { inter / union }
}

fn nms(mut boxes: Vec<FaceBox>) -> Vec<FaceBox> {
    boxes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    let mut kept: Vec<FaceBox> = Vec::new();
    for b in boxes {
        if kept.iter().all(|k| iou(k, &b) < NMS_IOU_THRESHOLD) {
            kept.push(b);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_runs_onnx_if_present() {
        if crate::commands::deps_cmd::face_model_path().is_none() {
            return;
        }
        let img = DynamicImage::new_rgb8(320, 480);
        let faces = detect_faces_onnx(&img).expect("ONNX plan should load and run");
        assert!(faces.is_empty(), "blank image should have no faces");
    }
}
