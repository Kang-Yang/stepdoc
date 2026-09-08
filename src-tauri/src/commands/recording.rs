use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;

use tauri::State;
use tauri::{AppHandle, Emitter};

use crate::capture::{
    clear_key_buffer, run_capture_worker, spawn_input_listener, wait_for_capture_jobs,
};
use crate::constants::{
    EVENT_RECORDING_FINALIZING, EVENT_RECORDING_PAUSED, EVENT_RECORDING_RESUMED,
    EVENT_RECORDING_STARTED, EVENT_RECORDING_STOPPED, MAIN_WINDOW_LABEL, STOP_DRAIN_TIMEOUT,
};
use crate::models::{AppState, RecordedStep, RecordingOptions, RecordingStatus, RecordingTiming};
use crate::ocr;
use crate::utils::{now_ms_u64, recording_elapsed_ms};
use crate::window::{
    close_recording_bar, minimize_main_window, restore_main_window_from_recording,
    show_recording_bar,
};

#[tauri::command]
pub fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    options: Option<RecordingOptions>,
) -> Result<(), String> {
    if state.recording.swap(true, Ordering::SeqCst) {
        return Err("已经在录制中".into());
    }

    let options = options.unwrap_or_default();
    state.ocr_debug.store(options.ocr_debug, Ordering::SeqCst);
    {
        let mut debug_session = state
            .ocr_debug_session
            .lock()
            .map_err(|error| error.to_string())?;
        *debug_session = if options.ocr_debug {
            Some(ocr::begin_debug_session()?)
        } else {
            None
        };
    }

    state.recording_paused.store(false, Ordering::SeqCst);
    if let Ok(mut timing) = state.recording_timing.lock() {
        *timing = RecordingTiming {
            segment_started_at: now_ms_u64(),
            accumulated_ms: 0,
            paused: false,
        };
    }

    if let Ok(mut steps) = state.steps.lock() {
        steps.clear();
    }
    if let Ok(mut key_buffer) = state.key_buffer.lock() {
        key_buffer.chars.clear();
        key_buffer.last_event = 0;
    }

    let session_id = state.recording_session.fetch_add(1, Ordering::SeqCst) + 1;
    let recording = state.recording.clone();
    let recording_paused = state.recording_paused.clone();
    let recording_session = state.recording_session.clone();
    let steps = state.steps.clone();
    let capture_tx_holder = state.capture_tx.clone();
    let capture_pending = state.capture_pending.clone();
    let capture_total = state.capture_total.clone();
    let screenshot_cache = state.screenshot_cache.clone();
    let ocr_debug = state.ocr_debug.clone();
    let ocr_debug_session = state.ocr_debug_session.clone();
    let ocr_queue = state.ocr_queue.clone();
    let app_handle = app.clone();

    // 同时丢弃上一会话中尚未排空、仍留在队列里的任务。
    ocr_queue.cancel_pending();
    capture_pending.store(0, Ordering::SeqCst);
    capture_total.store(0, Ordering::SeqCst);
    screenshot_cache.start();
    ocr::warmup_engine();

    let (capture_tx, capture_rx) = mpsc::channel();
    {
        let mut sender = capture_tx_holder
            .lock()
            .map_err(|error| error.to_string())?;
        *sender = Some(capture_tx);
    }

    let worker_steps = steps.clone();
    let worker_app = app_handle.clone();
    let worker_ocr_debug = ocr_debug.clone();
    let worker_ocr_debug_session = ocr_debug_session.clone();
    let worker_ocr_queue = ocr_queue.clone();
    let worker_capture_pending = capture_pending.clone();
    thread::spawn(move || {
        run_capture_worker(
            capture_rx,
            worker_steps,
            worker_app,
            worker_ocr_debug,
            worker_ocr_debug_session,
            worker_ocr_queue,
            worker_capture_pending,
        );
    });

    spawn_input_listener(
        session_id,
        recording,
        recording_paused,
        recording_session,
        capture_tx_holder,
        capture_pending,
        capture_total,
        screenshot_cache,
    );

    show_recording_bar(&app)?;
    minimize_main_window(&app);
    let _ = app.emit_to(MAIN_WINDOW_LABEL, EVENT_RECORDING_STARTED, ());

    Ok(())
}

#[tauri::command]
pub fn pause_recording(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if !state.recording.load(Ordering::SeqCst) {
        return Err("当前未在录制".into());
    }
    if state.recording_paused.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let now = now_ms_u64();
    if let Ok(mut timing) = state.recording_timing.lock() {
        timing.accumulated_ms += now.saturating_sub(timing.segment_started_at);
        timing.paused = true;
    }
    let _ = app.emit_to(MAIN_WINDOW_LABEL, EVENT_RECORDING_PAUSED, ());
    Ok(())
}

#[tauri::command]
pub fn resume_recording(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if !state.recording.load(Ordering::SeqCst) {
        return Err("当前未在录制".into());
    }
    if !state.recording_paused.swap(false, Ordering::SeqCst) {
        return Ok(());
    }

    if let Ok(mut timing) = state.recording_timing.lock() {
        timing.segment_started_at = now_ms_u64();
        timing.paused = false;
    }
    let _ = app.emit_to(MAIN_WINDOW_LABEL, EVENT_RECORDING_RESUMED, ());
    Ok(())
}

#[tauri::command]
pub fn get_recording_status(state: State<'_, AppState>) -> Result<RecordingStatus, String> {
    let recording = state.recording.load(Ordering::SeqCst);
    let paused = state.recording_paused.load(Ordering::SeqCst);
    let elapsed_ms = state
        .recording_timing
        .lock()
        .map(|timing| recording_elapsed_ms(&timing))
        .unwrap_or(0);
    let step_count = state.steps.lock().map(|steps| steps.len()).unwrap_or(0);

    Ok(RecordingStatus {
        recording,
        paused,
        elapsed_ms,
        step_count,
    })
}

#[tauri::command]
pub async fn stop_recording(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<RecordedStep>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stop_recording_inner(app, state))
        .await
        .map_err(|error| format!("结束录制任务失败: {error}"))?
}

fn stop_recording_inner(app: AppHandle, state: AppState) -> Result<Vec<RecordedStep>, String> {
    if !state.recording.load(Ordering::SeqCst) {
        return Err("当前未在录制".into());
    }

    state.recording.store(false, Ordering::SeqCst);
    state.recording_paused.store(false, Ordering::SeqCst);
    state.ocr_debug.store(false, Ordering::SeqCst);
    if let Ok(mut debug_session) = state.ocr_debug_session.lock() {
        *debug_session = None;
    }
    clear_key_buffer(&state.key_buffer);
    {
        let mut sender = state.capture_tx.lock().map_err(|error| error.to_string())?;
        *sender = None;
    }
    state.screenshot_cache.stop();

    // 立即把用户带回结果视图。OCR 的完成过程运行在该命令的后台任务中，
    // 当最终步骤就绪时会发出 EVENT_RECORDING_STOPPED。
    close_recording_bar(&app);
    restore_main_window_from_recording(&app);
    state
        .ocr_queue
        .set_expected_total(state.capture_total.load(Ordering::SeqCst));
    let _ = app.emit_to(
        MAIN_WINDOW_LABEL,
        EVENT_RECORDING_FINALIZING,
        state.ocr_queue.progress(),
    );

    // 等待采集与 OCR 队列（几乎）无限直到排空：过早发出 EVENT_RECORDING_STOPPED
    // 正是进度条结束后仍显示占位描述、而真实结果要几秒后才通过步骤更新事件到达的原因。
    wait_for_capture_jobs(&state.capture_pending, STOP_DRAIN_TIMEOUT);
    state.ocr_queue.wait_pending(STOP_DRAIN_TIMEOUT);
    let steps = state
        .steps
        .lock()
        .map_err(|error| error.to_string())?
        .clone();

    // 在等待期间可能发生了新会话（或清空）；只有当没有更新的录制会话占用它时才收尾 UI，
    // 否则过期的停止事件会清掉它的状态。
    if !state.recording.load(Ordering::SeqCst) {
        let _ = app.emit_to(MAIN_WINDOW_LABEL, EVENT_RECORDING_STOPPED, steps.clone());
    }

    Ok(steps)
}

#[tauri::command]
pub fn clear_steps(state: State<'_, AppState>) -> Result<(), String> {
    // 清空也必须同时停止识别：丢弃排队的 OCR 任务能够立即结束收尾进度，
    // 而不是让 worker 继续处理那些已不存在的步骤，并让待处理的停止流程得以立即收尾。
    state.ocr_queue.cancel_pending();
    state
        .steps
        .lock()
        .map_err(|error| error.to_string())?
        .clear();
    Ok(())
}
