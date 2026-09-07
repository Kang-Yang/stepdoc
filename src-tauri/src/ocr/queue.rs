use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use image::RgbaImage;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::constants::{EVENT_OCR_PROGRESS, EVENT_RECORDING_STEP_UPDATED};
use crate::models::{RecordedStep, StepUpdate};
use crate::ocr::{self, dump_ocr_debug, OcrCaptureContext};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrProgress {
    pub completed: usize,
    pub total: usize,
    pub active_step: Option<usize>,
}

pub struct OcrJob {
    pub step_id: String,
    pub step_no: usize,
    pub event_type: String,
    pub ocr_crops: Vec<RgbaImage>,
    pub region_labels: Vec<String>,
    pub capture_context: OcrCaptureContext,
    pub debug_enabled: bool,
    pub debug_session_dir: Option<PathBuf>,
    pub steps: Arc<Mutex<Vec<RecordedStep>>>,
    pub app: AppHandle,
    /// Session stamp assigned by `enqueue`; jobs whose stamp no longer matches the queue's
    /// current generation were cancelled and are dropped without processing.
    pub generation: usize,
}

pub struct OcrQueue {
    tx: mpsc::Sender<OcrJob>,
    pending: Arc<AtomicUsize>,
    total: Arc<AtomicUsize>,
    expected_total: Arc<AtomicUsize>,
    completed: Arc<AtomicUsize>,
    generation: Arc<AtomicUsize>,
}

impl OcrQueue {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<OcrJob>();
        let pending = Arc::new(AtomicUsize::new(0));
        let pending_worker = pending.clone();
        let total = Arc::new(AtomicUsize::new(0));
        let total_worker = total.clone();
        let expected_total = Arc::new(AtomicUsize::new(0));
        let expected_total_worker = expected_total.clone();
        let completed = Arc::new(AtomicUsize::new(0));
        let completed_worker = completed.clone();
        let generation = Arc::new(AtomicUsize::new(0));
        let generation_worker = generation.clone();

        thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                let app = job.app.clone();
                let job_generation = job.generation;
                if job_generation != generation_worker.load(Ordering::SeqCst) {
                    // Cancelled session: the counters were already reset by `cancel_pending`,
                    // so just drop the job instead of processing it or touching the counters.
                    continue;
                }
                emit_progress(
                    &app,
                    completed_worker.load(Ordering::SeqCst),
                    reported_total(&total_worker, &expected_total_worker),
                    Some(job.step_no),
                );
                process_job(job);
                // Cancellation may have fired while this job was running; skip the counter
                // updates so `cancel_pending`'s reset stays authoritative.
                if job_generation != generation_worker.load(Ordering::SeqCst) {
                    continue;
                }
                pending_worker.fetch_sub(1, Ordering::SeqCst);
                let completed = completed_worker.fetch_add(1, Ordering::SeqCst) + 1;
                emit_progress(
                    &app,
                    completed,
                    reported_total(&total_worker, &expected_total_worker),
                    None,
                );
            }
        });

        Self {
            tx,
            pending,
            total,
            expected_total,
            completed,
            generation,
        }
    }

    pub fn enqueue(&self, mut job: OcrJob) {
        job.generation = self.generation.load(Ordering::SeqCst);
        self.pending.fetch_add(1, Ordering::SeqCst);
        self.total.fetch_add(1, Ordering::SeqCst);
        let _ = self.tx.send(job);
    }

    /// Drops every queued job and zeroes the progress counters. Called when the user clears the
    /// steps or starts a new session, so stale recognition work neither burns CPU nor keeps the
    /// progress UI moving for steps that no longer exist. A job already being processed finishes,
    /// but no longer reports progress.
    pub fn cancel_pending(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.pending.store(0, Ordering::SeqCst);
        self.total.store(0, Ordering::SeqCst);
        self.expected_total.store(0, Ordering::SeqCst);
        self.completed.store(0, Ordering::SeqCst);
    }

    /// Sets the final number of clicks after the capture queue is closed.
    pub fn set_expected_total(&self, total: usize) {
        self.expected_total.store(total, Ordering::SeqCst);
    }

    pub fn progress(&self) -> OcrProgress {
        OcrProgress {
            completed: self.completed.load(Ordering::SeqCst),
            total: reported_total(&self.total, &self.expected_total),
            active_step: None,
        }
    }

    pub fn wait_pending(&self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        while self.pending.load(Ordering::SeqCst) > 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
    }
}

fn reported_total(total: &AtomicUsize, expected_total: &AtomicUsize) -> usize {
    total
        .load(Ordering::SeqCst)
        .max(expected_total.load(Ordering::SeqCst))
}

fn emit_progress(app: &AppHandle, completed: usize, total: usize, active_step: Option<usize>) {
    let _ = app.emit(
        EVENT_OCR_PROGRESS,
        OcrProgress {
            completed,
            total,
            active_step,
        },
    );
}

fn process_job(job: OcrJob) {
    let OcrJob {
        step_id,
        step_no,
        event_type,
        ocr_crops,
        region_labels,
        capture_context,
        debug_enabled,
        debug_session_dir,
        steps,
        app,
        ..
    } = job;

    let report = ocr::recognize_click_label_from_crops_with_report(
        &ocr_crops,
        &capture_context,
        debug_enabled,
    );
    let label = report.picked_label.clone();

    if debug_enabled {
        if let Some(session_dir) = debug_session_dir.as_deref() {
            let _ = dump_ocr_debug(
                session_dir,
                step_no,
                &step_id,
                &ocr_crops,
                &region_labels,
                &report,
            );
        }
    }

    let description = ocr::build_click_description(&event_type, label.as_deref(), &report.click_kind);

    let mut found = false;
    if let Ok(mut locked) = steps.lock() {
        if let Some(step) = locked.iter_mut().find(|step| step.id == step_id) {
            step.description = description.clone();
            found = true;
        }
    }

    // Emit on every job, not only when the text changed: a redundant update is cheaper than the
    // UI staying on the placeholder because an earlier event was swallowed or lost.
    if found {
        let _ = app.emit(
            EVENT_RECORDING_STEP_UPDATED,
            StepUpdate {
                id: step_id,
                description,
            },
        );
    }
}
