use std::fs;
use std::path::PathBuf;

use crate::core::comic_slicer::slice_comic_images;
use crate::models::panel::{ComicFormat, Panel, PanelStyle};

#[tauri::command]
pub async fn import_local_chapter(
    folder_path: String,
    comic_format: ComicFormat,
    panel_style: Option<PanelStyle>,
) -> Result<Vec<Panel>, String> {
    let entries = fs::read_dir(&folder_path)
        .map_err(|e| format!("Gagal membaca folder {}: {}", folder_path, e))?;

    let mut image_paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_lowercase().as_str(),
                        "jpg" | "jpeg" | "png" | "webp"
                    )
                })
                .unwrap_or(false)
        })
        .collect();

    image_paths.sort();

    if image_paths.is_empty() {
        return Err(format!(
            "Tidak ditemukan file gambar (.jpg/.png/.webp) di dalam folder {}",
            folder_path
        ));
    }

    slice_comic_images(&image_paths, comic_format, panel_style.unwrap_or_default())
        .map_err(|e| e.to_string())
}
