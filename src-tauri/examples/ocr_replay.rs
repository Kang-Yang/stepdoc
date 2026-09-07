//! Dev probe: replay saved OCR debug steps through the real pipeline (RapidOCR + click-proximity
//! scoring + click-kind classification) to verify labels against actual recordings.
//!
//! Saved crops are centered on the click, so the replay reconstructs each crop's origin around a
//! virtual click point — identical relative geometry to the original recording.
//!
//! Usage: cargo run --example ocr_replay -- <step-dir> [more-step-dirs ...]
//! e.g.   cargo run --example ocr_replay -- "$LOCALAPPDATA/stepdoc/ocr-debug/<session>/step-03"

use std::path::{Path, PathBuf};

use image::GenericImageView;
use stepdoc_lib::ocr::{
    build_click_description, recognize_click_label_from_crops_with_report, OcrCaptureContext,
};

/// Virtual screenshot-space click coordinate; crop origins are mirrored around it.
const BASE: u32 = 4096;

fn main() {
    let dirs: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if dirs.is_empty() {
        eprintln!("usage: ocr_replay <step-dir> [...]");
        std::process::exit(2);
    }

    for dir in &dirs {
        replay_step(dir);
    }
}

fn replay_step(dir: &Path) {
    let mut entries: Vec<(u32, String, PathBuf)> = Vec::new();
    let read = match std::fs::read_dir(dir) {
        Ok(read) => read,
        Err(error) => {
            println!("=== {} ===\n  cannot read dir: {error}", dir.display());
            return;
        }
    };
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some((index, label)) = parse_crop_name(&name) else {
            continue;
        };
        entries.push((index, label, entry.path()));
    }
    entries.sort_by_key(|(index, _, _)| *index);

    let mut crops = Vec::new();
    let mut labels = Vec::new();
    let mut origins = Vec::new();
    for (_, label, path) in &entries {
        let image = image::open(path).expect("open crop");
        let (w, h) = image.dimensions();
        origins.push((BASE - w / 2, BASE - h / 2));
        labels.push(label.clone());
        crops.push(image.to_rgba8());
    }
    if crops.is_empty() {
        println!("=== {} ===\n  no *-crop.png found", dir.display());
        return;
    }

    let context = OcrCaptureContext {
        region_labels: labels,
        crop_origins: origins,
        click_point: Some((BASE as f64, BASE as f64)),
        capture_source: Some("replay".to_string()),
    };
    let report = recognize_click_label_from_crops_with_report(&crops, &context, false);

    println!("=== {} ===", dir.display());
    println!(
        "  engine={} kind={} picked={:?} -> {}",
        report.engine,
        report.click_kind,
        report.picked_label,
        build_click_description("left-click", report.picked_label.as_deref(), &report.click_kind)
    );
    for region in &report.regions {
        println!("  [{}] picked_in_region={:?}", region.region, region.picked_in_region);
        let mut candidates = region.candidates.clone();
        candidates.sort_by(|a, b| b.score.cmp(&a.score));
        for candidate in candidates.iter().take(5) {
            let click_d = if candidate.click_distance.is_finite() {
                format!("{:.0}", candidate.click_distance.sqrt())
            } else {
                "-".to_string()
            };
            println!(
                "    {:<26} src={:<7} q={:<4} score={:<5} click_d={:<6} conf={:<5} accepted={:<5} rect={:?}",
                truncate(&candidate.text, 26),
                candidate.source,
                candidate.quality,
                candidate.score,
                click_d,
                candidate
                    .ocr_confidence
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "-".to_string()),
                candidate.accepted,
                candidate.rect.map(|(x, y, w, h)| (
                    x as i32, y as i32, w as i32, h as i32
                )),
            );
        }
    }
}

/// `01-center-crop.png` -> (1, "center")
fn parse_crop_name(name: &str) -> Option<(u32, String)> {
    let stem = name.strip_suffix("-crop.png")?;
    let (index, label) = stem.split_once('-')?;
    let index = index.parse().ok()?;
    Some((index, label.to_string()))
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max).collect();
    format!("{cut}…")
}
