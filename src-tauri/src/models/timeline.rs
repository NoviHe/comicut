use serde::{Deserialize, Serialize};
use crate::models::panel::PanelRole;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EffectType {
    ZoomPunch,
    CameraShake,
    VerticalPanScroll,
    ImpactFlashWhite,
    ImpactFlashRed,
    SlowPushIn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineClip {
    pub id: String,
    pub panel_id: String,
    pub panel_index: usize,
    pub role: PanelRole,
    pub start_sec: f32,
    pub end_sec: f32,
    pub duration_sec: f32,
    pub effect: EffectType,
    pub beat_intensity: f32,
    pub shake_amplitude: f32,
    pub zoom_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkConfig {
    pub enabled: bool,
    pub text: String,
    pub font: String,
    #[serde(default)]
    pub font_size: Option<u32>,
    pub position_x: f32, // 0.0 - 100.0 (% horizontal)
    pub position_y: f32, // 0.0 - 100.0 (% vertical)
    pub opacity: f32,    // 0.0 - 1.0 (default 0.7)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderConfig {
    pub output_path: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub blur_background: bool,
    pub vignette_strength: f32,
    pub beat_flash_enabled: bool,
    #[serde(default)]
    pub watermark: Option<WatermarkConfig>,
}
