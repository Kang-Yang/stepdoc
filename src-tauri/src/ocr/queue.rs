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

/// 饱和递减：`cancel_pending` 会把计数清成 0，此时在飞任务稍后的递减（generation 检查与
/// 实际递减之间存在时序窗口）会把 0 往下绕成 `usize::MAX`，导致 `wait_pending` 卡满超时。
/// 用不会低于 0 的递减兜底，消除该下溢。
fn decrement_pending(counter: &AtomicUsize) {
    let _ = counter.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| value.checked_sub(1));
}

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
    /// 由 `enqueue` 分配的一次会话标记；标记已不再匹配队列当前代际的作业，意味着已被取消，
    /// 会被直接丢弃而不处理。
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
                    // 已取消的会话：`cancel_pending` 已经重置过计数器，因此直接丢弃该任务，
                    // 既不处理它也不触碰计数器。
                    continue;
                }
                emit_progress(
                    &app,
                    completed_worker.load(Ordering::SeqCst),
                    reported_total(&total_worker, &expected_total_worker),
                    Some(job.step_no),
                );
                process_job(job);
                // 本任务运行期间可能触发了取消；跳过计数器的更新，使 `cancel_pending` 的重置保持权威。
                if job_generation != generation_worker.load(Ordering::SeqCst) {
                    continue;
                }
                decrement_pending(&pending_worker);
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

    /// 丢弃所有已排队任务并把进度计数器清零。当用户清空步骤或开始新会话时调用，使过期的识别
    /// 任务既不会白白消耗 CPU，也不会继续为已不存在的步骤推进进度 UI。正在处理的任务会走完，
    /// 但不再上报进度。
    pub fn cancel_pending(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.pending.store(0, Ordering::SeqCst);
        self.total.store(0, Ordering::SeqCst);
        self.expected_total.store(0, Ordering::SeqCst);
        self.completed.store(0, Ordering::SeqCst);
    }

    /// 在捕获队列关闭后，设置最终的点击数量。
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

    // 每个任务都触发事件，而不只是文本发生变化时：一次冗余的更新也要比界面停留在占位文本
    // 上廉价得多，因为更早的事件可能已被吞没或丢失。
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
