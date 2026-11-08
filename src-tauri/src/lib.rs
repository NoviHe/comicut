pub mod commands;
pub mod core;
pub mod models;

use commands::{
    ai_cmd::review_epic_panels,
    audio_cmd::analyze_audio_beats,
    comic_cmd::import_local_chapter,
    render_cmd::{generate_auto_timeline, render_montagem_video},
    scraper_cmd::{get_platform_presets, scrape_comic_chapter},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_platform_presets,
            scrape_comic_chapter,
            import_local_chapter,
            review_epic_panels,
            analyze_audio_beats,
            generate_auto_timeline,
            render_montagem_video
        ])
        .run(tauri::generate_context!())
        .expect("error while running ComiCut Tauri application");
}
