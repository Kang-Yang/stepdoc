use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;

use rdev::EventType;

use crate::capture::{enqueue_capture_job, CaptureJob};
use crate::constants::{CLICK_DEBOUNCE_DISTANCE, CLICK_DEBOUNCE_MS};
use crate::platform::cursor_position;
use crate::screenshot::ScreenshotCache;
use crate::utils::now_ms_u64;
use crate::window::is_point_on_recording_bar;

pub fn spawn_input_listener(
    session_id: u64,
    recording: Arc<AtomicBool>,
    recording_paused: Arc<AtomicBool>,
    recording_session: Arc<AtomicU64>,
    capture_tx_holder: Arc<Mutex<Option<Sender<CaptureJob>>>>,
    capture_pending: Arc<AtomicUsize>,
    capture_total: Arc<AtomicUsize>,
    screenshot_cache: Arc<ScreenshotCache>,
) {
    thread::spawn(move || {
        let mut last_position = (0.0f64, 0.0f64);
        let mut last_click: Option<(u128, f64, f64)> = None;

        let _ = rdev::listen(move |event| {
            if recording_session.load(Ordering::SeqCst) != session_id {
                return;
            }
            if !recording.load(Ordering::SeqCst) || recording_paused.load(Ordering::SeqCst) {
                return;
            }

            match event.event_type {
                EventType::MouseMove { x, y } => {
                    last_position = (x, y);
                }
                EventType::ButtonPress(button) => {
                    let (x, y) = cursor_position().unwrap_or(last_position);
                    if is_point_on_recording_bar(x, y) {
                        return;
                    }

                    if !should_record_click(&mut last_click, x, y) {
                        return;
                    }

                    enqueue_capture_job(
                        &capture_tx_holder,
                        &capture_pending,
                        &capture_total,
                        CaptureJob::Click {
                            button,
                            x,
                            y,
                            cached_frame: screenshot_cache.frame_at(x, y),
                        },
                    );
                }
                _ => {}
            }
        });
    });
}

fn should_record_click(last_click: &mut Option<(u128, f64, f64)>, x: f64, y: f64) -> bool {
    let now = now_ms_u64();
    if let Some((prev_at, prev_x, prev_y)) = *last_click {
        let elapsed = now.saturating_sub(prev_at);
        let distance = ((x - prev_x).powi(2) + (y - prev_y).powi(2)).sqrt();
        if elapsed < CLICK_DEBOUNCE_MS && distance < CLICK_DEBOUNCE_DISTANCE {
            return false;
        }
    }

    *last_click = Some((now, x, y));
    true
}
