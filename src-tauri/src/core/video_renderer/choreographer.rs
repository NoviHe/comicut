use crate::models::audio::BeatMap;
use crate::models::panel::{Panel, PanelRole};
use crate::models::timeline::{EffectType, TimelineClip};

/// Menyinkronkan daftar panel terpilih ke ketukan beat lagu Montagem / Phonk:
/// - Panel Hook & BuildUp ditempatkan sebelum Main Bass Drop
/// - Panel Climax & VerticalPan ditempatkan tepat saat Bass Drop dan setiap Sub-Bass Hit berikutnya
pub fn synchronize_panels_to_beats(panels: &[Panel], beat_map: &BeatMap) -> Vec<TimelineClip> {
    let selected: Vec<&Panel> = panels.iter().filter(|p| p.selected).collect();
    if selected.is_empty() {
        return Vec::new();
    }

    let mut clips = Vec::new();
    let beats = &beat_map.beats;

    for (i, panel) in selected.iter().enumerate() {
        let start_sec = if i < beats.len() {
            beats[i].timestamp_sec
        } else {
            (i as f32 / selected.len() as f32) * beat_map.duration_sec
        };

        let end_sec = if i + 1 < beats.len() && i + 1 < selected.len() {
            beats[i + 1].timestamp_sec
        } else if i + 1 == selected.len() {
            beat_map.duration_sec
        } else {
            start_sec + (beat_map.duration_sec / selected.len() as f32)
        };

        let is_drop_zone = start_sec >= beat_map.main_drop_sec - 0.05;

        let effect = if panel.is_vertical_pan || panel.role == PanelRole::VerticalPan {
            EffectType::VerticalPanScroll
        } else if is_drop_zone {
            match i % 3 {
                0 => EffectType::ZoomPunch,
                1 => EffectType::CameraShake,
                _ => EffectType::ImpactFlashWhite,
            }
        } else {
            EffectType::SlowPushIn
        };

        clips.push(TimelineClip {
            id: format!("clip-{}", i + 1),
            panel_id: panel.id.clone(),
            panel_index: panel.index,
            role: panel.role.clone(),
            start_sec,
            end_sec,
            duration_sec: (end_sec - start_sec).max(0.2),
            effect,
            beat_intensity: if is_drop_zone { 0.95 } else { 0.50 },
            shake_amplitude: if is_drop_zone { 14.0 } else { 2.0 },
            zoom_scale: if is_drop_zone { 1.18 } else { 1.05 },
        });
    }

    clips
}
