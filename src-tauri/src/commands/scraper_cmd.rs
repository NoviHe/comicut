use serde::Deserialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::core::scraper::{
    build_browser_extract_script, default_platform_presets, scrape_and_slice_chapter,
    slice_extracted_chapter, SCRAPER_USER_AGENT,
};
use crate::models::panel::{ComicFormat, PanelStyle};
use crate::models::scraper::{PlatformPreset, ScrapeResult, ScraperRequest};

const CHAPTER_BROWSER_LABEL: &str = "chapter-browser";

#[derive(Deserialize)]
struct BrowserExtractPayload {
    ok: bool,
    error: String,
    title: String,
    url: String,
    images: Vec<String>,
}

#[tauri::command]
pub fn get_platform_presets() -> Vec<PlatformPreset> {
    default_platform_presets()
}

#[tauri::command]
pub async fn scrape_comic_chapter(request: ScraperRequest) -> Result<ScrapeResult, String> {
    scrape_and_slice_chapter(&request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_chapter_browser(app: AppHandle, url: String) -> Result<(), String> {
    let parsed = url::Url::parse(url.trim()).map_err(|e| format!("URL tidak valid: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("URL harus menggunakan protokol http:// atau https://.".to_string());
    }

    if let Some(win) = app.get_webview_window(CHAPTER_BROWSER_LABEL) {
        win.navigate(parsed).map_err(|e| e.to_string())?;
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }

    let app_handle = app.clone();
    let win = WebviewWindowBuilder::new(&app, CHAPTER_BROWSER_LABEL, WebviewUrl::External(parsed))
        .title("ComiCut — Browser Chapter (Selesaikan Verifikasi / Muat Gambar)")
        .inner_size(1100.0, 820.0)
        .center()
        .decorations(true)
        .user_agent(SCRAPER_USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;

    win.on_window_event(move |event| {
        if let WindowEvent::Destroyed = event {
            let _ = app_handle.emit("chapter-browser-closed", ());
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn close_chapter_browser(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(CHAPTER_BROWSER_LABEL) {
        win.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn scrape_from_chapter_browser(
    app: AppHandle,
    comic_format: ComicFormat,
    panel_style: PanelStyle,
    custom_selector: Option<String>,
    custom_referer: Option<String>,
) -> Result<ScrapeResult, String> {
    let win = app
        .get_webview_window(CHAPTER_BROWSER_LABEL)
        .ok_or_else(|| "Jendela browser chapter belum terbuka atau sudah ditutup.".to_string())?;

    let fallback_url = win.url().map(|u| u.to_string()).unwrap_or_default();
    let script = build_browser_extract_script(custom_selector.as_deref()).map_err(|e| e.to_string())?;

    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let tx_slot = Mutex::new(Some(tx));
    win.eval_with_callback(script, move |raw| {
        if let Ok(mut guard) = tx_slot.lock() {
            if let Some(sender) = guard.take() {
                let _ = sender.send(raw);
            }
        }
    })
    .map_err(|e| e.to_string())?;

    let raw_json = tokio::time::timeout(Duration::from_secs(10), rx)
        .await
        .map_err(|_| "Timeout membaca DOM dari jendela browser.".to_string())?
        .map_err(|_| "Gagal menerima hasil DOM dari jendela browser.".to_string())?;

    if raw_json.trim().is_empty() || raw_json.trim() == "null" {
        return Err("Halaman browser belum siap atau gagal membaca DOM.".to_string());
    }

    let payload: BrowserExtractPayload = serde_json::from_str(&raw_json)
        .map_err(|e| format!("Format respons browser tidak valid: {e}"))?;
    if !payload.ok {
        return Err(format!("Gagal membaca halaman browser: {}", payload.error));
    }

    let effective_url = if payload.url.trim().is_empty() {
        fallback_url
    } else {
        payload.url
    };

    let result = slice_extracted_chapter(
        payload.title,
        effective_url,
        payload.images,
        custom_referer,
        comic_format,
        panel_style,
    )
    .await
    .map_err(|e| e.to_string())?;

    let _ = win.close();
    Ok(result)
}
