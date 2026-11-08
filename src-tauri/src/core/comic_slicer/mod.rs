pub mod heuristics;
pub mod manga;
pub mod webtoon;

use anyhow::Result;
use std::path::PathBuf;

use crate::models::panel::{ComicFormat, Panel};

pub fn slice_comic_images(image_paths: &[PathBuf], format: ComicFormat) -> Result<Vec<Panel>> {
    match format {
        ComicFormat::WebtoonVertical => webtoon::slice_vertical_webtoon(image_paths),
        ComicFormat::MangaRTL => manga::slice_manga_pages(image_paths, true),
        ComicFormat::MangaLTR => manga::slice_manga_pages(image_paths, false),
    }
}
