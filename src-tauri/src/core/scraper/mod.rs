use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, REFERER, USER_AGENT};
use scraper::{Html, Selector};
use std::fs;
use std::path::PathBuf;
use url::Url;

use crate::core::comic_slicer::slice_comic_images;
use crate::models::panel::ComicFormat;
use crate::models::scraper::{PlatformKind, PlatformPreset, ScrapeResult, ScraperRequest};

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
            id: PlatformKind::VoraToon,
            name: "VoraToon (voratoon.web.id)".to_string(),
            base_url: "https://voratoon.web.id".to_string(),
            example_url: "https://voratoon.web.id/".to_string(),
            default_format: ComicFormat::WebtoonVertical,
            image_selector: "#readerarea img, .reading-content img, .chapter-area img".to_string(),
            referer_header: "https://voratoon.web.id/".to_string(),
            description: "Preset khusus https://voratoon.web.id/ (mendukung ekstraksi #readerarea, lazy-src, & ts_reader JSON).".to_string(),
        },
        PlatformPreset {
            id: PlatformKind::Custom,
            name: "Website Komik Lainnya (Custom URL)".to_string(),
            base_url: "https://".to_string(),
            example_url: "https://voratoon.web.id/".to_string(),
            default_format: ComicFormat::WebtoonVertical,
            image_selector: "#readerarea img, .reading-content img, .chapter-content img, article img".to_string(),
            referer_header: "".to_string(),
            description: "Universal Scraper untuk website komik/manhwa/manga apapun yang diinput user.".to_string(),
        },
    ]
}

pub async fn scrape_and_slice_chapter(req: &ScraperRequest) -> Result<ScrapeResult> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .build()?;

    let parsed_url = Url::parse(&req.url)
        .map_err(|e| anyhow!("URL tidak valid ({}): {}", req.url, e))?;

    let default_origin = format!("{}://{}/", parsed_url.scheme(), parsed_url.host_str().unwrap_or("localhost"));
    let referer = req
        .custom_referer
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(default_origin);

    let (chapter_title, image_urls, source_label) = match req.platform {
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
                .unwrap_or("#readerarea img, .reading-content img, .chapter-area img, article img");
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
                .unwrap_or("#readerarea img, .reading-content img, .chapter-content img, #_imageList img, article img");
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

    // Download gambar ke folder cache sementara
    let cache_dir = std::env::temp_dir().join("comicut_cache").join("scraped_chapter");
    if cache_dir.exists() {
        let _ = fs::remove_dir_all(&cache_dir);
    }
    fs::create_dir_all(&cache_dir)?;

    let mut downloaded_paths: Vec<PathBuf> = Vec::new();
    for (idx, img_url) in image_urls.iter().take(45).enumerate() {
        let mut headers = HeaderMap::new();
        if let Ok(ref_val) = HeaderValue::from_str(&referer) {
            headers.insert(REFERER, ref_val);
        }
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/131.0.0.0"),
        );

        if let Ok(resp) = client.get(img_url).headers(headers).send().await {
            if resp.status().is_success() {
                if let Ok(bytes) = resp.bytes().await {
                    let ext = if img_url.contains(".png") {
                        "png"
                    } else if img_url.contains(".webp") {
                        "webp"
                    } else {
                        "jpg"
                    };
                    let file_path = cache_dir.join(format!("page_{:03}.{}", idx + 1, ext));
                    if fs::write(&file_path, &bytes).is_ok() {
                        downloaded_paths.push(file_path);
                    }
                }
            }
        }
    }

    let extracted_panels = slice_comic_images(&downloaded_paths, req.comic_format)?;
    let downloaded_str = downloaded_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    Ok(ScrapeResult {
        chapter_title,
        source_platform: source_label,
        source_url: req.url.clone(),
        downloaded_paths: downloaded_str,
        extracted_panels,
    })
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

    let html_text = client
        .get(page_url)
        .headers(headers)
        .send()
        .await?
        .text()
        .await?;

    let base = Url::parse(page_url)?;
    let mut image_urls: Vec<String> = Vec::new();
    let mut chapter_title = format!("{} Chapter", platform_name);

    // 1. Parse dokumen HTML dengan blok terpisah agar tidak melintasi await
    {
        let document = Html::parse_document(&html_text);

        if let Ok(title_sel) = Selector::parse("title") {
            if let Some(el) = document.select(&title_sel).next() {
                let t = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                if !t.is_empty() {
                    chapter_title = t;
                }
            }
        }

        // Coba ekstrak dari CSS selector yang dipilih (mendukung data-url, data-lazy-src, data-src, src)
        if let Ok(img_sel) = Selector::parse(css_selector) {
            for img in document.select(&img_sel) {
                let candidate = img
                    .value()
                    .attr("data-url")
                    .or_else(|| img.value().attr("data-lazy-src"))
                    .or_else(|| img.value().attr("data-src"))
                    .or_else(|| img.value().attr("src"));

                if let Some(raw_src) = candidate {
                    let trimmed = raw_src.trim();
                    if is_likely_comic_image(trimmed) {
                        if let Ok(resolved) = base.join(trimmed) {
                            let s = resolved.to_string();
                            if !image_urls.contains(&s) {
                                image_urls.push(s);
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Fallback khusus situs Manhwa/Manga berbasis Themesia / VoraToon (ts_reader.run JSON)
    if image_urls.is_empty() {
        if let Ok(re_ts) = Regex::new(r#"\"images\"\s*:\s*\[([^\]]+)\]"#) {
            if let Some(caps) = re_ts.captures(&html_text) {
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

    Ok((chapter_title, image_urls, platform_name.to_string()))
}

fn is_likely_comic_image(url: &str) -> bool {
    let lower = url.to_lowercase();
    if lower.is_empty() || lower.starts_with("data:image/svg") || lower.contains("logo") || lower.contains("avatar") {
        return false;
    }
    lower.contains(".jpg")
        || lower.contains(".jpeg")
        || lower.contains(".png")
        || lower.contains(".webp")
        || lower.contains("webtoon")
}
