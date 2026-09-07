use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedStep {
    pub id: String,
    pub event_type: String,
    pub description: String,
    pub coordinates: Option<Vec<f64>>,
    pub image_base64: Option<String>,
    pub timestamp: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepUpdate {
    pub id: String,
    pub description: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
    pub recording: bool,
    pub paused: bool,
    pub elapsed_ms: u128,
    pub step_count: usize,
}
