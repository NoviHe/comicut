use anyhow::{anyhow, Result};
use serde_json::Value;
use std::fmt::Write;

use crate::core::epic_reviewer::vision_llm::{ask, thumb_b64};
use crate::core::video_renderer::choreographer::{assemble, plan};
use crate::models::audio::BeatMap;
use crate::models::panel::{Panel, VisionReviewConfig};
use crate::models::timeline::{EffectType, TimelineClip};

const MAX_IMAGES: usize = 40;

const PROMPT: &str = "You are the editor of a fast-cut comic 'montagem' video (phonk beat, 9:16 Reels/Shorts). \
The cut times are fixed to the beat. You choose which comic panel plays in each cut and its camera effect. \
Rules: use each panel at most once and keep story order unless a different order is more dramatic; \
the cut marked DROP must get the single most epic/climactic panel with ImpactFlashWhite or ImpactFlashRed; \
before the drop build tension with calm panels and SlowPushIn; after the drop use high-energy panels with \
ZoomPunch or CameraShake, matching strong beats; VerticalPanScroll only for tall strip panels and cuts of 0.7s or more. \
Effects: ZoomPunch, CameraShake, VerticalPanScroll, ImpactFlashWhite, ImpactFlashRed, SlowPushIn. \
Reply with ONLY a JSON array, one object per cut: [{\"cut\":1,\"panel\":3,\"effect\":\"SlowPushIn\"}].";

/// AI picks panel + effect for every beat-fitted cut. Invalid or missing answers fall back to the local rules.
pub async fn direct_with_ai(
    panels: &[Panel],
    beat_map: &BeatMap,
    config: &VisionReviewConfig,
) -> Result<Vec<TimelineClip>> {
    let plan = plan(panels, beat_map).ok_or_else(|| anyhow!("Tidak ada panel terpilih."))?;

    let mut images: Vec<(String, String)> = Vec::new();
    for (i, p) in plan.pool.iter().enumerate().take(MAX_IMAGES) {
        let Ok(b64) = thumb_b64(&p.image_path, 384) else { continue };
        let tall = if p.is_vertical_pan { ", tall strip" } else { "" };
        images.push((format!("Panel {} ({}x{}{}):", i + 1, p.bbox.width, p.bbox.height, tall), b64));
    }
    if images.is_empty() {
        return Err(anyhow!("Tidak ada gambar panel yang bisa dikirim ke AI."));
    }

    let mut cuts_txt = String::from("Cuts:\n");
    for (i, &(start, is_drop)) in plan.cuts.iter().enumerate() {
        let stop = plan.cuts.get(i + 1).map(|c| c.0).unwrap_or(plan.video_end);
        let strength = beat_map
            .beats
            .iter()
            .find(|b| (b.timestamp_sec - start).abs() < 0.1)
            .map(|b| b.intensity)
            .unwrap_or(0.0);
        let _ = writeln!(
            cuts_txt,
            "Cut {}: start {:.2}s, length {:.2}s, beat strength {:.2}{}",
            i + 1,
            start,
            stop - start,
            strength,
            if is_drop { ", DROP" } else { "" }
        );
    }
    let _ = write!(
        cuts_txt,
        "Main drop at {:.2}s, {:.0} BPM. Panels follow in story order.",
        beat_map.main_drop_sec, beat_map.bpm
    );

    println!("[director] AI call: {} cuts, {} panels, model={}", plan.cuts.len(), images.len(), config.model_name);
    let prompt = format!("{}\n\n{}", PROMPT, cuts_txt);
    let text = ask(config, &prompt, &images).await?;
    println!("[director] response: {}", text);

    let (Some(s), Some(e)) = (text.find('['), text.rfind(']')) else {
        return Err(anyhow!(
            "Respons AI bukan JSON array: {}",
            text.chars().take(200).collect::<String>()
        ));
    };
    let items: Vec<Value> = serde_json::from_str(&text[s..=e])
        .map_err(|err| anyhow!("JSON respons AI tidak valid: {}", err))?;

    let mut picks: Vec<Option<(usize, EffectType)>> = vec![None; plan.cuts.len()];
    let mut used = 0usize;
    for it in &items {
        let (Some(cut), Some(panel)) = (it["cut"].as_u64(), it["panel"].as_u64()) else { continue };
        let (cut, panel) = (cut as usize, panel as usize);
        if cut == 0 || cut > picks.len() || panel == 0 || panel > plan.pool.len() {
            continue;
        }
        let Ok(effect) = serde_json::from_value::<EffectType>(it["effect"].clone()) else { continue };
        picks[cut - 1] = Some((panel - 1, effect));
        used += 1;
    }
    println!("[director] applied {}/{} AI decisions, rest local", used, picks.len());
    if used == 0 {
        return Err(anyhow!("AI tidak memberi keputusan yang valid."));
    }
    Ok(assemble(&plan, &picks))
}
