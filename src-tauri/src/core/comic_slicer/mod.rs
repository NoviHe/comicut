pub mod credit;
pub mod heuristics;
pub mod manga;
pub mod webtoon;

use anyhow::Result;
use std::path::PathBuf;

use crate::models::panel::{ComicFormat, Panel};

pub fn slice_comic_images(image_paths: &[PathBuf], format: ComicFormat) -> Result<Vec<Panel>> {
    let credit = credit::detect_credit_pages(image_paths);
    match format {
        ComicFormat::WebtoonVertical => webtoon::slice_vertical_webtoon(image_paths, &credit),
        ComicFormat::MangaRTL => manga::slice_manga_pages(image_paths, true, &credit),
        ComicFormat::MangaLTR => manga::slice_manga_pages(image_paths, false, &credit),
    }
}
