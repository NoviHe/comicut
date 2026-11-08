use crate::core::audio_beat::analyze_track;
use crate::models::audio::BeatMap;

#[tauri::command]
pub async fn analyze_audio_beats(audio_path: String) -> Result<BeatMap, String> {
    analyze_track(&audio_path).map_err(|e| e.to_string())
}
