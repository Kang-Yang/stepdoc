use image::RgbaImage;

use crate::ocr::report::OcrRegionReport;
use crate::ocr::text_pick::{is_confident_click_label, pick_fallback_label};

/// Scan click-centered crop regions in priority order (center → right → above → below).
/// Stops early once a confident label is found unless `full_scan` is set (OCR debug).
///
/// `crop_origins` holds each crop's top-left in screenshot pixels (parallel to `crops`), and
/// `click_point` is the real click in screenshot pixels — both are threaded into candidates so
/// ranking happens by true proximity to the click, not by each crop's local centre.
pub fn scan_crop_regions<F>(
    crops: &[RgbaImage],
    region_labels: &[String],
    crop_origins: &[(u32, u32)],
    click_point: Option<(f64, f64)>,
    full_scan: bool,
    mut analyze: F,
) -> Vec<OcrRegionReport>
where
    F: FnMut(&str, &RgbaImage, Option<(u32, u32)>, Option<(f64, f64)>) -> OcrRegionReport,
{
    let mut regions = Vec::new();

    for (index, crop) in crops.iter().enumerate() {
        let region = region_labels
            .get(index)
            .map(String::as_str)
            .unwrap_or("region");
        let origin = crop_origins.get(index).copied();
        regions.push(analyze(region, crop, origin, click_point));

        if full_scan {
            continue;
        }

        let picked = pick_fallback_label(
            regions
                .iter()
                .map(|report| (report.region.as_str(), report.picked_in_region.as_deref())),
        );
        if picked
            .as_ref()
            .is_some_and(|label| is_confident_click_label(label))
        {
            break;
        }
    }

    regions
}

/// Pick the strongest candidate across all visual capture regions. Region scan order is only a
/// tie-breaker, so a nearby, higher-scored label wins over incidental neighboring text.
pub fn pick_best_label_from_regions(regions: &[OcrRegionReport]) -> Option<String> {
    let mut best: Option<(i32, usize, String)> = None;

    for (region_index, region) in regions.iter().enumerate() {
        for candidate in &region.candidates {
            if !candidate.accepted {
                continue;
            }

            let score = candidate.score;
            let tie_break = region_tie_break_priority(region.region.as_str(), region_index);
            match &best {
                Some((best_score, best_tie_break, _))
                    if score < *best_score
                        || (score == *best_score && tie_break >= *best_tie_break) => {}
                _ => best = Some((score, tie_break, candidate.text.clone())),
            }
        }
    }

    best.map(|(_, _, label)| label).or_else(|| {
        pick_fallback_label(
            regions
                .iter()
                .map(|region| (region.region.as_str(), region.picked_in_region.as_deref())),
        )
    })
}

fn region_tie_break_priority(region: &str, index: usize) -> usize {
    match region {
        "center" => 0,
        "right" => 1,
        "above" => 2,
        "below" => 3,
        _ => 4 + index,
    }
}

/// Classify what the click actually landed on, using OCR geometry and crop pixels:
/// - `text`: an recognised line's box contains the click (or sits within a hair of it) -> a
///   readable label exists.
/// - `icon`: no text at the click, but pixels in a small window around the click form a glyph
///   (e.g. an X close button) -> the click has no textual label.
/// - `blank`: neither text nor a glyph near the click.
///
/// Containment beats centre-distance: a click on a wide control lands far from the line's
/// centre yet inside its box, while a click on a tab's close X sits just outside the text box.
/// This stops icon clicks from being labelled with OCR garbage in a neighbouring crop.
pub fn classify_click_kind(
    crops: &[RgbaImage],
    crop_origins: &[(u32, u32)],
    click_point: Option<(f64, f64)>,
    regions: &[OcrRegionReport],
) -> String {
    let Some(click) = click_point else {
        return "text".to_string();
    };
    let Some(center_index) = regions.iter().position(|region| region.region == "center") else {
        return "text".to_string();
    };
    let crop = crops.get(center_index);
    let (crop_w, crop_h) = crop.map(|c| c.dimensions()).unwrap_or((240, 72));

    // 1. Primary signal: the click point falls inside some accepted line's box (with a small
    //    margin so clicks on the control's padding still count as text clicks).
    let box_contains_click = |grow_min: f32, grow_factor: f32| -> bool {
        for (index, region) in regions.iter().enumerate() {
            let Some((ox, oy)) = crop_origins.get(index).copied() else {
                continue;
            };
            for candidate in &region.candidates {
                if !candidate.accepted {
                    continue;
                }
                let Some((x, y, w, h)) = candidate.rect else {
                    continue;
                };
                let grow = (h * grow_factor).max(grow_min);
                let (x, y, w, h) = (x - grow, y - grow, w + grow * 2.0, h + grow * 2.0);
                let left = ox as f32 + x;
                let top = oy as f32 + y;
                if click.0 >= left as f64
                    && click.0 <= (left + w) as f64
                    && click.1 >= top as f64
                    && click.1 <= (top + h) as f64
                {
                    return true;
                }
            }
        }
        false
    };
    if box_contains_click(4.0, 0.25) {
        return "text".to_string();
    }

    // 2. Secondary: only for candidates without box geometry. When a box IS known and the click
    //    is outside it, centre proximity must not rescue the label — a menu item a few pixels
    //    above a blank-gap click would otherwise claim the click as its own.
    let nearest_textless = regions[center_index]
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.accepted && candidate.rect.is_none() && candidate.click_distance.is_finite()
        })
        .map(|candidate| candidate.click_distance.sqrt() as f64)
        .fold(None::<f64>, |best, dist| {
            Some(best.map_or(dist, |best| best.min(dist)))
        });
    if nearest_textless.is_some_and(|dist| dist <= (crop_h as f64 * 0.35).max(20.0)) {
        return "text".to_string();
    }

    // 3. No text box at the click: ink in the window means a drawn control. The click sits on
    //    that glyph — an icon — unless a text box still reaches it with a slightly looser margin
    //    (OCR boxes can clip a few pixels), in which case trust the text. The window is small so
    //    neighbouring rows' ink (menu bar above, header below) cannot fake a glyph.
    let Some(crop) = crop else {
        return "blank".to_string();
    };
    let origin = crop_origins.get(center_index).copied().unwrap_or((0, 0));
    let (click_x, click_y) = (
        (click.0 - origin.0 as f64).clamp(0.0, crop_w.saturating_sub(1) as f64),
        (click.1 - origin.1 as f64).clamp(0.0, crop_h.saturating_sub(1) as f64),
    );
    let win = ((crop_h as f32 * 0.25).round() as u32).max(14);
    if glyph_density_near(crop, click_x as u32, click_y as u32, win) >= 0.05 {
        if box_contains_click(6.0, 0.3) {
            return "text".to_string();
        }
        return "icon".to_string();
    }
    "blank".to_string()
}

/// Fraction of pixels in a `win`-sized square around `(cx, cy)` that differ strongly from the
/// local background (luma contrast > 70/255). Large for drawn glyphs, ~0 for empty space.
fn glyph_density_near(image: &RgbaImage, cx: u32, cy: u32, win: u32) -> f32 {
    let (image_w, image_h) = image.dimensions();
    if image_w == 0 || image_h == 0 {
        return 0.0;
    }
    let half = (win / 2).max(1) as i64;
    let x0 = cx as i64 - half;
    let y0 = cy as i64 - half;
    let x1 = (cx as i64 + half).min(image_w as i64 - 1);
    let y1 = (cy as i64 + half).min(image_h as i64 - 1);

    let mut luma = Vec::new();
    for y in y0.max(0)..=y1.max(y0) {
        for x in x0.max(0)..=x1.max(x0) {
            let p = image.get_pixel(x as u32, y as u32);
            luma.push(((p[0] as u32 + p[1] as u32 + p[2] as u32) / 3) as i32);
        }
    }
    if luma.is_empty() {
        return 0.0;
    }

    luma.sort_unstable();
    let median = luma[luma.len() / 2];
    let ink = luma.iter().filter(|v| (**v - median).abs() > 70).count();
    ink as f32 / luma.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr::report::OcrCandidateReport;

    fn candidate(text: &str, score: i32) -> OcrCandidateReport {
        OcrCandidateReport {
            text: text.to_string(),
            source: "line".to_string(),
            quality: score,
            distance: 0.0,
            click_distance: 0.0,
            rect: None,
            score,
            ocr_confidence: Some(0.99),
            accepted: true,
            reject_reason: None,
        }
    }

    fn candidate_with_click(
        text: &str,
        quality: i32,
        click_distance: f32,
        score: i32,
    ) -> OcrCandidateReport {
        OcrCandidateReport {
            text: text.to_string(),
            source: "line".to_string(),
            quality,
            distance: 0.0,
            click_distance,
            rect: None,
            score,
            ocr_confidence: Some(0.99),
            accepted: true,
            reject_reason: None,
        }
    }

    fn candidate_at(
        text: &str,
        rect: (f32, f32, f32, f32),
        click_distance: f32,
    ) -> OcrCandidateReport {
        OcrCandidateReport {
            text: text.to_string(),
            source: "line".to_string(),
            quality: 70,
            distance: 0.0,
            click_distance,
            rect: Some(rect),
            score: 50,
            ocr_confidence: Some(0.99),
            accepted: true,
            reject_reason: None,
        }
    }

    fn region(name: &str, crop_size: &str, candidates: Vec<OcrCandidateReport>) -> OcrRegionReport {
        OcrRegionReport {
            region: name.to_string(),
            crop_size: crop_size.to_string(),
            upscaled_size: crop_size.to_string(),
            raw_lines: Vec::new(),
            raw_words: Vec::new(),
            picked_in_region: candidates.first().map(|candidate| candidate.text.clone()),
            candidates,
            ocr_error: None,
        }
    }

    #[test]
    fn prefers_the_higher_scored_center_label_over_a_wider_neighboring_crop() {
        let regions = vec![
            region("right", "280x72", vec![candidate("文本文件内置", 108)]),
            region("center", "240x72", vec![candidate("文本文件", 149)]),
        ];

        assert_eq!(
            pick_best_label_from_regions(&regions).as_deref(),
            Some("文本文件")
        );
    }

    #[test]
    fn keeps_the_best_candidate_from_a_fallback_crop() {
        let regions = vec![
            region("right", "280x72", vec![candidate("保存", 130)]),
            region("center", "240x72", vec![candidate("设置", 110)]),
        ];

        assert_eq!(
            pick_best_label_from_regions(&regions).as_deref(),
            Some("保存")
        );
    }

    /// Regression: clicking the "文件" button must not be overshadowed by the longer "资源管理器"
    /// window title that lands in a neighbouring crop far from the click point. With absolute
    /// click distance folded into the score, the on-target short button wins despite its lower
    /// raw text-quality (CJK-count) score.
    #[test]
    fn click_proximity_beats_a_distant_longer_title() {
        let regions = vec![
            // "资源管理器" sits above the click, far away -> low click-rank score.
            region("above", "220x56", vec![candidate_with_click("资源管理器", 110, 22500.0, 22)]),
            // "文件" is the real button, right at the click point -> high click-rank score.
            region("center", "240x72", vec![candidate_with_click("文件", 66, 100.0, 118)]),
            region("right", "280x72", vec![candidate_with_click("文件扩展名", 30, 900.0, 20)]),
        ];

        assert_eq!(pick_best_label_from_regions(&regions).as_deref(), Some("文件"));
    }

    /// When the click-point crop has no text (icon-only button), a button on the adjacent right
    /// region still beats a distant window title across the scan.
    #[test]
    fn nearby_right_button_beats_distant_title_when_center_is_icon_only() {
        let regions = vec![
            region("above", "220x56", vec![candidate_with_click("资源管理器", 110, 30000.0, -30)]),
            region("center", "240x72", Vec::new()),
            region("right", "280x72", vec![candidate_with_click("保存", 68, 250.0, 96)]),
        ];

        assert_eq!(pick_best_label_from_regions(&regions).as_deref(), Some("保存"));
    }

    fn bg_crop(color: u8) -> RgbaImage {
        RgbaImage::from_pixel(240, 72, image::Rgba([color, color, color, 255]))
    }

    fn center_regions(candidates: Vec<OcrCandidateReport>) -> Vec<OcrRegionReport> {
        vec![region("center", "240x72", candidates)]
    }

    /// Nearest recognised line sits right at the click -> a text click, even if the centre crop
    /// also contains stray glyph pixels.
    #[test]
    fn classifies_text_when_line_is_at_the_click() {
        let crop = bg_crop(26);
        let origins = vec![(0, 0), (56, 0), (0, 0), (0, 40)];
        let regions = center_regions(vec![candidate_with_click("Untitled-1", 38, 25.0, 30)]);

        assert_eq!(
            classify_click_kind(&[crop], &origins, Some((120.0, 36.0)), &regions),
            "text"
        );
    }

    /// An X-close-button click: no text near the click point, but the crop pixels form a glyph
    /// there -> icon click, never a stray neighbour label.
    #[test]
    fn classifies_icon_when_glyph_is_at_the_click_but_text_is_far() {
        let mut crop = bg_crop(26);
        for x in 112..=128 {
            for y in 28..=44 {
                crop.put_pixel(x, y, image::Rgba([240, 240, 240, 255]));
            }
        }
        let origins = vec![(0, 0), (56, 0), (0, 0), (0, 40)];
        // "选超语" hallucination sits 60px away from the click.
        let regions = center_regions(vec![candidate_with_click("选超语", 78, 3600.0, 40)]);

        assert_eq!(
            classify_click_kind(&[crop], &origins, Some((120.0, 36.0)), &regions),
            "icon"
        );
    }

    /// Nothing at or near the click point, and no glyph -> blank click.
    #[test]
    fn classifies_blank_when_nothing_is_at_the_click() {
        let crop = bg_crop(26);
        let origins = vec![(0, 0), (56, 0), (0, 0), (0, 40)];
        let regions = center_regions(vec![candidate_with_click("Untitled-1", 38, 3600.0, 20)]);

        assert_eq!(
            classify_click_kind(&[crop], &origins, Some((120.0, 36.0)), &regions),
            "blank"
        );
    }

    /// Regression from a real recording (step 1): the click landed in the blank gap between the
    /// menu bar and the sidebar header — 8px below 编辑's box (outside its padding margin) and
    /// 33px from 资源管理器. The menu item's centre proximity must not claim this as a text
    /// click when its box is known to exclude the click.
    #[test]
    fn blank_gap_click_below_a_menu_item_is_not_rescued_by_center_distance() {
        let crop = bg_crop(26);
        let origins = vec![(0, 0), (56, 0), (0, 0), (0, 40)];
        let regions = center_regions(vec![
            candidate_at("编辑(E)", (96.0, 9.0, 46.0, 19.0), 306.25),
            candidate_at("资源管理器", (59.0, 45.0, 64.0, 15.0), 1113.0),
        ]);

        assert_eq!(
            classify_click_kind(&[crop], &origins, Some((120.0, 36.0)), &regions),
            "blank"
        );
    }

    /// OCR boxes can clip; a click just outside a tight box but on the text's own ink must stay
    /// a text click rather than being downgraded to an icon.
    #[test]
    fn click_on_text_ink_just_outside_a_clipped_box_stays_text() {
        let mut crop = bg_crop(26);
        for x in 100..=140 {
            for y in 30..=44 {
                crop.put_pixel(x, y, image::Rgba([240, 240, 240, 255]));
            }
        }
        let origins = vec![(0, 0), (56, 0), (0, 0), (0, 40)];
        let regions = center_regions(vec![candidate_at("保存", (100.0, 28.0, 40.0, 8.0), 400.0)]);

        assert_eq!(
            classify_click_kind(&[crop], &origins, Some((120.0, 41.0)), &regions),
            "text"
        );
    }

    /// Regression from a real recording (step 2): the click landed on the left part of the wide
    /// 「新建文本文件」menu row — 37px from the line CENTRE but inside its box. Centre-distance
    /// alone misfiled this; containment must classify it as a text click.
    #[test]
    fn classifies_text_when_click_is_inside_the_line_box() {
        let crop = bg_crop(26);
        let origins = vec![(0, 0), (56, 0), (0, 0), (0, 40)];
        let regions = center_regions(vec![candidate_at(
            "新建文本文件",
            (63.0, 26.0, 84.0, 18.0),
            1370.0,
        )]);

        assert_eq!(
            classify_click_kind(&[crop], &origins, Some((69.0, 35.0)), &regions),
            "text"
        );
    }

    /// Regression from a real recording (step 3): the click landed on a tab's close X, 12px
    /// right of the "Untitled-1" text box. The box must not swallow the click, and the glyph at
    /// the click makes it an icon click — never the tab label.
    #[test]
    fn tab_close_x_click_is_icon_not_the_tab_label() {
        let mut crop = bg_crop(26);
        for x in 112..=128 {
            for y in 28..=44 {
                crop.put_pixel(x, y, image::Rgba([240, 240, 240, 255]));
            }
        }
        let origins = vec![(0, 0), (56, 0), (0, 0), (0, 40)];
        let regions = center_regions(vec![candidate_at(
            "Untitled-1",
            (47.0, 32.0, 61.0, 16.0),
            1822.0,
        )]);

        assert_eq!(
            classify_click_kind(&[crop], &origins, Some((120.0, 35.0)), &regions),
            "icon"
        );
    }

    /// A click just inside the box margin (on the control's padding) still counts as text.
    #[test]
    fn click_on_control_padding_next_to_text_counts_as_text() {
        let crop = bg_crop(26);
        let origins = vec![(0, 0), (56, 0), (0, 0), (0, 40)];
        let regions = center_regions(vec![candidate_at("保存", (40.0, 20.0, 50.0, 20.0), 900.0)]);

        // Box [40,90]x[20,40], margin 5 -> [35,95]x[15,45]. Click (93, 30) is inside.
        assert_eq!(
            classify_click_kind(&[crop], &origins, Some((93.0, 30.0)), &regions),
            "text"
        );
    }
}
