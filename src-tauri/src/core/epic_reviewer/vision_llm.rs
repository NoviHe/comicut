use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::io::Cursor;
use std::time::Duration;

use crate::models::panel::{Panel, PanelRole, VisionReviewConfig};

// Request size ceiling: one thumbnail per panel.
const MAX_LLM_PANELS: usize = 40;

const PROMPT: &str = "You pick the most epic comic panels for a fast-cut montage video (Reels/Shorts). \
Panels follow, each preceded by its number. Rate every panel. Reply with ONLY a JSON array, no prose: \
[{\"panel\":1,\"score\":0.0-1.0,\"role\":\"Hook|BuildUp|AuraCloseup|Climax|VerticalPan|Filler\"}]. \
Score high for action, dramatic poses, power reveals, striking composition; low for plain dialogue or empty frames. \
Give score 0 and role Filler to scanlator credit pages, site ads, watermarks, 'join our discord/app' banners and translator/editor credits.";

pub async fn score_and_rank_panels(
    mut panels: Vec<Panel>,
    config: &VisionReviewConfig,
) -> Result<Vec<Panel>> {
    if panels.is_empty() {
        return Ok(panels);
    }
    heuristic_rank(&mut panels, config);
    if matches!(config.provider.as_str(), "OpenAI" | "Anthropic") {
        llm_rerank(&mut panels, config).await?;
        // The model scores credit pages / site ads / info text 0 + Filler: drop them from the list entirely.
        let before = panels.len();
        panels.retain(|p| !(p.role == PanelRole::Filler && p.ai_epic_score.is_some_and(|s| s <= 0.02)));
        for (i, p) in panels.iter_mut().enumerate() {
            p.index = i;
        }
        println!("[ai] removed {} credit/ad/info panels", before - panels.len());
    }
    Ok(panels)
}

fn heuristic_rank(panels: &mut [Panel], config: &VisionReviewConfig) {
    let n = panels.len();
    let score = |p: &Panel| (p.heuristic_score + if p.is_vertical_pan { 0.06 } else { 0.0 }).clamp(0.05, 0.99);

    // Rank by score; pick top N, but skip a candidate scoring within 0.02 of an
    // already-picked immediate neighbour (near-duplicate consecutive panels).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| score(&panels[*b]).partial_cmp(&score(&panels[*a])).unwrap_or(std::cmp::Ordering::Equal));
    let max_select = config.max_panels_to_select.max(1).min(order.len());
    let mut picked: Vec<usize> = Vec::new();
    for &i in &order {
        if picked.len() >= max_select {
            break;
        }
        let dup = picked.iter().any(|&j| (j as i64 - i as i64).abs() == 1 && (score(&panels[i]) - score(&panels[j])).abs() < 0.02);
        if !dup {
            picked.push(i);
        }
    }
    // Backfill if dedupe left room.
    for &i in &order {
        if picked.len() >= max_select {
            break;
        }
        if !picked.contains(&i) {
            picked.push(i);
        }
    }
    let top: std::collections::HashSet<usize> = picked.iter().copied().collect();

    // Roles in reading order among picked panels: first = Hook, best = Climax,
    // long strips = VerticalPan, rest split BuildUp (before climax) / AuraCloseup (after).
    let mut reading: Vec<usize> = picked.clone();
    reading.sort();
    let climax = picked.iter().copied().max_by(|a, b| score(&panels[*a]).partial_cmp(&score(&panels[*b])).unwrap_or(std::cmp::Ordering::Equal));
    let hook = reading.first().copied();

    for (idx, panel) in panels.iter_mut().enumerate() {
        panel.selected = top.contains(&idx);
        // Spread raw scores into a readable 0.15..0.99 band by rank percentile blend.
        let rank = order.iter().position(|&o| o == idx).unwrap_or(n) as f32;
        let pct = 1.0 - rank / (order.len().max(2) - 1) as f32;
        panel.ai_epic_score = Some((0.55 * score(panel) + 0.45 * (0.2 + 0.79 * pct)).clamp(0.15, 0.99));

        panel.role = if !panel.selected {
            PanelRole::Filler
        } else if Some(idx) == hook {
            PanelRole::Hook
        } else if Some(idx) == climax {
            PanelRole::Climax
        } else if panel.is_vertical_pan {
            PanelRole::VerticalPan
        } else if climax.is_some_and(|c| idx < c) {
            PanelRole::BuildUp
        } else {
            PanelRole::AuraCloseup
        };
    }
}

async fn llm_rerank(panels: &mut [Panel], config: &VisionReviewConfig) -> Result<()> {
    let mut images: Vec<(String, String)> = Vec::new();
    for (i, p) in panels.iter().enumerate().take(MAX_LLM_PANELS) {
        let Ok(b64) = thumb_b64(&p.image_path, 512) else { continue };
        images.push((format!("Panel {}:", i + 1), b64));
    }
    if images.is_empty() {
        return Err(anyhow!("Tidak ada file gambar panel yang bisa dibaca untuk dikirim ke AI."));
    }

    println!(
        "[ai] provider={} model={} base={:?} panels_sent={}",
        config.provider, config.model_name, config.base_url, images.len()
    );
    let text = ask(config, PROMPT, &images).await?;
    println!("[ai] response: {}", text);

    let (s, e) = (text.find('['), text.rfind(']'));
    let (Some(s), Some(e)) = (s, e) else {
        return Err(anyhow!(
            "Respons AI bukan JSON array: {}",
            text.chars().take(200).collect::<String>()
        ));
    };
    let verdicts: Vec<Value> = serde_json::from_str(&text[s..=e])
        .map_err(|err| anyhow!("JSON respons AI tidak valid: {}", err))?;

    for v in &verdicts {
        let Some(n) = v["panel"].as_u64().filter(|n| *n >= 1) else { continue };
        let Some(p) = panels.get_mut(n as usize - 1) else { continue };
        if let Some(sc) = v["score"].as_f64() {
            p.ai_epic_score = Some((sc as f32).clamp(0.0, 1.0));
        }
        if let Ok(role) = serde_json::from_value::<PanelRole>(v["role"].clone()) {
            p.role = role;
        }
    }

    let score = |p: &Panel| p.ai_epic_score.unwrap_or(p.heuristic_score);
    let mut order: Vec<usize> = (0..panels.len()).collect();
    order.sort_by(|a, b| {
        score(&panels[*b])
            .partial_cmp(&score(&panels[*a]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top: std::collections::HashSet<usize> =
        order.into_iter().take(config.max_panels_to_select.max(1)).collect();
    for (i, p) in panels.iter_mut().enumerate() {
        p.selected = top.contains(&i);
    }
    Ok(())
}

/// JPEG thumbnail (longest side `max`) as base64.
pub(crate) fn thumb_b64(path: &str, max: u32) -> Result<String> {
    let img = image::open(path)?;
    let mut buf = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img.thumbnail(max, max).to_rgb8())
        .write_to(&mut buf, image::ImageFormat::Jpeg)?;
    Ok(STANDARD.encode(buf.into_inner()))
}

/// One LLM call: `prompt` plus labelled images, via the configured OpenAI- or Anthropic-compatible endpoint.
pub(crate) async fn ask(
    config: &VisionReviewConfig,
    prompt: &str,
    images: &[(String, String)],
) -> Result<String> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(180)).build()?;
    if config.provider == "Anthropic" {
        call_anthropic(&client, config, prompt, images).await
    } else {
        call_openai(&client, config, prompt, images).await
    }
}

fn api_key(config: &VisionReviewConfig) -> Option<&str> {
    config.api_key.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

fn base_url<'a>(config: &'a VisionReviewConfig, default: &'a str) -> &'a str {
    config
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .trim_end_matches('/')
}

async fn send(req: reqwest::RequestBuilder) -> Result<Value> {
    let resp = req.send().await?;
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        return Err(anyhow!(
            "AI API HTTP {}: {}",
            status,
            body.chars().take(300).collect::<String>()
        ));
    }
    Ok(serde_json::from_str(&body)?)
}

// OpenAI-compatible: OpenAI, OpenRouter, Ollama (/v1), LM Studio, Gemini (/v1beta/openai), etc.
async fn call_openai(
    client: &reqwest::Client,
    config: &VisionReviewConfig,
    prompt: &str,
    images: &[(String, String)],
) -> Result<String> {
    let base = base_url(config, "https://api.openai.com/v1");
    let url = if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{}/chat/completions", base)
    };
    let mut content = vec![json!({"type": "text", "text": prompt})];
    for (label, b64) in images {
        content.push(json!({"type": "text", "text": label}));
        content.push(json!({
            "type": "image_url",
            "image_url": {"url": format!("data:image/jpeg;base64,{}", b64)}
        }));
    }
    let mut req = client.post(&url).json(&json!({
        "model": config.model_name,
        "messages": [{"role": "user", "content": content}],
    }));
    if let Some(k) = api_key(config) {
        req = req.bearer_auth(k);
    }
    let v = send(req).await?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow!("Respons OpenAI-compatible tanpa choices[0].message.content"))
}

// Anthropic-compatible: Anthropic, or any proxy exposing /v1/messages.
async fn call_anthropic(
    client: &reqwest::Client,
    config: &VisionReviewConfig,
    prompt: &str,
    images: &[(String, String)],
) -> Result<String> {
    let base = base_url(config, "https://api.anthropic.com");
    let url = if base.ends_with("/messages") {
        base.to_string()
    } else if base.ends_with("/v1") {
        format!("{}/messages", base)
    } else {
        format!("{}/v1/messages", base)
    };
    let mut content = vec![json!({"type": "text", "text": prompt})];
    for (label, b64) in images {
        content.push(json!({"type": "text", "text": label}));
        content.push(json!({
            "type": "image",
            "source": {"type": "base64", "media_type": "image/jpeg", "data": b64}
        }));
    }
    let mut req = client
        .post(&url)
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": config.model_name,
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": content}],
        }));
    if let Some(k) = api_key(config) {
        // Official API uses x-api-key; many proxies expect Bearer. Send both.
        req = req.header("x-api-key", k).bearer_auth(k);
    }
    let v = send(req).await?;
    let text: String = v["content"]
        .as_array()
        .map(|a| a.iter().filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join(""))
        .unwrap_or_default();
    if text.is_empty() {
        return Err(anyhow!("Respons Anthropic-compatible tanpa content[].text"));
    }
    Ok(text)
}

const SONG_PROMPT: &str = "These are key panels from a comic chapter that will become a fast-cut 'montagem' video \
(Reels/Shorts). Judge the overall mood and energy (e.g. brutal action, dark/ominous, triumphant, tense, comedic). \
Suggest 4 YouTube search queries for existing montagem / phonk / brazilian funk tracks that fit this mood, most fitting first. \
Prefer tracks with a clear bass drop and a length between 30 and 180 seconds. \
Reply with ONLY a JSON array of 4 strings, e.g. [\"Montagem Coral phonk\", \"Brazilian phonk aura\"].";

/// AI looks at the best panels and proposes search queries for a fitting song.
pub async fn suggest_song_queries(panels: &[Panel], config: &VisionReviewConfig) -> Result<Vec<String>> {
    let score = |p: &Panel| p.ai_epic_score.unwrap_or(p.heuristic_score);
    let mut ranked: Vec<&Panel> = panels.iter().collect();
    ranked.sort_by(|a, b| score(b).partial_cmp(&score(a)).unwrap_or(std::cmp::Ordering::Equal));
    let mut images: Vec<(String, String)> = Vec::new();
    for (i, p) in ranked.iter().take(8).enumerate() {
        if let Ok(b64) = thumb_b64(&p.image_path, 384) {
            images.push((format!("Panel {}:", i + 1), b64));
        }
    }
    if images.is_empty() {
        return Err(anyhow!("Tidak ada gambar panel yang bisa dikirim ke AI."));
    }
    println!("[song] AI mood call: {} panels, model={}", images.len(), config.model_name);
    let text = ask(config, SONG_PROMPT, &images).await?;
    println!("[song] response: {}", text);
    let (Some(s), Some(e)) = (text.find('['), text.rfind(']')) else {
        return Err(anyhow!("Respons AI bukan JSON array: {}", text.chars().take(200).collect::<String>()));
    };
    let list: Vec<String> = serde_json::from_str(&text[s..=e]).map_err(|err| anyhow!("JSON respons AI tidak valid: {}", err))?;
    // Trust boundary: the text goes to yt-dlp as a search, never as an option.
    let queries: Vec<String> = list
        .into_iter()
        .map(|q| q.trim().to_string())
        .filter(|q| (3..=80).contains(&q.chars().count()) && !q.starts_with('-'))
        .take(4)
        .collect();
    if queries.is_empty() {
        return Err(anyhow!("AI tidak memberi saran lagu yang valid."));
    }
    Ok(queries)
}
