mod debug_dump;
mod pipeline;
mod queue;
mod rapid;
mod region;
mod report;
mod text_pick;

#[cfg(windows)]
mod windows;

use image::RgbaImage;

use crate::ocr::report::OcrPipelineReport;

pub use debug_dump::{begin_debug_session, dump_ocr_debug, open_last_debug_dir};
pub use queue::{OcrJob, OcrQueue};

#[derive(Clone, Debug, Default)]
pub struct OcrCaptureContext {
    pub region_labels: Vec<String>,
    /// 每个裁剪在截图像素中的左上角原点，与裁剪列表对齐。
    pub crop_origins: Vec<(u32, u32)>,
    /// 截图空间中的绝对点击点，用于按真实接近程度对候选排序。
    pub click_point: Option<(f64, f64)>,
    pub capture_source: Option<String>,
}

impl OcrCaptureContext {
    fn apply_to_report(&self, report: &mut OcrPipelineReport) {
        report.capture_source = self.capture_source.clone();
    }
}

pub fn warmup_engine() {
    rapid::warmup();
}

pub fn recognize_click_label_from_crops_with_report(
    crops: &[RgbaImage],
    context: &OcrCaptureContext,
    full_scan: bool,
) -> OcrPipelineReport {
    let mut report =
        rapid::recognize_label_from_crops_with_report(crops, context, full_scan);
    context.apply_to_report(&mut report);

    if report.engine == "rapidocr-unavailable" {
        #[cfg(windows)]
        {
            let mut windows_report =
                windows::recognize_label_from_crops_with_report(crops, context, full_scan);
            context.apply_to_report(&mut windows_report);
            report = windows_report;
        }
        #[cfg(not(windows))]
        {
            // 平台没有回退方案；保留不可用的报告。
        }
    }

    report.click_kind = pipeline::classify_click_kind(
        crops,
        &context.crop_origins,
        context.click_point,
        &report.regions,
    );
    report
}

pub fn build_click_description(event_type: &str, label: Option<&str>, click_kind: &str) -> String {
    let prefix = match event_type {
        "right-click" => "右键点击",
        _ => "点击",
    };

    match click_kind {
        "icon" => format!("{prefix}图标"),
        "blank" => format!("{prefix}此处"),
        _ => match label {
            Some(text) if text_pick::is_plausible_ui_label(text) => format!("{prefix}「{text}」"),
            _ => format!("{prefix}此处"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::build_click_description;

    #[test]
    fn text_click_uses_the_label() {
        assert_eq!(build_click_description("left-click", Some("文件"), "text"), "点击「文件」");
    }

    #[test]
    fn icon_click_never_claims_a_stray_label() {
        assert_eq!(build_click_description("left-click", Some("选超语"), "icon"), "点击图标");
    }

    #[test]
    fn blank_click_falls_back_to_here() {
        assert_eq!(build_click_description("left-click", None, "blank"), "点击此处");
    }

    #[test]
    fn right_click_keeps_its_prefix() {
        assert_eq!(build_click_description("right-click", Some("主页"), "text"), "右键点击「主页」");
    }
}
