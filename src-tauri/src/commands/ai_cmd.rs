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
