use crate::core::video_renderer::choreographer::synchronize_panels_to_beats;
use crate::core::video_renderer::ffmpeg_pipe::render_timeline_to_mp4;
use crate::models::audio::BeatMap;
use crate::models::panel::Panel;
use crate::models::timeline::{RenderConfig, TimelineClip};

#[tauri::command]
pub fn generate_auto_timeline(panels: Vec<Panel>, beat_map: BeatMap) -> Vec<TimelineClip> {
    synchronize_panels_to_beats(&panels, &beat_map)
}

#[tauri::command]
pub async fn render_montagem_video(
    panels: Vec<Panel>,
    beat_map: BeatMap,
    timeline: Vec<TimelineClip>,
    config: RenderConfig,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        render_timeline_to_mp4(&panels, &beat_map, &timeline, &config).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ai_direct_timeline(
    panels: Vec<Panel>,
    beat_map: BeatMap,
    config: crate::models::panel::VisionReviewConfig,
) -> Result<Vec<TimelineClip>, String> {
    crate::core::video_renderer::ai_director::direct_with_ai(&panels, &beat_map, &config)
        .await
        .map_err(|e| e.to_string())
}
