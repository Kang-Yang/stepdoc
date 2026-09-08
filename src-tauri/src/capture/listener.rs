use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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

/// 全局输入监听是否已启动。`rdev::listen` 安装的是进程级全局钩子，只能存在一个实例，
/// 重复调用会直接失败；因此把监听线程做成**应用级单例**，只启动一次并常驻。是否触发捕获
/// 由 `recording` / `recording_paused` 标志门控，不再每次录制都新建线程（旧实现会导致
/// 第二次起录制彻底收不到鼠标事件，并累积无法退出的僵尸线程）。
static INPUT_LISTENER_STARTED: AtomicBool = AtomicBool::new(false);

pub fn ensure_input_listener(
    recording: Arc<AtomicBool>,
    recording_paused: Arc<AtomicBool>,
    capture_tx_holder: Arc<Mutex<Option<Sender<CaptureJob>>>>,
    capture_pending: Arc<AtomicUsize>,
    capture_total: Arc<AtomicUsize>,
    screenshot_cache: Arc<ScreenshotCache>,
) {
    if INPUT_LISTENER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    thread::spawn(move || {
        let mut last_position = (0.0f64, 0.0f64);
        let mut last_click: Option<(u128, f64, f64)> = None;

        let _ = rdev::listen(move |event| {
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