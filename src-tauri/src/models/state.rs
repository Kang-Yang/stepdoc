use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use tauri_plugin_global_shortcut::Shortcut;

use super::step::RecordedStep;
use crate::capture::CaptureJob;
use crate::ocr::OcrQueue;
use crate::screenshot::ScreenshotCache;

#[derive(Default)]
pub struct RecordingTiming {
    pub segment_started_at: u128,
    pub accumulated_ms: u128,
    pub paused: bool,
}

#[derive(Default)]
pub struct KeyBuffer {
    pub chars: String,
    pub last_event: u128,
}

pub struct AppState {
    pub recording: Arc<AtomicBool>,
    pub recording_paused: Arc<AtomicBool>,
    pub recording_timing: Arc<Mutex<RecordingTiming>>,
    pub steps: Arc<Mutex<Vec<RecordedStep>>>,
    pub key_buffer: Arc<Mutex<KeyBuffer>>,
    pub capture_tx: Arc<Mutex<Option<Sender<CaptureJob>>>>,
    pub capture_pending: Arc<AtomicUsize>,
    pub capture_total: Arc<AtomicUsize>,
    pub screenshot_cache: Arc<ScreenshotCache>,
    pub ocr_debug: Arc<AtomicBool>,
    pub ocr_debug_session: Arc<Mutex<Option<PathBuf>>>,
    pub ocr_queue: Arc<OcrQueue>,
    /// 当前注册的全局录制快捷键，保留它以便在用户更换或清除快捷键时注销。
    /// `None` 表示当前没有启用的快捷键。
    pub registered_hotkey: Mutex<Option<Shortcut>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            recording: Arc::new(AtomicBool::new(false)),
            recording_paused: Arc::new(AtomicBool::new(false)),
            recording_timing: Arc::new(Mutex::new(RecordingTiming::default())),
            steps: Arc::new(Mutex::new(Vec::new())),
            key_buffer: Arc::new(Mutex::new(KeyBuffer::default())),
            capture_tx: Arc::new(Mutex::new(None)),
            capture_pending: Arc::new(AtomicUsize::new(0)),
            capture_total: Arc::new(AtomicUsize::new(0)),
            screenshot_cache: Arc::new(ScreenshotCache::default()),
            ocr_debug: Arc::new(AtomicBool::new(false)),
            ocr_debug_session: Arc::new(Mutex::new(None)),
            ocr_queue: Arc::new(OcrQueue::spawn()),
            registered_hotkey: Mutex::new(None),
        }
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            recording: self.recording.clone(),
            recording_paused: self.recording_paused.clone(),
            recording_timing: self.recording_timing.clone(),
            steps: self.steps.clone(),
            key_buffer: self.key_buffer.clone(),
            capture_tx: self.capture_tx.clone(),
            capture_pending: self.capture_pending.clone(),
            capture_total: self.capture_total.clone(),
            screenshot_cache: self.screenshot_cache.clone(),
            ocr_debug: self.ocr_debug.clone(),
            ocr_debug_session: self.ocr_debug_session.clone(),
            ocr_queue: self.ocr_queue.clone(),
            registered_hotkey: Mutex::new(
                self.registered_hotkey.lock().map(|hotkey| *hotkey).unwrap_or(None),
            ),
        }
    }
}
