use image::RgbaImage;

use crate::ocr::report::OcrRegionReport;
use crate::ocr::text_pick::{is_confident_click_label, pick_fallback_label};

/// 按优先顺序扫描以点击为中心的裁剪区域（center → right → above → below）。
/// 一旦找到可信的标签就提前停止，除非设置了 `full_scan`（OCR 调试模式）。
///
/// `crop_origins` 保存每个裁剪在截图像素中的左上角（与 `crops` 平行存放），而
/// `click_point` 是真实点击在截图像素中的位置——两者都被传入候选项，
/// 使排序基于与点击的真实接近程度，而不是每个裁剪的局部中心。
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

/// 在所有视觉捕获区域中选取最强候选。区域扫描顺序仅作为平局裁决，
/// 因此邻近且得分更高的标签能胜过偶然出现的相邻文本。
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

/// 结合 OCR 几何信息与裁剪像素，判定点击到底落在什么对象上：
/// - `text`：某条已识别文本行的框内包含点击点（或在其极近距离内）-> 存在可读标签。
/// - `icon`：点击处无文本，但点击附近小窗口内的像素构成一个字形（诸如一个 X 关闭按钮）
///   -> 该次点击没有文本标签。
/// - `blank`：点击附近既无文本也无字形。
///
/// 「包含」优先于「中心距离」：点击宽控件时，落点距行中心很远却仍在框内；
/// 而点击标签页的关闭 X 时，落点恰好位于文本框外侧一点。这能防止图标点击
/// 被相邻区域里零散的 OCR 输出错误地打上标签。
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

    // 1. 主信号：点击点落在某个已接受文本行的框内（留出少量余量，因此点击控件的内边距
    //    仍算作文本点击）。
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

    // 2. 次信号：仅针对没有框几何信息的候选项。当框已知且点击位于框外时，
    //    中心距离不得「拯救」该标签——否则空白间隙上方几像素处的菜单项会错误地
    //    把这次点击据为己有。
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

    // 3. 点击处没有文本框：窗口内的墨迹表示绘制了一个控件。点击落在这个字形上——
    //    即图标——除非某个文本框仍能以更宽松的余量够到它（OCR 框可能裁剪掉几像素），
    //    此时应信任文本。窗口非常小，因此相邻行（上方的菜单栏、下方的标题栏）的墨迹
    //    不会伪造出一个字形。
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

/// 在以 `(cx, cy)` 为中心的 `win` 像素见方区域内，与局部背景差异较大
/// （亮度对比度 > 70/255）的像素占比。对绘制的字形偏大，对空白区域约为 0。
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
            // "资源管理器" 位于点击上方且较远，-> 点击排名得分低。
            region("above", "220x56", vec![candidate_with_click("资源管理器", 110, 22500.0, 22)]),
            // "文件" 是真实按钮，正位于点击处 -> 点击排名得分高。
            region("center", "240x72", vec![candidate_with_click("文件", 66, 100.0, 118)]),
            region("right", "280x72", vec![candidate_with_click("文件扩展名", 30, 900.0, 20)]),
        ];

        assert_eq!(pick_best_label_from_regions(&regions).as_deref(), Some("文件"));
    }

    /// 当点击点所在裁剪区域没有文本（纯图标按钮）时，扫描范围内相邻右侧区域的按钮
    /// 仍能胜过较远的窗口标题。
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

    /// 最近的可识别文本行正好位于点击处 -> 这是文本点击，即使中心裁剪
    /// 也包含零散的字形像素。
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

    /// 一个 X 关闭按钮点击：点击点附近没有文本，但其周围的裁剪像素构成一个字形
    /// -> 图标点击，绝不会被当作相邻的零散标签。
    #[test]
    fn classifies_icon_when_glyph_is_at_the_click_but_text_is_far() {
        let mut crop = bg_crop(26);
        for x in 112..=128 {
            for y in 28..=44 {
                crop.put_pixel(x, y, image::Rgba([240, 240, 240, 255]));
            }
        }
        let origins = vec![(0, 0), (56, 0), (0, 0), (0, 40)];
        // "选超语" 幻觉文本位于点击处 60px 远。
        let regions = center_regions(vec![candidate_with_click("选超语", 78, 3600.0, 40)]);

        assert_eq!(
            classify_click_kind(&[crop], &origins, Some((120.0, 36.0)), &regions),
            "icon"
        );
    }

    /// 点击点及其附近什么都没有，也没有字形 -> 空白点击。
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

    /// 来自真实录制的回归测试（第 1 步）：点击落在菜单栏与侧边栏标题之间的空白间隙——
    /// 位于「编辑」框下方 8px（在内边距之外），且距「资源管理器」33px。当已知其框排除点击时，
    /// 该菜单项的「中心接近度」不得把这判定为文本点击。
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

    /// OCR 框可能裁剪；一个落在紧贴方框之外但位于文本自身墨迹上的点击，必须保持文本点击
    /// 而不是降级为图标。
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

/// 来自真实录制的回归测试（第 2 步）：点击落在宽的「新建文本文件」菜单行的左侧——
    /// 距行中心 37px 但在框内。仅凭中心距离此前发生了误判；包含关系必须将其判定为文本点击。
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

    /// 来自真实录制的回归测试（第 3 步）：点击落在标签页的关闭 X 上，位于 "Untitled-1" 文本框
    /// 右侧 12px。该框不得吞掉这次点击，点击处的字形使其成为图标点击——绝不会是标签页标签。
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

    /// 落在方框余量内部（位于控件的内边距）的点击仍算作文本点击。
    #[test]
    fn click_on_control_padding_next_to_text_counts_as_text() {
        let crop = bg_crop(26);
        let origins = vec![(0, 0), (56, 0), (0, 0), (0, 40)];
        let regions = center_regions(vec![candidate_at("保存", (40.0, 20.0, 50.0, 20.0), 900.0)]);

        // 方框 [40,90]x[20,40]，余量 5 -> [35,95]x[15,45]。点击 (93, 30) 在内部。
        assert_eq!(
            classify_click_kind(&[crop], &origins, Some((93.0, 30.0)), &regions),
            "text"
        );
    }
}
