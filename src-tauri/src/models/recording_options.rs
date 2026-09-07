use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingOptions {
    #[serde(default = "default_exclude_recording_bar")]
    #[allow(dead_code)]
    pub exclude_recording_bar: bool,
    #[serde(default)]
    pub ocr_debug: bool,
}

impl Default for RecordingOptions {
    fn default() -> Self {
        Self {
            exclude_recording_bar: true,
            ocr_debug: false,
        }
    }
}

fn default_exclude_recording_bar() -> bool {
    true
}
