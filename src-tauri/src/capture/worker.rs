use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rdev::Button;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::capture::CaptureJob;
use crate::constants::EVENT_RECORDING_STEP;
use crate::models::RecordedStep;
use crate::ocr::{self, OcrCaptureContext, OcrJob};
use crate::screenshot::{
    capture_click_assets, capture_click_assets_from_frame, CachedScreenFrame, ClickCapture,
};
use crate::utils::now_timestamp;

pub fn enqueue_capture_job(
    capture_tx: &Mutex<Option<Sender<CaptureJob>>>,
    capture_pending: &AtomicUsize,
    capture_total: &AtomicUsize,
    job: CaptureJob,
) {
    let Ok(sender) = capture_tx.lock() else {
        return;
    };
    if let Some(tx) = sender.as_ref() {
        capture_pending.fetch_add(1, Ordering::SeqCst);
        capture_total.fetch_add(1, Ordering::SeqCst);
        if tx.send(job).is_err() {
            capture_pending.fetch_sub(1, Ordering::SeqCst);
            capture_total.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

pub fn run_capture_worker(
    rx: mpsc::Receiver<CaptureJob>,
    steps: Arc<Mutex<Vec<RecordedStep>>>,
    app: AppHandle,
    ocr_debug: Arc<AtomicBool>,
    ocr_debug_session: Arc<Mutex<Option<PathBuf>>>,
    ocr_queue: Arc<crate::ocr::OcrQueue>,
    capture_pending: Arc<AtomicUsize>,
) {
    while let Ok(job) = rx.recv() {
        let CaptureJob::Click {
            button,
            x,
            y,
            cached_frame,
        } = job;
        let assets = build_click_step(button, (x, y), cached_frame.as_ref());
        let step = assets.step;

        if let Ok(mut locked) = steps.lock() {
            locked.push(step.clone());
        }
        let _ = app.emit(EVENT_RECORDING_STEP, step.clone());

        if step.image_base64.is_some() && step.event_type.contains("click") {
            let step_no = steps.lock().map(|locked| locked.len()).unwrap_or(0);
            let debug_session_dir = ocr_debug_session
                .lock()
                .ok()
                .and_then(|guard| guard.clone());

            ocr_queue.enqueue(OcrJob {
                step_id: step.id.clone(),
                step_no,
                event_type: step.event_type.clone(),
                ocr_crops: assets.ocr_crops,
                region_labels: assets.region_labels,
                capture_context: assets.capture_context,
                debug_enabled: ocr_debug.load(Ordering::SeqCst),
                debug_session_dir,
                steps: steps.clone(),
                app: app.clone(),
                // `enqueue` 会写入当前的队列代数；这个占位值在任务被处理前总会被覆盖。
                generation: 0,
            });
        }
        capture_pending.fetch_sub(1, Ordering::SeqCst);
    }
}

pub fn wait_for_capture_jobs(capture_pending: &AtomicUsize, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while capture_pending.load(Ordering::SeqCst) > 0 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
}

struct ClickStepAssets {
    step: RecordedStep,
    ocr_crops: Vec<image::RgbaImage>,
    region_labels: Vec<String>,
    capture_context: OcrCaptureContext,
}

fn build_click_step(
    button: Button,
    (x, y): (f64, f64),
    cached_frame: Option<&CachedScreenFrame>,
) -> ClickStepAssets {
    let event_type = match button {
        Button::Left => "left-click",
        Button::Right => "right-click",
        _ => "click",
    };

    let capture_result = cached_frame
        .map(|frame| capture_click_assets_from_frame(frame, x, y))
        .unwrap_or_else(|| capture_click_assets(x, y));
    let capture = capture_result.unwrap_or_else(|error| {
        eprintln!("截图失败（{x:.0}, {y:.0}）: {error}");
        ClickCapture {
            display_base64: String::new(),
            ocr_crops: Vec::new(),
            region_labels: Vec::new(),
            crop_origins: Vec::new(),
            click_x: x,
            click_y: y,
            capture_source: "none".to_string(),
        }
    });

    let capture_context = OcrCaptureContext {
        region_labels: capture.region_labels.clone(),
        crop_origins: capture.crop_origins.clone(),
        click_point: capture
            .ocr_crops
            .first()
            .map(|_| (capture.click_x, capture.click_y)),
        capture_source: Some(capture.capture_source.clone()),
    };

    ClickStepAssets {
        step: RecordedStep {
            id: Uuid::new_v4().to_string(),
            event_type: event_type.to_string(),
            description: ocr::build_click_description(event_type, None, "text"),
            coordinates: Some(vec![x, y]),
            image_base64: if capture.display_base64.is_empty() {
                None
            } else {
                Some(capture.display_base64)
            },
            timestamp: now_timestamp(),
        },
        ocr_crops: capture.ocr_crops,
        region_labels: capture.region_labels,
        capture_context,
    }
}
