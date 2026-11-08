pub mod contact_sheet;
pub mod vision_llm;

use anyhow::Result;
use crate::models::panel::{Panel, VisionReviewConfig};

pub async fn evaluate_epic_panels(
    panels: Vec<Panel>,
    config: &VisionReviewConfig,
) -> Result<Vec<Panel>> {
    vision_llm::score_and_rank_panels(panels, config).await
}
