pub mod beat_detector;
pub mod decoder;

use anyhow::Result;
use crate::models::audio::BeatMap;

pub fn analyze_track(audio_path: &str) -> Result<BeatMap> {
    let decoded = decoder::decode_audio_mono(audio_path)?;
    beat_detector::detect_montagem_beats(
        audio_path,
        &decoded.samples,
        decoded.sample_rate,
    )
}
