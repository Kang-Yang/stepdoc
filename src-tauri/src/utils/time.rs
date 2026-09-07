use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::RecordingTiming;

pub fn now_timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn now_ms_u64() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub fn recording_elapsed_ms(timing: &RecordingTiming) -> u128 {
    if timing.paused {
        return timing.accumulated_ms;
    }

    let now = now_ms_u64();
    timing.accumulated_ms + now.saturating_sub(timing.segment_started_at)
}
