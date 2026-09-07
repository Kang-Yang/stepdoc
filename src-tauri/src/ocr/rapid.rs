use std::sync::{Mutex, OnceLock};
use std::thread;

use image::{DynamicImage, GenericImageView, RgbaImage};
use rapidocr_core::config::PipelineConfig;
use rapidocr_core::model::{ModelCache, ModelDownloadMode, PPOCRV5_CH_MOBILE};
use rapidocr_core::types::Quad;
use rapidocr_core::RapidOcr;

use crate::utils::{ocr_frames_dir, ocr_models_dir};

use crate::ocr::pipeline::{pick_best_label_from_regions, scan_crop_regions};
use crate::ocr::region::prepare_ocr_input;
use crate::ocr::report::{OcrCandidateReport, OcrPipelineReport, OcrRegionReport};
use crate::ocr::text_pick::{
    candidate_pick_score, is_plausible_ui_label, label_quality_score, normalize_label,
    reject_reason, truncate_label,
};
use crate::screenshot::HEURISTIC_REGION_LABELS;

const ENGINE_NAME: &str = "rapidocr-ppocrv5-ch";
const ENGINE_UNAVAILABLE: &str = "rapidocr-unavailable";

pub fn warmup() {
    thread::spawn(|| {
        let _ = ensure_engine();
    });
}

pub fn recognize_label_from_crops_with_report(
    crops: &[RgbaImage],
    context: &crate::ocr::OcrCaptureContext,
    full_scan: bool,
) -> OcrPipelineReport {
    if let Err(error) = ensure_engine() {
        return OcrPipelineReport {
            engine: ENGINE_UNAVAILABLE.to_string(),
            picked_label: None,
            capture_source: None,
            click_kind: "text".to_string(),
            regions: crops
                .iter()
                .enumerate()
                .map(|(index, crop)| OcrRegionReport {
                    region: context
                        .region_labels
                        .get(index)
                        .cloned()
                        .or_else(|| {
                            HEURISTIC_REGION_LABELS
                                .get(index)
                                .map(|label| (*label).to_string())
                        })
                        .unwrap_or_else(|| "region".to_string()),
                    crop_size: format!("{}x{}", crop.width(), crop.height()),
                    upscaled_size: "0x0".to_string(),
                    raw_lines: Vec::new(),
                    raw_words: Vec::new(),
                    picked_in_region: None,
                    candidates: Vec::new(),
                    ocr_error: Some(error.clone()),
                })
                .collect(),
        };
    }

        // Region-independent distance reference: the centre crop's height (72 design px scaled by
    // the display factor). Every region must normalize absolute click distances identically.
    let reference = crops
        .first()
        .map(|crop| crop.height() as f32)
        .unwrap_or(72.0);

    let regions = scan_crop_regions(
        crops,
        &context.region_labels,
        &context.crop_origins,
        context.click_point,
        full_scan,
        // Distances normalize against the centre crop's height so every region scores the same
        // absolute distance identically.
        |region, crop, origin, click_point| {
            analyze_region(region, crop, origin, click_point, reference)
        },
    );

    let picked_label = pick_best_label_from_regions(&regions);

    OcrPipelineReport {
        engine: ENGINE_NAME.to_string(),
        picked_label,
        capture_source: None,
        click_kind: "text".to_string(),
        regions,
    }
}

fn analyze_region(
    region: &str,
    crop: &RgbaImage,
    origin: Option<(u32, u32)>,
    click_point: Option<(f64, f64)>,
    reference: f32,
) -> OcrRegionReport {
    let input = prepare_ocr_input(crop);
    let crop_size = format!("{}x{}", crop.width(), crop.height());
    let upscaled_size = format!("{}x{}", input.width(), input.height());

    match run_ocr(&input) {
        Ok(lines) => {
            let (picked, candidates, raw_lines, raw_words) =
                inspect_lines(&lines, input.dimensions(), origin, click_point, reference);
            OcrRegionReport {
                region: region.to_string(),
                crop_size,
                upscaled_size,
                raw_lines,
                raw_words,
                picked_in_region: picked,
                candidates,
                ocr_error: None,
            }
        }
        Err(error) => OcrRegionReport {
            region: region.to_string(),
            crop_size,
            upscaled_size,
            raw_lines: Vec::new(),
            raw_words: Vec::new(),
            picked_in_region: None,
            candidates: Vec::new(),
            ocr_error: Some(error),
        },
    }
}

struct RapidLine {
    text: String,
    rect: (f32, f32, f32, f32),
    confidence: f32,
}

fn inspect_lines(
    lines: &[RapidLine],
    (image_w, image_h): (u32, u32),
    origin: Option<(u32, u32)>,
    click_point: Option<(f64, f64)>,
    reference: f32,
) -> (
    Option<String>,
    Vec<OcrCandidateReport>,
    Vec<String>,
    Vec<String>,
) {
    let center_x = image_w as f32 / 2.0;
    let center_y = image_h as f32 / 2.0;
    let mut best: Option<(i32, String)> = None;
    let mut candidates = Vec::new();
    let mut raw_lines = Vec::new();
    let mut raw_words = Vec::new();

    for line in lines {
        let text = normalize_label(&line.text);
        if text.is_empty() {
            continue;
        }
        raw_lines.push(text.clone());
        raw_words.push(text.clone());
        record_candidate(
            &mut candidates,
            &mut best,
            text,
            "line",
            Some(line.rect),
            Some(line.confidence),
            center_x,
            center_y,
            origin,
            click_point,
            reference,
        );

        // A wide multi-part line is usually several controls OCR'd as one strip (tab bar,
        // button row). Emit per-segment candidates with proportional sub-boxes so the label
        // under the click can win over the whole strip. Narrow lines keep a single label —
        // splitting those would butcher multi-word labels like "Save As".
        if line.rect.2 >= reference * 2.0 {
            for (segment, segment_rect) in line_segments(&line.text, line.rect) {
                record_candidate(
                    &mut candidates,
                    &mut best,
                    segment,
                    "segment",
                    Some(segment_rect),
                    Some(line.confidence),
                    center_x,
                    center_y,
                    origin,
                    click_point,
                    reference,
                );
            }
        }
    }

    let picked = best
        .map(|(_, text)| truncate_label(&text, 24))
        .filter(|text| is_plausible_ui_label(text));
    (picked, candidates, raw_lines, raw_words)
}

/// Split a raw OCR line into whitespace-separated segments with estimated crop-local sub-rects.
/// Widths are distributed proportionally to rendered character width (CJK ≈ 2 units).
fn line_segments(raw: &str, rect: (f32, f32, f32, f32)) -> Vec<(String, (f32, f32, f32, f32))> {
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() < 2 {
        return Vec::new();
    }

    let units: Vec<f32> = parts
        .iter()
        .map(|part| {
            part.chars()
                .map(|ch| if is_wide_char(ch) { 2.0 } else { 1.0 })
                .sum()
        })
        .collect();
    let total_units: f32 = units.iter().sum();
    if total_units <= 0.0 {
        return Vec::new();
    }

    let mut segments = Vec::with_capacity(parts.len());
    let mut offset = rect.0;
    for (part, unit) in parts.iter().zip(units) {
        let width = rect.2 * (unit / total_units);
        let text = normalize_label(part);
        if !text.is_empty() {
            segments.push((text, (offset, rect.1, width, rect.3)));
        }
        offset += width;
    }
    segments
}

fn is_wide_char(ch: char) -> bool {
    matches!(ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x3000..=0x303F | 0xFF00..=0xFFEF)
}

fn record_candidate(
    candidates: &mut Vec<OcrCandidateReport>,
    best: &mut Option<(i32, String)>,
    text: String,
    source: &str,
    rect: Option<(f32, f32, f32, f32)>,
    ocr_confidence: Option<f32>,
    center_x: f32,
    center_y: f32,
    origin: Option<(u32, u32)>,
    click_point: Option<(f64, f64)>,
    reference: f32,
) {
    let mut reject_reason = reject_reason(&text).map(str::to_string);
    // Very low recognition confidence is OCR debris even when the text looks plausible.
    if reject_reason.is_none() && ocr_confidence.is_some_and(|confidence| confidence < 0.6) {
        reject_reason = Some("low_confidence".to_string());
    }
    let accepted = reject_reason.is_none();
    let quality = label_quality_score(&text);
    let distance = rect
        .map(|bounds| center_distance(bounds, center_x, center_y))
        .unwrap_or(f32::MAX);
    // Distance from the real click point matters more than local closeness inside a crop.
    let click_distance = click_distance_sq(rect, origin, click_point);
    let score = if accepted {
        candidate_pick_score(&text, source, click_distance, reference)
    } else {
        i32::MIN / 4
    };

    candidates.push(OcrCandidateReport {
        text: text.clone(),
        source: source.to_string(),
        quality,
        distance,
        click_distance,
        rect,
        score,
        ocr_confidence,
        accepted,
        reject_reason,
    });

    if !accepted {
        return;
    }
    match best {
        Some((best_score, _)) if score <= *best_score => {}
        _ => *best = Some((score, text)),
    }
}

/// Squared pixel distance from a line centre (crop-local, mapped into screenshot space by
/// `origin`) to the real click point. Falls back to a large value when unavailable.
fn click_distance_sq(
    rect: Option<(f32, f32, f32, f32)>,
    origin: Option<(u32, u32)>,
    click_point: Option<(f64, f64)>,
) -> f32 {
    let (Some(rect), Some((ox, oy)), Some((cx, cy))) = (rect, origin, click_point) else {
        return f32::MAX;
    };
    let px = rect.0 + rect.2 / 2.0 + ox as f32;
    let py = rect.1 + rect.3 / 2.0 + oy as f32;
    let dx = px - cx as f32;
    let dy = py - cy as f32;
    dx * dx + dy * dy
}

fn center_distance((x, y, width, height): (f32, f32, f32, f32), cx: f32, cy: f32) -> f32 {
    let px = x + width / 2.0;
    let py = y + height / 2.0;
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy
}

fn quad_rect(quad: &Quad) -> (f32, f32, f32, f32) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for [x, y] in quad.points {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    (
        min_x,
        min_y,
        (max_x - min_x).max(1.0),
        (max_y - min_y).max(1.0),
    )
}

fn run_ocr(image: &DynamicImage) -> Result<Vec<RapidLine>, String> {
    let dir = ocr_frames_dir();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join(format!("{}.png", uuid::Uuid::new_v4()));
    image.save(&path).map_err(|error| error.to_string())?;
    let output = with_engine(|engine| engine.run_path(&path).map_err(|error| error.to_string()));
    let _ = std::fs::remove_file(&path);
    let output = output?;
    Ok(output
        .lines
        .into_iter()
        .map(|line| RapidLine {
            text: line.text,
            rect: quad_rect(&line.bbox),
            confidence: line.score,
        })
        .collect())
}

fn with_engine<T>(task: impl FnOnce(&mut RapidOcr) -> Result<T, String>) -> Result<T, String> {
    let engine = ENGINE
        .get()
        .ok_or_else(|| "RapidOCR 尚未初始化".to_string())?;
    let mut guard = engine.lock().map_err(|error| error.to_string())?;
    task(&mut guard)
}

static ENGINE: OnceLock<Mutex<RapidOcr>> = OnceLock::new();
static INIT_ERROR: OnceLock<String> = OnceLock::new();

fn ensure_engine() -> Result<(), String> {
    if let Some(error) = INIT_ERROR.get() {
        return Err(error.clone());
    }
    if ENGINE.get().is_some() {
        return Ok(());
    }

    match init_engine() {
        Ok(engine) => {
            let _ = ENGINE.set(Mutex::new(engine));
            Ok(())
        }
        Err(error) => {
            let _ = INIT_ERROR.set(error.clone());
            Err(error)
        }
    }
}

fn init_engine() -> Result<RapidOcr, String> {
    let model_dir = ocr_models_dir();
    std::fs::create_dir_all(&model_dir).map_err(|error| error.to_string())?;

    let cache = ModelCache::new(&model_dir);
    // Visual click crops can include neighboring text. Detect text first so recognition receives
    // individual lines rather than a guessed strip.
    let pipeline = PipelineConfig {
        use_det: true,
        use_cls: false,
        use_rec: true,
    };
    cache
        .ensure_model_set_for_pipeline(&PPOCRV5_CH_MOBILE, pipeline, ModelDownloadMode::Missing)
        .map_err(|error| format!("下载 RapidOCR 模型失败: {error}"))?;

    let cfg = cache.config_for(&PPOCRV5_CH_MOBILE).with_pipeline(pipeline);

    RapidOcr::new(cfg).map_err(|error| format!("初始化 RapidOCR 失败: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, rect: (f32, f32, f32, f32), confidence: f32) -> RapidLine {
        RapidLine {
            text: text.to_string(),
            rect,
            confidence,
        }
    }

    /// A wide merged strip (tab bar / button row) must yield the segment under the click, not
    /// the whole strip: clicking「查看」describes 查看, not 主页共享查看.
    #[test]
    fn wide_merged_line_picks_the_segment_under_the_click() {
        let lines = vec![line("主页 共享 查看", (10.0, 10.0, 180.0, 20.0), 0.95)];
        let (picked, candidates, _, _) =
            inspect_lines(&lines, (240, 72), Some((0, 0)), Some((160.0, 20.0)), 72.0);

        assert_eq!(picked.as_deref(), Some("查看"));
        assert!(
            candidates
                .iter()
                .any(|c| c.source == "segment" && c.text == "查看")
        );
    }

    /// Narrow multi-word labels ("Save As") are single controls — no segment split.
    #[test]
    fn narrow_multi_word_line_stays_one_label() {
        let lines = vec![line("Save As", (60.0, 26.0, 70.0, 18.0), 0.99)];
        let (picked, candidates, _, _) =
            inspect_lines(&lines, (240, 72), Some((0, 0)), Some((110.0, 35.0)), 72.0);

        assert_eq!(picked.as_deref(), Some("Save As"));
        assert!(!candidates.iter().any(|c| c.source == "segment"));
    }

    /// Sub-0.6 confidence lines are OCR debris even when the text looks plausible.
    #[test]
    fn low_confidence_lines_are_rejected() {
        let lines = vec![line("选超语", (139.0, 57.0, 52.0, 14.0), 0.55)];
        let (picked, candidates, _, _) =
            inspect_lines(&lines, (240, 72), Some((0, 0)), Some((120.0, 35.0)), 72.0);

        assert_eq!(picked, None);
        assert!(candidates.iter().all(|candidate| !candidate.accepted));
    }
}
