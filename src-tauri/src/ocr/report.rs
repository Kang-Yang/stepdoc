use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrCandidateReport {
    pub text: String,
    pub source: String,
    pub quality: i32,
    /// Squared pixel distance from the crop-centre, used for within-region ranking/debug.
    pub distance: f32,
    /// Added: squared pixel distance from the real click point (screenshot space), used to rank
    /// candidates across regions so a distant title can't overshadow the clicked button.
    pub click_distance: f32,
    /// Crop-local bounding box of the detected text (x, y, width, height), when known. Lets the
    /// classifier test whether the click point actually lands on the text.
    pub rect: Option<(f32, f32, f32, f32)>,
    pub score: i32,
    /// Mean recognition confidence from the OCR engine (0.0–1.0), when available.
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
    /// What the click landed on: "text" (a readable label), "icon" (a text-less glyph like an X),
    /// or "blank". Drives the step description so an icon click never claims a stray label.
    pub click_kind: String,
    pub regions: Vec<OcrRegionReport>,
}
