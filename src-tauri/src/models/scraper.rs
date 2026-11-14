use serde::{Deserialize, Serialize};
use crate::models::panel::{ComicFormat, Panel, PanelStyle};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlatformKind {
    Webtoon,
    MangaDex,
    VoraToon,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformPreset {
    pub id: PlatformKind,
    pub name: String,
    pub base_url: String,
    pub example_url: String,
    pub default_format: ComicFormat,
    pub image_selector: String,
    pub referer_header: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScraperRequest {
    pub url: String,
    pub platform: PlatformKind,
    pub comic_format: ComicFormat,
    #[serde(default)]
    pub panel_style: PanelStyle,
    pub custom_selector: Option<String>,
    pub custom_referer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrapeResult {
    pub chapter_title: String,
    pub source_platform: String,
    pub source_url: String,
    pub downloaded_paths: Vec<String>,
    pub extracted_panels: Vec<Panel>,
}
