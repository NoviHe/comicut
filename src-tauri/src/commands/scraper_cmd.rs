use crate::core::scraper::{default_platform_presets, scrape_and_slice_chapter};
use crate::models::scraper::{PlatformPreset, ScrapeResult, ScraperRequest};

#[tauri::command]
pub fn get_platform_presets() -> Vec<PlatformPreset> {
    default_platform_presets()
}

#[tauri::command]
pub async fn scrape_comic_chapter(request: ScraperRequest) -> Result<ScrapeResult, String> {
    scrape_and_slice_chapter(&request)
        .await
        .map_err(|e| e.to_string())
}
