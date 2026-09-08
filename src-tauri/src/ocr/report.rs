use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrCandidateReport {
    pub text: String,
    pub source: String,
    pub quality: i32,
    /// 距裁剪中心像素的平方距离，用于区域内的排名与调试。
    pub distance: f32,
    /// 距真实点击点（截图坐标空间）的平方像素距离，用于跨区域排序候选，使远处的标题不会
    /// 掩盖被点击的按钮。
    pub click_distance: f32,
    /// 检测到文本时裁剪局部的包围盒 (x, y, width, height)。让分类器
    /// 判断点击点是否真的落在文本上。
    pub rect: Option<(f32, f32, f32, f32)>,
    pub score: i32,
    /// OCR 引擎的平均识别置信度（0.0–1.0），若可用。
    pub ocr_confidence: Option<f32>,
    pub accepted: bool,
    pub reject_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrRegionReport {
    pub region: String,
    pub crop_size: String,
    pub upscaled_size: String,
    pub raw_lines: Vec<String>,
    pub raw_words: Vec<String>,
    pub picked_in_region: Option<String>,
    pub candidates: Vec<OcrCandidateReport>,
    pub ocr_error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrPipelineReport {
    pub engine: String,
    pub picked_label: Option<String>,
    pub capture_source: Option<String>,
    /// 本次点击落在什么上："text"（可读标签）、"icon"（类似 X 的无文字字形）、
    /// 或 "blank"。它驱动步骤描述，使图标点击永远不会声称一次不相干的标签。
    pub click_kind: String,
    pub regions: Vec<OcrRegionReport>,
}
