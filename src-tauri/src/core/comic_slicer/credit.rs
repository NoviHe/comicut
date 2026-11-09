use std::path::PathBuf;

/// Scanlator credit / ad / "join our discord" pages are shipped as extra images at the start and end of a
/// chapter, in a different size than the story pages. Flag first/last pages whose width or aspect ratio
/// deviates from the chapter's dominant page size.
/// shortcut: size heuristic only, mid-chapter ads and credit pages that match the story size are not caught;
/// the AI reviewer prompt covers those.
pub fn detect_credit_pages(paths: &[PathBuf]) -> Vec<bool> {
    let dims: Vec<(u32, u32)> = paths
        .iter()
        .map(|p| image::image_dimensions(p).unwrap_or((0, 0)))
        .collect();
    let flags = credit_flags(&dims);
    for (i, f) in flags.iter().enumerate() {
        if *f {
            println!("[slicer] page {} ({}x{}) looks like a credit/ad page, skipped", i + 1, dims[i].0, dims[i].1);
        }
    }
    flags
}

const EDGE: usize = 3;

pub fn credit_flags(dims: &[(u32, u32)]) -> Vec<bool> {
    let n = dims.len();
    let mut out = vec![false; n];
    let valid: Vec<(u32, u32)> = dims.iter().copied().filter(|d| d.0 > 0 && d.1 > 0).collect();
    if valid.len() < 2 * EDGE {
        return out; // too few pages to know what "normal" is
    }

    // Dominant width = width with the most pages within 8%.
    let near = |a: u32, b: u32| (a as f32 - b as f32).abs() <= 0.08 * b as f32;
    let (mode_w, support) = valid
        .iter()
        .map(|d| (d.0, valid.iter().filter(|e| near(e.0, d.0)).count()))
        .max_by_key(|&(w, c)| (c, w))
        .unwrap();
    if support * 2 < valid.len() {
        return out; // no dominant size
    }

    let mut aspects: Vec<f32> = valid.iter().map(|d| d.0 as f32 / d.1 as f32).collect();
    aspects.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = aspects[aspects.len() / 2];

    for (i, &(w, h)) in dims.iter().enumerate() {
        if !(i < EDGE || i >= n.saturating_sub(EDGE)) || w == 0 || h == 0 {
            continue;
        }
        let ratio = (w as f32 / h as f32) / med;
        out[i] = !near(w, mode_w) || ratio > 1.6 || ratio < 0.625;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_edge_ads_only() {
        // Real layout of voratoon solo-leveling ch.172: 3 ad pages, 40 story strips, 1 ad page.
        let mut d = vec![(1920, 1219), (904, 1280), (1080, 1072)];
        d.extend(std::iter::repeat((720, 3600)).take(40));
        d.push((1280, 1024));
        let f = credit_flags(&d);
        assert_eq!(f.iter().filter(|x| **x).count(), 4);
        assert!(f[0] && f[1] && f[2] && f[43]);
        assert!(!f[3] && !f[42]);
    }

    #[test]
    fn uniform_chapter_has_none() {
        let d = vec![(800, 1200); 12];
        assert!(credit_flags(&d).iter().all(|x| !x));
    }

    #[test]
    fn tiny_chapter_untouched() {
        assert!(credit_flags(&[(100, 100), (900, 900)]).iter().all(|x| !x));
    }
}
