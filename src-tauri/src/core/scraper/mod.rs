use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, REFERER, USER_AGENT};
use scraper::{Html, Selector};
use std::fs;
use std::path::PathBuf;
use url::Url;

use crate::core::comic_slicer::slice_comic_images;
use crate::models::panel::{ComicFormat, PanelStyle};
use crate::models::scraper::{PlatformKind, PlatformPreset, ScrapeResult, ScraperRequest};

pub const SCRAPER_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

pub const DEFAULT_CUSTOM_SELECTOR: &str =
    "#readerarea img, .reading-content img, .chapter-content img, img[class*=\"chapterPage\"], #_imageList img, article img";

pub fn default_platform_presets() -> Vec<PlatformPreset> {
    vec![
        PlatformPreset {
            id: PlatformKind::Webtoon,
            name: "LINE Webtoon".to_string(),
            base_url: "https://www.webtoons.com".to_string(),
            example_url: "https://www.webtoons.com/id/action/example/episode-1/viewer?title_no=1&episode_no=1".to_string(),
            default_format: ComicFormat::WebtoonVertical,
            image_selector: "#_imageList img._images, #_imageList img".to_string(),
            referer_header: "https://www.webtoons.com/".to_string(),
            description: "Adapter resmi LINE Webtoon (otomatis menggunakan data-url & Referer anti-403).".to_string(),
        },
        PlatformPreset {
            id: PlatformKind::MangaDex,
            name: "MangaDex (API v5)".to_string(),
            base_url: "https://mangadex.org".to_string(),
            example_url: "https://mangadex.org/chapter/801513ba-a712-498c-8f57-cae55b38cc92".to_string(),
            default_format: ComicFormat::MangaRTL,
            image_selector: "mangadex-at-home-api".to_string(),
            referer_header: "https://mangadex.org/".to_string(),
            description: "Mengambil gambar langsung dari endpoint MangaDex At-Home Server API.".to_string(),
        },
        PlatformPreset {
            id: PlatformKind::Custom,
            name: "Website Komik Lainnya (Custom URL)".to_string(),
            base_url: "https://".to_string(),
            example_url: "https://shinigami.id/series/...".to_string(),
            default_format: ComicFormat::WebtoonVertical,
            image_selector: "#readerarea img, .reading-content img, .chapter-content img, img[class*=\"chapterPage\"], article img".to_string(),
            referer_header: "".to_string(),
            description: "Universal Scraper untuk website komik/manhwa/manga apapun yang diinput user.".to_string(),
        },
    ]
}

pub async fn scrape_and_slice_chapter(req: &ScraperRequest) -> Result<ScrapeResult> {
    let client = reqwest::Client::builder()
        .user_agent(SCRAPER_USER_AGENT)
        .build()?;

    let parsed_url = Url::parse(&req.url)
        .map_err(|e| anyhow!("URL tidak valid ({}): {}", req.url, e))?;

    let referer = req
        .custom_referer
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default_referer_for(&parsed_url));

    println!(
        "[scraper] start platform={:?} url={} referer={} selector={:?}",
        req.platform, req.url, referer, req.custom_selector
    );

    let is_shinigami = parsed_url.host_str().is_some_and(|h| h.contains("shinigami"));
    let (chapter_title, image_urls, source_label) = match req.platform {
        _ if is_shinigami => fetch_shinigami_chapter(&client, &req.url).await?,
        PlatformKind::MangaDex => fetch_mangadex_chapter(&client, &req.url).await?,
        PlatformKind::Webtoon => {
            fetch_html_chapter_images(
                &client,
                &req.url,
                "#_imageList img._images, #_imageList img",
                "https://www.webtoons.com/",
                "LINE Webtoon",
            )
            .await?
        }
        PlatformKind::VoraToon => {
            let selector = req
                .custom_selector
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(DEFAULT_CUSTOM_SELECTOR);
            fetch_html_chapter_images(
                &client,
                &req.url,
                selector,
                "https://voratoon.web.id/",
                "VoraToon (voratoon.web.id)",
            )
            .await?
        }
        PlatformKind::Custom => {
            let selector = req
                .custom_selector
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(DEFAULT_CUSTOM_SELECTOR);
            fetch_html_chapter_images(
                &client,
                &req.url,
                selector,
                &referer,
                parsed_url.host_str().unwrap_or("Custom Website"),
            )
            .await?
        }
    };

    if image_urls.is_empty() {
        return Err(anyhow!(
            "Tidak ditemukan gambar chapter pada URL {}. Pastikan URL mengarah langsung ke halaman baca chapter.",
            req.url
        ));
    }

    download_and_slice_images(
        &client,
        chapter_title,
        source_label,
        req.url.clone(),
        image_urls,
        &referer,
        req.comic_format,
        req.panel_style,
    )
    .await
}

/// Downloads and slices chapter image URLs already extracted from a user-visible browser WebView.
pub async fn slice_extracted_chapter(
    chapter_title: String,
    source_url: String,
    raw_image_urls: Vec<String>,
    custom_referer: Option<String>,
    comic_format: ComicFormat,
    panel_style: PanelStyle,
) -> Result<ScrapeResult> {
    let parsed_url = Url::parse(&source_url)
        .map_err(|e| anyhow!("URL browser tidak valid ({}): {}", source_url, e))?;
    let image_urls = filter_browser_extracted_urls(raw_image_urls, &source_url);
    if image_urls.is_empty() {
        return Err(anyhow!(
            "Belum ditemukan gambar chapter yang valid di jendela browser. Pastikan verifikasi selesai dan gambar chapter sudah muncul di layar, lalu coba lagi."
        ));
    }

    let referer = custom_referer
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default_referer_for(&parsed_url));
    let source_label = format!(
        "{} (Browser)",
        parsed_url.host_str().unwrap_or("Browser")
    );
    let title = if chapter_title.trim().is_empty() {
        format!("{} Chapter", parsed_url.host_str().unwrap_or("Chapter"))
    } else {
        chapter_title.trim().to_string()
    };

    let client = reqwest::Client::builder()
        .user_agent(SCRAPER_USER_AGENT)
        .build()?;

    download_and_slice_images(
        &client,
        title,
        source_label,
        source_url,
        image_urls,
        &referer,
        comic_format,
        panel_style,
    )
    .await
}

fn default_referer_for(parsed_url: &Url) -> String {
    format!(
        "{}://{}/",
        parsed_url.scheme(),
        parsed_url.host_str().unwrap_or("localhost")
    )
}

async fn download_and_slice_images(
    client: &reqwest::Client,
    chapter_title: String,
    source_label: String,
    source_url: String,
    image_urls: Vec<String>,
    referer: &str,
    comic_format: ComicFormat,
    panel_style: PanelStyle,
) -> Result<ScrapeResult> {
    println!(
        "[scraper] title={:?} source={} images_found={}",
        chapter_title,
        source_label,
        image_urls.len()
    );
    for (i, u) in image_urls.iter().enumerate() {
        println!("[scraper]   img {:03}: {}", i + 1, u);
    }

    let cache_dir = std::env::temp_dir()
        .join("comicut_cache")
        .join("scraped_chapter");
    if cache_dir.exists() {
        let _ = fs::remove_dir_all(&cache_dir);
    }
    fs::create_dir_all(&cache_dir)?;

    let mut downloaded_paths: Vec<PathBuf> = Vec::new();
    for (idx, img_url) in image_urls.iter().take(45).enumerate() {
        if let Some(bytes) = fetch_image_bytes(client, img_url, referer, idx + 1).await {
            let ext = image_extension_for_url(img_url);
            let file_path = cache_dir.join(format!("page_{:03}.{}", idx + 1, ext));
            if fs::write(&file_path, &bytes).is_ok() {
                downloaded_paths.push(file_path);
            }
        }
    }

    if downloaded_paths.is_empty() {
        return Err(anyhow!(
            "Menemukan {} URL gambar, tetapi gagal mengunduh gambar dari CDN (kemungkinan butuh cookie sesi browser). Simpan halaman gambar dari browser lalu gunakan Folder Lokal.",
            image_urls.len()
        ));
    }

    println!(
        "[scraper] downloaded {}/{} pages, slicing",
        downloaded_paths.len(),
        image_urls.len().min(45)
    );
    let extracted_panels = slice_comic_images(&downloaded_paths, comic_format, panel_style)?;
    let downloaded_str = downloaded_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    println!("[scraper] done panels={}", extracted_panels.len());
    Ok(ScrapeResult {
        chapter_title,
        source_platform: source_label,
        source_url,
        downloaded_paths: downloaded_str,
        extracted_panels,
    })
}

async fn fetch_image_bytes(
    client: &reqwest::Client,
    img_url: &str,
    referer: &str,
    index: usize,
) -> Option<Vec<u8>> {
    // Try with Referer first; if a CDN rejects cross-origin Referer (e.g. no-referrer S3 URLs), retry without Referer.
    for send_referer in [true, false] {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(SCRAPER_USER_AGENT));
        if send_referer && !referer.is_empty() {
            if let Ok(ref_val) = HeaderValue::from_str(referer) {
                headers.insert(REFERER, ref_val);
            }
        }

        match client.get(img_url).headers(headers).send().await {
            Err(e) => {
                println!("[scraper] download {:03} FAILED: {} ({})", index, e, img_url);
                return None;
            }
            Ok(resp) if resp.status().is_success() => {
                if let Ok(bytes) = resp.bytes().await {
                    println!("[scraper] download {:03} ok {} bytes", index, bytes.len());
                    return Some(bytes.to_vec());
                }
                return None;
            }
            Ok(resp) => {
                let code = resp.status().as_u16();
                println!(
                    "[scraper] download {:03} HTTP {} (referer={}) ({})",
                    index,
                    resp.status(),
                    send_referer,
                    img_url
                );
                if !(send_referer && matches!(code, 400 | 401 | 403)) {
                    return None;
                }
            }
        }
    }
    None
}

fn image_extension_for_url(img_url: &str) -> &'static str {
    let clean = img_url.split(['?', '#']).next().unwrap_or(img_url).to_lowercase();
    if clean.ends_with(".png") || clean.contains(".png") {
        "png"
    } else if clean.ends_with(".webp") || clean.contains(".webp") {
        "webp"
    } else {
        "jpg"
    }
}

/// Filters and resolves raw image URLs extracted from the interactive browser DOM.
pub fn filter_browser_extracted_urls(raw_urls: Vec<String>, base_url: &str) -> Vec<String> {
    let Ok(base) = Url::parse(base_url) else {
        return Vec::new();
    };
    let mut urls = Vec::new();
    for raw in raw_urls {
        let trimmed = raw.trim();
        if !is_likely_comic_image(trimmed) {
            continue;
        }
        if let Ok(resolved) = base.join(trimmed) {
            if matches!(resolved.scheme(), "http" | "https") {
                let s = resolved.to_string();
                if !urls.contains(&s) {
                    urls.push(s);
                }
            }
        }
    }
    urls
}

/// Builds a self-contained JS snippet that extracts rendered chapter image URLs from the active page DOM.
pub fn build_browser_extract_script(custom_selector: Option<&str>) -> Result<String> {
    let selector = custom_selector
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_CUSTOM_SELECTOR);
    let selector_json = serde_json::to_string(selector)?;
    Ok(format!(
        r#"(() => {{
  try {{
    const selector = {selector_json};
    const isNonComicEl = (img) => {{
      if (img.closest('header, nav, footer, aside')) return true;
      const meta = ((img.className || '') + ' ' + (img.id || '')).toLowerCase();
      if (['thumbnail', 'avatar', 'logo', 'favicon', 'banner'].some((k) => meta.includes(k))) return true;
      const alt = (img.alt || '').trim().toLowerCase();
      return alt === 'cover' || alt.startsWith('cover ');
    }};
    const pickSrc = (img) =>
      (img.currentSrc || img.getAttribute('data-url') || img.getAttribute('data-lazy-src') || img.getAttribute('data-src') || img.getAttribute('src') || '').trim();
    const collect = (els) => {{
      const out = [];
      const seen = new Set();
      for (const img of els) {{
        if (isNonComicEl(img)) continue;
        if (img.complete && img.naturalWidth > 0 && (img.naturalWidth < 150 || img.naturalHeight < 150)) continue;
        const src = pickSrc(img);
        if (!src || seen.has(src)) continue;
        seen.add(src);
        out.push(src);
      }}
      return out;
    }};
    let urls = [];
    if (selector) {{
      try {{
        urls = collect(document.querySelectorAll(selector));
      }} catch (_) {{}}
    }}
    if (urls.length === 0) {{
      urls = collect(document.querySelectorAll('img'));
    }}
    return {{
      ok: true,
      error: '',
      title: document.title || '',
      url: window.location.href || '',
      images: urls
    }};
  }} catch (e) {{
    return {{ ok: false, error: String(e), title: '', url: '', images: [] }};
  }}
}})()"#
    ))
}

async fn fetch_mangadex_chapter(
    client: &reqwest::Client,
    chapter_url: &str,
) -> Result<(String, Vec<String>, String)> {
    // Ekstrak UUID chapter dari URL https://mangadex.org/chapter/<uuid>
    let re = Regex::new(r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})")?;
    let chapter_id = re
        .captures(chapter_url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow!("UUID Chapter MangaDex tidak ditemukan pada URL."))?;

    let api_url = format!("https://api.mangadex.org/at-home/server/{}", chapter_id);
    println!("[scraper] mangadex api GET {}", api_url);
    let json: serde_json::Value = client.get(&api_url).send().await?.json().await?;

    let base_url = json["baseUrl"]
        .as_str()
        .ok_or_else(|| anyhow!("Respons MangaDex tidak memiliki baseUrl"))?;
    let hash = json["chapter"]["hash"]
        .as_str()
        .ok_or_else(|| anyhow!("Respons MangaDex tidak memiliki chapter hash"))?;

    let mut urls = Vec::new();
    if let Some(arr) = json["chapter"]["data"].as_array() {
        for item in arr {
            if let Some(filename) = item.as_str() {
                urls.push(format!("{}/data/{}/{}", base_url, hash, filename));
            }
        }
    }

    Ok((
        format!("MangaDex Chapter {}", &chapter_id[..8]),
        urls,
        "MangaDex API v5".to_string(),
    ))
}

// Shinigami = SvelteKit SPA, HTML kosong; gambar dari JSON API.
async fn fetch_shinigami_chapter(
    client: &reqwest::Client,
    chapter_url: &str,
) -> Result<(String, Vec<String>, String)> {
    let re = Regex::new(r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})")?;
    let id = re
        .captures(chapter_url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow!("UUID chapter Shinigami tidak ditemukan pada URL."))?;

    let api = format!("https://api.shngm.io/v1/chapter/detail/{}", id);
    println!("[scraper] shinigami api GET {}", api);
    let resp = client.get(&api).send().await?;
    println!("[scraper] shinigami api status={}", resp.status());
    let json: serde_json::Value = resp.json().await?;

    let d = &json["data"];
    let base = d["base_url"].as_str().unwrap_or("https://assets.shngm.id");
    let path = d["chapter"]["path"]
        .as_str()
        .ok_or_else(|| anyhow!("Respons Shinigami tanpa chapter.path: {}", json["message"]))?;
    let urls: Vec<String> = d["chapter"]["data"]
        .as_array()
        .map(|a| a.iter().filter_map(|f| f.as_str()).map(|f| format!("{}{}{}", base, path, f)).collect())
        .unwrap_or_default();

    Ok((
        format!("Shinigami Chapter {}", d["chapter_number"]),
        urls,
        "Shinigami".to_string(),
    ))
}

async fn fetch_html_chapter_images(
    client: &reqwest::Client,
    page_url: &str,
    css_selector: &str,
    referer: &str,
    platform_name: &str,
) -> Result<(String, Vec<String>, String)> {
    let mut headers = HeaderMap::new();
    if let Ok(r) = HeaderValue::from_str(referer) {
        headers.insert(REFERER, r);
    }

    println!("[scraper] html GET {} selector={:?}", page_url, css_selector);
    // Sites behind Cloudflare return transient 5xx (e.g. 503 / error 1102); retry a few times.
    let mut html_text = String::new();
    for attempt in 1..=4u64 {
        let resp = client.get(page_url).headers(headers.clone()).send().await?;
        let status = resp.status();
        println!("[scraper] html status={} (attempt {})", status, attempt);
        if status.is_success() {
            html_text = resp.text().await?;
            break;
        }
        if attempt == 4 || !(status.is_server_error() || status.as_u16() == 429) {
            return Err(anyhow!("Situs membalas HTTP {} untuk {}", status, page_url));
        }
        tokio::time::sleep(std::time::Duration::from_millis(800 * attempt)).await;
    }
    println!("[scraper] html bytes={}", html_text.len());
    extract_chapter_images_from_html(&html_text, page_url, css_selector, platform_name)
}

/// Extracts comic page image URLs from fetched HTML, filtering out non-chapter assets
/// and surfacing clear guidance when the page is Turnstile-gated or client-rendered.
fn extract_chapter_images_from_html(
    html_text: &str,
    page_url: &str,
    css_selector: &str,
    platform_name: &str,
) -> Result<(String, Vec<String>, String)> {
    let base = Url::parse(page_url)?;
    let document = Html::parse_document(html_text);
    let mut image_urls: Vec<String> = Vec::new();
    let mut chapter_title = format!("{} Chapter", platform_name);

    if let Ok(title_sel) = Selector::parse("title") {
        if let Some(el) = document.select(&title_sel).next() {
            let t = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if !t.is_empty() {
                chapter_title = t;
            }
        }
    }

    collect_images_for_selector(&document, css_selector, &base, false, &mut image_urls);
    println!("[scraper] selector matched {} images", image_urls.len());

    // 1b. Selector found nothing: take every <img> that looks like a comic page.
    if image_urls.is_empty() {
        collect_images_for_selector(&document, "img", &base, true, &mut image_urls);
        println!("[scraper] generic <img> fallback found {} images", image_urls.len());
    }

    // 2. Fallback khusus situs Manhwa/Manga berbasis Themesia / VoraToon (ts_reader.run JSON)
    if image_urls.is_empty() {
        if let Ok(re_ts) = Regex::new(r#"\"images\"\s*:\s*\[([^\]]+)\]"#) {
            if let Some(caps) = re_ts.captures(html_text) {
                if let Some(list_match) = caps.get(1) {
                    if let Ok(re_url) = Regex::new(r#"\"(https?:[^\"]+)\""#) {
                        for u_cap in re_url.captures_iter(list_match.as_str()) {
                            if let Some(m) = u_cap.get(1) {
                                let clean_url = m.as_str().replace("\\/", "/");
                                if is_likely_comic_image(&clean_url) && !image_urls.contains(&clean_url) {
                                    image_urls.push(clean_url);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if image_urls.is_empty() {
        if let Some(reason) = detect_unscrapable_html_reason(html_text) {
            return Err(anyhow!(reason));
        }
    }

    Ok((chapter_title, image_urls, platform_name.to_string()))
}

fn collect_images_for_selector(
    document: &Html,
    css_selector: &str,
    base: &Url,
    skip_non_comic_elements: bool,
    image_urls: &mut Vec<String>,
) {
    let Ok(img_sel) = Selector::parse(css_selector) else {
        return;
    };
    for img in document.select(&img_sel) {
        if skip_non_comic_elements && is_non_comic_element(&img) {
            continue;
        }
        let candidate = img
            .value()
            .attr("data-url")
            .or_else(|| img.value().attr("data-lazy-src"))
            .or_else(|| img.value().attr("data-src"))
            .or_else(|| img.value().attr("src"));

        if let Some(raw_src) = candidate.map(str::trim) {
            if is_likely_comic_image(raw_src) {
                if let Ok(resolved) = base.join(raw_src) {
                    let s = resolved.to_string();
                    if !image_urls.contains(&s) {
                        image_urls.push(s);
                    }
                }
            }
        }
    }
}

fn is_non_comic_element(img: &scraper::ElementRef) -> bool {
    let val = img.value();
    for attr in ["class", "id"] {
        if let Some(v) = val.attr(attr) {
            let lower = v.to_lowercase();
            if ["thumbnail", "avatar", "logo", "favicon", "banner"]
                .iter()
                .any(|kw| lower.contains(kw))
            {
                return true;
            }
        }
    }
    if let Some(alt) = val.attr("alt") {
        let lower_alt = alt.trim().to_lowercase();
        if lower_alt.starts_with("cover ") || lower_alt == "cover" {
            return true;
        }
    }
    false
}

fn detect_unscrapable_html_reason(html_text: &str) -> Option<&'static str> {
    let lower = html_text.to_lowercase();
    let has_turnstile = [
        "reader-turnstile_widget",
        "challenges.cloudflare.com/turnstile",
        "cf-turnstile",
        "cf-browser-verification",
        "__cf_chl_",
        "just a moment...",
    ]
    .iter()
    .any(|m| lower.contains(m));

    if has_turnstile {
        return Some(
            "Situs memakai proteksi Cloudflare Turnstile / verifikasi browser sehingga gambar chapter tidak bisa diambil langsung. Buka chapter di browser, simpan gambarnya ke folder, lalu gunakan mode Folder Lokal.",
        );
    }

    let has_spa_shell = lower.contains("menyiapkan halaman chapter")
        || lower.contains("self.__next_f.push")
        || lower.contains("__next_data__");
    if has_spa_shell {
        return Some(
            "Halaman ini dimuat secara dinamis lewat JavaScript (SPA) dan tidak berisi URL gambar chapter di HTML awal. Simpan gambar dari browser lalu gunakan mode Folder Lokal.",
        );
    }

    None
}

fn is_likely_comic_image(url: &str) -> bool {
    let lower = url.trim().to_lowercase();
    if lower.is_empty() || lower.starts_with("data:image/svg") {
        return false;
    }

    let has_blocked_keyword = ["logo", "avatar", "thumbnail", "histats", "favicon", "apple-icon"]
        .iter()
        .any(|kw| lower.contains(kw));
    if has_blocked_keyword {
        return false;
    }

    let path_part = lower.split(['?', '#']).next().unwrap_or("");
    let has_blocked_segment = path_part.split('/').any(|seg| {
        matches!(
            seg,
            "cover"
                | "covers"
                | "background"
                | "backgrounds"
                | "ads"
                | "banner"
                | "banners"
                | "placeholder"
                | "placeholders"
                | "icon"
                | "icons"
        )
    });
    let file_name = path_part.rsplit('/').next().unwrap_or("");
    if has_blocked_segment || file_name.starts_with("icon.") {
        return false;
    }

    lower.contains(".jpg")
        || lower.contains(".jpeg")
        || lower.contains(".png")
        || lower.contains(".webp")
        || lower.contains("webtoon")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_non_comic_urls_without_breaking_chapter_paths() {
        assert!(!is_likely_comic_image(
            "https://cvr.voratoon.id/prod/series/became-the-patron-of-villains/cover/CoverPelindung.webp?X-Amz-Algorithm=AWS4"
        ));
        assert!(!is_likely_comic_image(
            "https://cvr.voratoon.id/prod/series/became-the-patron-of-villains/background/patron.webp"
        ));
        assert!(!is_likely_comic_image(
            "https://cvr.voratoon.id/prod/ads/header-r4b.webp"
        ));
        assert!(!is_likely_comic_image("/icon.png"));
        assert!(!is_likely_comic_image("/placeholders/avatar.svg"));
        assert!(!is_likely_comic_image("//s4is.histats.com/stats/i/5044842.gif"));

        assert!(is_likely_comic_image(
            "https://cdn.example.com/uploads/series/undercover-hero/ch61/001.webp"
        ));
        assert!(is_likely_comic_image(
            "https://webtoon-phinf.pstatic.net/20230101_1/page.jpg?type=opt"
        ));
    }

    #[test]
    fn rejects_voratoon_turnstile_shell_instead_of_returning_cover() {
        let html = r#"
        <html><head><title>Chapter 61 | VoraToon</title></head>
        <body>
            <main class="reader-screen_page__9kJUJ">
                <div class="reader-turnstile_widget__Ge0Gw" hidden=""></div>
                <img alt="Cover Became The Patron Of Villains" class="reader-screen_seriesThumbnail__c9A_t" src="https://cvr.voratoon.id/prod/series/became-the-patron-of-villains/cover/CoverPelindung.webp?sig=1" />
                <img alt="Avatar Anda" src="/placeholders/avatar.svg" />
                <img alt="" src="//s4is.histats.com/stats/i/5044842.gif" />
                <section class="reader-screen_readerContent__xm8kN">
                    <p role="status">Menyiapkan halaman chapter…</p>
                </section>
            </main>
            <script>self.__next_f.push([1,"shell"])</script>
        </body></html>"#;

        let err = extract_chapter_images_from_html(
            html,
            "https://13.voratoon.com/series/became-the-patron-of-villains/chapter/61",
            DEFAULT_CUSTOM_SELECTOR,
            "13.voratoon.com",
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Turnstile"), "unexpected error: {msg}");
        assert!(msg.contains("Folder Lokal"), "unexpected error: {msg}");
    }

    #[test]
    fn rejects_spa_shell_without_turnstile_when_no_images_found() {
        let html = r#"
        <html><head><title>Chapter 1</title></head>
        <body>
            <div id="__next"></div>
            <script>self.__next_f.push([1,"app"])</script>
        </body></html>"#;

        let err = extract_chapter_images_from_html(
            html,
            "https://example.com/series/foo/chapter/1",
            DEFAULT_CUSTOM_SELECTOR,
            "example.com",
        )
        .unwrap_err();
        assert!(err.to_string().contains("JavaScript (SPA)"));
    }

    #[test]
    fn keeps_valid_static_chapter_images_even_with_cover_or_turnstile_script() {
        let html = r#"
        <html><head><title>Chapter 12</title>
        <script src="https://challenges.cloudflare.com/turnstile/v0/api.js"></script>
        </head>
        <body>
            <div id="readerarea">
                <img data-src="https://cdn.example.com/ch12/01.webp" />
                <img src="https://cdn.example.com/ch12/02.jpg" />
            </div>
        </body></html>"#;

        let (title, urls, _) = extract_chapter_images_from_html(
            html,
            "https://example.com/ch-12",
            DEFAULT_CUSTOM_SELECTOR,
            "example.com",
        )
        .unwrap();
        assert_eq!(title, "Chapter 12");
        assert_eq!(
            urls,
            vec![
                "https://cdn.example.com/ch12/01.webp",
                "https://cdn.example.com/ch12/02.jpg",
            ]
        );
    }

    #[test]
    fn filters_browser_urls_and_rejects_blob_or_cover() {
        let filtered = filter_browser_extracted_urls(
            vec![
                "https://cvr.voratoon.id/prod/series/foo/cover/Cover.webp".to_string(),
                "blob:https://13.voratoon.com/12345.webp".to_string(),
                "/chapters/61/01.webp".to_string(),
                "https://cvr.voratoon.id/prod/series/foo/ch61/02.webp?sig=1".to_string(),
                "https://cvr.voratoon.id/prod/series/foo/ch61/02.webp?sig=1".to_string(),
            ],
            "https://13.voratoon.com/series/foo/chapter/61",
        );
        assert_eq!(
            filtered,
            vec![
                "https://13.voratoon.com/chapters/61/01.webp",
                "https://cvr.voratoon.id/prod/series/foo/ch61/02.webp?sig=1",
            ]
        );
    }

    #[test]
    fn builds_browser_extract_script_with_json_escaped_selector() {
        let script = build_browser_extract_script(Some("img[class*=\"chapter\"]")).unwrap();
        assert!(script.contains(r#""img[class*=\"chapter\"]""#));
        assert!(script.contains("document.querySelectorAll"));
    }
}
