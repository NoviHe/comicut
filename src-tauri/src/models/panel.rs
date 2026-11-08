use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComicFormat {
    WebtoonVertical,
    MangaRTL,
    MangaLTR,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PanelRole {
    Hook,
    BuildUp,
    AuraCloseup,
    Climax,
    VerticalPan,
    Filler,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    pub id: String,
    pub index: usize,
    pub image_path: String,
    pub preview_data_url: Option<String>,
    pub bbox: BoundingBox,
    pub aspect_ratio: f32,
    pub edge_density: f32,
    pub contrast_score: f32,
    pub heuristic_score: f32,
    pub ai_epic_score: Option<f32>,
    pub role: PanelRole,
    pub is_vertical_pan: bool,
    pub selected: bool,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionReviewConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub model_name: String,
    pub base_url: Option<String>,
    pub max_panels_to_select: usize,
}
