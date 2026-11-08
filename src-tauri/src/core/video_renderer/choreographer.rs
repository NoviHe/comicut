use crate::models::audio::BeatMap;
use crate::models::panel::{Panel, PanelRole};
use crate::models::timeline::{EffectType, TimelineClip};

/// Beat-fitted cut list: the shared first stage of the local and the AI director.
pub struct Plan<'a> {
    /// (start second, is_drop) per cut.
    pub cuts: Vec<(f32, bool)>,
    /// Panels available for the cuts (best-scoring ones when the song has fewer cuts than panels).
    pub pool: Vec<&'a Panel>,
    pub video_end: f32,
    pub drop: f32,
}

/// Cuts land on detected beats and are fitted to the chosen panels:
/// - Before the drop: every 2k beats; from the drop on: every k beats.
/// - k (1, 2, 4) is the smallest stride that gives no more cuts than chosen panels.
/// - Song longer than the panels can fill: the video ends when the panels run out (no repeats).
/// - Fewer cuts than panels: only the best-scoring panels are used, kept in reading order.
pub fn plan<'a>(panels: &'a [Panel], beat_map: &BeatMap) -> Option<Plan<'a>> {
    let chosen: Vec<&Panel> = panels.iter().filter(|p| p.selected).collect();
    if chosen.is_empty() {
        return None;
    }
    let end = beat_map.duration_sec.max(0.5);
    let drop = beat_map.main_drop_sec;

    let build = |k: usize| -> Vec<(f32, bool)> {
        let mut cuts: Vec<(f32, bool)> = vec![(0.0, false)]; // (time, is_drop)
        let (mut pre, mut post) = (0usize, 0usize);
        for b in &beat_map.beats {
            let t = b.timestamp_sec;
            if t < 0.15 || t >= end - 0.2 {
                continue;
            }
            let keep = if b.is_bass_drop {
                post = 1;
                true
            } else if t >= drop - 0.05 {
                let keep = post % k == 0;
                post += 1;
                keep
            } else {
                let keep = pre % (2 * k) == 0;
                pre += 1;
                keep
            };
            if keep && t - cuts.last().unwrap().0 >= 0.25 {
                cuts.push((t, b.is_bass_drop));
            }
        }
        cuts
    };

    // Slower than 4 beats per cut feels dead.
    let mut cuts = build(1);
    for k in [2usize, 4] {
        if cuts.len() <= chosen.len() {
            break;
        }
        cuts = build(k);
    }
    if cuts.len() == 1 {
        // No usable beats: even cuts on the BPM grid.
        let step = (60.0 / beat_map.bpm.max(60.0)) * 2.0;
        let mut t = step;
        while t < end - 0.2 {
            cuts.push((t, false));
            t += step;
        }
    }

    // The window always reaches a few cuts past the drop so the drop is never cut off.
    let mut video_end = end;
    let drop_idx = cuts.iter().position(|c| c.1).unwrap_or(0);
    let window = chosen.len().max(drop_idx + 4);
    if cuts.len() > window {
        video_end = cuts[window].0;
        cuts.truncate(window);
    }

    let mut pool: Vec<&Panel> = chosen.clone();
    if cuts.len() < pool.len() {
        let score = |p: &Panel| p.ai_epic_score.unwrap_or(p.heuristic_score);
        pool.sort_by(|a, b| score(b).partial_cmp(&score(a)).unwrap_or(std::cmp::Ordering::Equal));
        pool.truncate(cuts.len());
        pool.sort_by_key(|p| p.index);
    }
    println!(
        "[timeline] {} chosen panels, {} cuts -> {} unique panels used{}",
        chosen.len(),
        cuts.len(),
        pool.len().min(cuts.len()),
        if cuts.len() > pool.len() { " (panels repeat)" } else { "" }
    );
    Some(Plan { cuts, pool, video_end, drop })
}

/// Turns a plan into clips. `picks[i] = Some((pool index, effect))` overrides the local choice for cut i
/// (used by the AI director); anything missing or invalid falls back to the local rules.
pub fn assemble(plan: &Plan, picks: &[Option<(usize, EffectType)>]) -> Vec<TimelineClip> {
    let pool = &plan.pool;
    let drop = plan.drop;
    // Local rule: the drop gets the Climax panel; every other cut cycles through the remaining panels.
    let climax: Option<&Panel> = pool.iter().copied().find(|p| p.role == PanelRole::Climax);
    let rest: Vec<&Panel> = pool.iter().copied().filter(|p| climax.map_or(true, |c| c.id != p.id)).collect();
    let mut cycle = 0usize;
    let mut clips = Vec::new();
    for (i, &(start, is_drop)) in plan.cuts.iter().enumerate() {
        let stop = plan.cuts.get(i + 1).map(|c| c.0).unwrap_or(plan.video_end);
        let dur = stop - start;
        let after_drop = start >= drop - 0.05;

        let ai = picks
            .get(i)
            .and_then(|p| p.as_ref())
            .and_then(|(pi, e)| pool.get(*pi).map(|p| (*p, e.clone())));
        let panel = match (&ai, is_drop, climax) {
            (Some((p, _)), _, _) => *p,
            (None, true, Some(c)) => c,
            _ if rest.is_empty() => pool[0],
            _ => {
                let p = rest[cycle % rest.len()];
                cycle += 1;
                p
            }
        };

        let tall = panel.is_vertical_pan || panel.role == PanelRole::VerticalPan;
        let long_enough = dur >= 0.7;
        let local_effect = if tall && long_enough {
            EffectType::VerticalPanScroll
        } else if is_drop {
            EffectType::ImpactFlashWhite
        } else if after_drop {
            if i % 2 == 0 { EffectType::ZoomPunch } else { EffectType::CameraShake }
        } else {
            EffectType::SlowPushIn
        };
        let effect = match ai {
            // A scroll on a non-tall panel does nothing visible.
            Some((_, EffectType::VerticalPanScroll)) if !tall => EffectType::SlowPushIn,
            Some((_, e)) => e,
            None => local_effect,
        };

        clips.push(TimelineClip {
            id: format!("clip-{}", i + 1),
            panel_id: panel.id.clone(),
            panel_index: panel.index,
            role: panel.role.clone(),
            start_sec: start,
            end_sec: stop,
            duration_sec: dur.max(0.2),
            effect,
            beat_intensity: if after_drop { 0.95 } else { 0.5 },
            shake_amplitude: if after_drop { 14.0 } else { 2.0 },
            zoom_scale: if after_drop { 1.18 } else { 1.06 },
        });
    }
    clips
}

pub fn synchronize_panels_to_beats(panels: &[Panel], beat_map: &BeatMap) -> Vec<TimelineClip> {
    plan(panels, beat_map).map(|p| assemble(&p, &[])).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::audio::BeatMarker;
    use crate::models::panel::BoundingBox;

    fn panel(i: usize) -> Panel {
        Panel {
            id: format!("p{i}"), index: i, image_path: String::new(), preview_data_url: None,
            bbox: BoundingBox { x: 0, y: 0, width: 720, height: 1000 },
            aspect_ratio: 0.7, edge_density: 0.5, contrast_score: 0.5, heuristic_score: 0.5,
            ai_epic_score: None, role: PanelRole::BuildUp, is_vertical_pan: false, selected: true, caption: None,
        }
    }

    fn song(n_beats: usize) -> BeatMap {
        // 120 BPM, drop on beat 8.
        let beats: Vec<BeatMarker> = (0..n_beats)
            .map(|i| BeatMarker { timestamp_sec: 0.5 + 0.5 * i as f32, intensity: 0.8, is_bass_drop: i == 8, is_sub_bass_hit: true })
            .collect();
        BeatMap {
            audio_path: String::new(), file_name: String::new(),
            duration_sec: 0.5 * n_beats as f32 + 1.0, bpm: 120.0,
            main_drop_sec: 0.5 + 4.0, beats, waveform_peaks: vec![],
        }
    }

    #[test]
    fn cuts_fit_panel_count() {
        // 60 beats, 12 panels: stride widens until cuts <= panels, so no panel repeats.
        let panels: Vec<Panel> = (0..12).map(panel).collect();
        let clips = synchronize_panels_to_beats(&panels, &song(60));
        assert!(clips.len() <= 12, "{} clips", clips.len());
        // Song is 31s; the video must stop when the panels run out.
        assert!(clips.last().unwrap().end_sec < 31.0, "video not shortened");
        let mut ids: Vec<&str> = clips.iter().map(|c| c.panel_id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), clips.len(), "panel repeated");
    }

    #[test]
    fn few_beats_use_best_panels() {
        // 10 beats, 30 panels: only as many panels as cuts, best scores first.
        let mut panels: Vec<Panel> = (0..30).map(panel).collect();
        panels[29].ai_epic_score = Some(0.99);
        let clips = synchronize_panels_to_beats(&panels, &song(10));
        assert!(clips.len() < 30);
        assert!(clips.iter().any(|c| c.panel_id == "p29"));
    }

    #[test]
    fn ai_picks_override_and_bad_ones_fall_back() {
        let panels: Vec<Panel> = (0..6).map(panel).collect();
        let m = song(12);
        let p = plan(&panels, &m).unwrap();
        let mut picks = vec![None; p.cuts.len()];
        picks[1] = Some((4, EffectType::CameraShake)); // valid: panel index 4, shake
        picks[2] = Some((99, EffectType::ZoomPunch)); // pool index out of range: ignored
        picks[3] = Some((0, EffectType::VerticalPanScroll)); // not a tall panel: downgraded
        let clips = assemble(&p, &picks);
        assert_eq!(clips[1].panel_id, p.pool[4].id);
        assert_eq!(clips[1].effect, EffectType::CameraShake);
        assert!(p.pool.iter().any(|x| x.id == clips[2].panel_id)); // bad pool index falls back to a local pick
        assert_eq!(clips[3].effect, EffectType::SlowPushIn);
    }
}
