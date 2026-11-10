pub mod commands;
pub mod core;
pub mod models;

use commands::{
    ai_cmd::{review_epic_panels, suggest_songs},
    audio_cmd::{analyze_audio_beats, download_audio},
    comic_cmd::import_local_chapter,
    deps_cmd::{check_dependency_updates, install_dependency},
    render_cmd::{ai_direct_timeline, generate_auto_timeline, render_montagem_video},
    scraper_cmd::{get_platform_presets, scrape_comic_chapter},
    system_cmd::{
        app_restart, check_dependencies, get_device_diagnostics, show_in_folder, window_close,
        window_minimize, window_toggle_maximize,
    },
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // App-managed tools (FFmpeg / yt-dlp / Deno installed from the UI) take precedence on PATH.
    commands::deps_cmd::init_managed_path();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_platform_presets,
            scrape_comic_chapter,
            import_local_chapter,
            review_epic_panels,
            analyze_audio_beats,
            download_audio,
            generate_auto_timeline,
            render_montagem_video,
            ai_direct_timeline,
            suggest_songs,
            check_dependencies,
            check_dependency_updates,
            install_dependency,
            get_device_diagnostics,
            show_in_folder,
            app_restart,
            window_minimize,
            window_toggle_maximize,
            window_close
        ])
        .run(tauri::generate_context!())
        .expect("error while running ComiCut Tauri application");
}
