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
                .unwrap_or("#readerarea img, .reading-content img, .chapter-area img, img[class*=\"chapterPage\"], article img");
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
                .unwrap_or("#readerarea img, .reading-content img, .chapter-content img, img[class*=\"chapterPage\"], #_imageList img, article img");
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

    println!("[scraper] title={:?} source={} images_found={}", chapter_title, source_label, image_urls.len());
    for (i, u) in image_urls.iter().enumerate() {
        println!("[scraper]   img {:03}: {}", i + 1, u);
    }

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

        match client.get(img_url).headers(headers).send().await {
            Err(e) => println!("[scraper] download {:03} FAILED: {} ({})", idx + 1, e, img_url),
            Ok(resp) if !resp.status().is_success() => {
                println!("[scraper] download {:03} HTTP {} ({})", idx + 1, resp.status(), img_url)
            }
            Ok(resp) => {
                if let Ok(bytes) = resp.bytes().await {
                    println!("[scraper] download {:03} ok {} bytes", idx + 1, bytes.len());
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

    println!("[scraper] downloaded {}/{} pages, slicing", downloaded_paths.len(), image_urls.len().min(45));
    let extracted_panels = slice_comic_images(&downloaded_paths, req.comic_format)?;
    let downloaded_str = downloaded_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    println!("[scraper] done panels={}", extracted_panels.len());
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

    println!("[scraper] selector matched {} images", image_urls.len());

    // 1b. Selector found nothing: take every <img> that looks like a comic page.
    if image_urls.is_empty() {
        let document = Html::parse_document(&html_text);
        if let Ok(any_img) = Selector::parse("img") {
            for img in document.select(&any_img) {
                let candidate = img
                    .value()
                    .attr("data-url")
                    .or_else(|| img.value().attr("data-lazy-src"))
                    .or_else(|| img.value().attr("data-src"))
                    .or_else(|| img.value().attr("src"));
                if let Some(raw) = candidate.map(str::trim) {
                    if is_likely_comic_image(raw) {
                        if let Ok(u) = base.join(raw) {
                            let s = u.to_string();
                            if !image_urls.contains(&s) {
                                image_urls.push(s);
                            }
                        }
                    }
                }
            }
        }
        println!("[scraper] generic <img> fallback found {} images", image_urls.len());
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
    if lower.is_empty() || lower.starts_with("data:image/svg") || lower.contains("logo") || lower.contains("avatar") || lower.contains("thumbnail") || lower.contains("histats") || lower.contains("favicon") {
        return false;
    }
    lower.contains(".jpg")
        || lower.contains(".jpeg")
        || lower.contains(".png")
        || lower.contains(".webp")
        || lower.contains("webtoon")
}
