use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatMarker {
    pub timestamp_sec: f32,
    pub intensity: f32,
    pub is_bass_drop: bool,
    pub is_sub_bass_hit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatMap {
    pub audio_path: String,
    pub file_name: String,
    pub duration_sec: f32,
    pub bpm: f32,
    pub main_drop_sec: f32,
    pub beats: Vec<BeatMarker>,
    pub waveform_peaks: Vec<f32>,
}
