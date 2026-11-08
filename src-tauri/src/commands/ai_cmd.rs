use crate::core::epic_reviewer::evaluate_epic_panels;
use crate::models::panel::{Panel, VisionReviewConfig};

#[tauri::command]
pub async fn review_epic_panels(
    panels: Vec<Panel>,
    config: VisionReviewConfig,
) -> Result<Vec<Panel>, String> {
    evaluate_epic_panels(panels, &config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn suggest_songs(
    panels: Vec<Panel>,
    config: VisionReviewConfig,
) -> Result<Vec<String>, String> {
    crate::core::epic_reviewer::vision_llm::suggest_song_queries(&panels, &config)
        .await
        .map_err(|e| e.to_string())
}
