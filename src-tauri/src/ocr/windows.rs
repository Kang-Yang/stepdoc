use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::{OcrEngine, OcrLine, OcrWord};
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

use crate::ocr::pipeline::{pick_best_label_from_regions, scan_crop_regions};
use crate::ocr::region::prepare_ocr_input;
use crate::ocr::report::{OcrCandidateReport, OcrPipelineReport, OcrRegionReport};
use crate::ocr::text_pick::{
    candidate_pick_score, is_plausible_ui_label, label_quality_score, normalize_label,
    reject_reason, truncate_label,
};

pub fn recognize_label_from_crops_with_report(
    crops: &[RgbaImage],
    context: &crate::ocr::OcrCaptureContext,
    full_scan: bool,
) -> OcrPipelineReport {
    let engine = engine_language_tag();
    // 与区域无关的距离参考：中心裁剪的高度（参见 rapid.rs）。
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
        |region, crop, origin, click_point| {
            let ocr_input = prepare_ocr_input(crop);
            let (_, region_report) =
                analyze_region(region, crop, origin, click_point, reference, &ocr_input);
            region_report
        },
    );

    let picked_label = pick_best_label_from_regions(&regions);

    OcrPipelineReport {
        engine,
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
    upscaled: &DynamicImage,
) -> (Option<String>, OcrRegionReport) {
    let crop_size = format!("{}x{}", crop.width(), crop.height());
    let upscaled_size = format!("{}x{}", upscaled.width(), upscaled.height());

    let Some(result) = run_ocr(upscaled) else {
        return (
            None,
            OcrRegionReport {
                region: region.to_string(),
                crop_size,
                upscaled_size,
                raw_lines: Vec::new(),
                raw_words: Vec::new(),
                picked_in_region: None,
                candidates: Vec::new(),
                ocr_error: Some("Windows OCR 识别失败".to_string()),
            },
        );
    };

    let (picked, candidates, raw_lines, raw_words) =
        inspect_ocr_result(&result, upscaled.dimensions(), origin, click_point, reference);

    let picked_in_region = picked.clone();

    (
        picked,
        OcrRegionReport {
            region: region.to_string(),
            crop_size,
            upscaled_size,
            raw_lines,
            raw_words,
            picked_in_region,
            candidates,
            ocr_error: None,
        },
    )
}

fn inspect_ocr_result(
    result: &windows::Media::Ocr::OcrResult,
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
    let lines = result.Lines().ok();

    let mut best: Option<(i32, String)> = None;
    let mut candidates = Vec::new();
    let mut raw_lines = Vec::new();
    let mut raw_words = Vec::new();

    let Some(lines) = lines else {
        return (None, candidates, raw_lines, raw_words);
    };

    for line in iterate_lines(&lines) {
        let line_text = line
            .Text()
            .ok()
            .map(|value| normalize_label(&value.to_string()))
            .unwrap_or_default();
        if !line_text.is_empty() {
            raw_lines.push(line_text.clone());
        }
        record_candidate(
            &mut candidates,
            &mut best,
            line_text,
            "line",
            line_bounding_rect(&line).ok(),
            None,
            center_x,
            center_y,
            reference,
            origin,
            click_point,
        );

        let Ok(words) = line.Words() else {
            continue;
        };
        for word in iterate_words(&words) {
            let text = word
                .Text()
                .ok()
                .map(|value| normalize_label(&value.to_string()))
                .unwrap_or_default();
            if !text.is_empty() {
                raw_words.push(text.clone());
            }
            let Ok(rect) = word.BoundingRect() else {
                continue;
            };
            record_candidate(
                &mut candidates,
                &mut best,
                text,
                "word",
                Some((rect.X, rect.Y, rect.Width, rect.Height)),
                None,
                center_x,
                center_y,
                reference,
                origin,
                click_point,
            );
        }
    }

    let picked = best.map(|(_, text)| truncate_label(&text, 24));
    let picked = picked.filter(|text| is_plausible_ui_label(text));
    (picked, candidates, raw_lines, raw_words)
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
    reference: f32,
    origin: Option<(u32, u32)>,
    click_point: Option<(f64, f64)>,
) {
    if text.is_empty() {
        return;
    }

    let mut reject_reason = reject_reason(&text).map(str::to_string);
    // 即使文本看起来合理，极低的识别置信度也属于 OCR 碎片噪声。
    if reject_reason.is_none() && ocr_confidence.is_some_and(|confidence| confidence < 0.6) {
        reject_reason = Some("low_confidence".to_string());
    }
    let accepted = reject_reason.is_none();
    let quality = label_quality_score(&text);
    let distance = rect
        .map(|bounds| center_distance(bounds, center_x, center_y))
        .unwrap_or(f32::MAX);
    // 与真实点击点的距离比裁剪内部的局部接近程度更重要。
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

/// 从行中心（裁剪局部坐标，经 `origin` 映射到截图空间）到真实点击点的平方像素距离。
/// 不可用时回退到一个大值。
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

fn run_ocr(image: &DynamicImage) -> Option<windows::Media::Ocr::OcrResult> {
    let mut png_bytes = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
        .ok()?;

    let stream = InMemoryRandomAccessStream::new().ok()?;
    let writer = DataWriter::CreateDataWriter(&stream).ok()?;
    writer.WriteBytes(&png_bytes).ok()?;
    writer.StoreAsync().ok()?.get().ok()?;
    writer.FlushAsync().ok()?.get().ok()?;
    stream.Seek(0).ok()?;

    let decoder = BitmapDecoder::CreateAsync(&stream).ok()?.get().ok()?;
    let bitmap = decoder.GetSoftwareBitmapAsync().ok()?.get().ok()?;
    let engine = create_ocr_engine()?;
    engine.RecognizeAsync(&bitmap).ok()?.get().ok()
}

fn engine_language_tag() -> String {
    for tag in ["zh-Hans-CN", "zh-CN", "zh-Hans", "zh-TW", "zh"] {
        if let Ok(language) = Language::CreateLanguage(&HSTRING::from(tag)) {
            if OcrEngine::TryCreateFromLanguage(&language).is_ok() {
                return tag.to_string();
            }
        }
    }

    "user-profile".to_string()
}

fn create_ocr_engine() -> Option<OcrEngine> {
    for tag in ["zh-Hans-CN", "zh-CN", "zh-Hans", "zh-TW", "zh"] {
        let language = Language::CreateLanguage(&HSTRING::from(tag)).ok()?;
        if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&language) {
            return Some(engine);
        }
    }

    OcrEngine::TryCreateFromUserProfileLanguages().ok()
}

fn iterate_lines(lines: &windows::Foundation::Collections::IVectorView<OcrLine>) -> Vec<OcrLine> {
    let Ok(count) = lines.Size() else {
        return Vec::new();
    };

    let mut items = Vec::with_capacity(count as usize);
    for index in 0..count {
        if let Ok(line) = lines.GetAt(index) {
            items.push(line);
        }
    }
    items
}

fn iterate_words(words: &windows::Foundation::Collections::IVectorView<OcrWord>) -> Vec<OcrWord> {
    let Ok(count) = words.Size() else {
        return Vec::new();
    };

    let mut items = Vec::with_capacity(count as usize);
    for index in 0..count {
        if let Ok(word) = words.GetAt(index) {
            items.push(word);
        }
    }
    items
}

fn line_bounding_rect(line: &OcrLine) -> Result<(f32, f32, f32, f32), windows::core::Error> {
    let words = line.Words()?;
    let count = words.Size()?;
    if count == 0 {
        return Err(windows::core::Error::from(windows::core::HRESULT(-1)));
    }

    let first = words.GetAt(0)?;
    let mut min_x = first.BoundingRect()?.X;
    let mut min_y = first.BoundingRect()?.Y;
    let mut max_x = min_x;
    let mut max_y = min_y;

    for index in 0..count {
        let word = words.GetAt(index)?;
        let rect = word.BoundingRect()?;
        min_x = min_x.min(rect.X);
        min_y = min_y.min(rect.Y);
        max_x = max_x.max(rect.X + rect.Width);
        max_y = max_y.max(rect.Y + rect.Height);
    }

    Ok((min_x, min_y, max_x - min_x, max_y - min_y))
}

fn center_distance((x, y, width, height): (f32, f32, f32, f32), cx: f32, cy: f32) -> f32 {
    let px = x + width / 2.0;
    let py = y + height / 2.0;
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy
}
