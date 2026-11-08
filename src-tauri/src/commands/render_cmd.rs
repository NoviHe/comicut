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
    render_timeline_to_mp4(&panels, &beat_map, &timeline, &config).map_err(|e| e.to_string())
}
