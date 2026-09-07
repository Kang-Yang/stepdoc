use std::sync::RwLock;

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::constants::{MAIN_WINDOW_LABEL, RECORDING_BAR_LABEL, RECORDING_BAR_WIDTH};

static RECORDING_BAR_BOUNDS: RwLock<Option<(f64, f64, f64, f64)>> = RwLock::new(None);

pub fn show_recording_bar(app: &AppHandle) -> Result<(), String> {
    let app_for_show = app.clone();
    run_on_main_thread_result(app, move || show_recording_bar_inner(&app_for_show))
}

pub fn close_recording_bar(app: &AppHandle) {
    let app_for_close = app.clone();
    run_on_main_thread_blocking(app, move || {
        if let Some(bar) = app_for_close.get_webview_window(RECORDING_BAR_LABEL) {
            let _ = bar.hide();
        }
        clear_recording_bar_bounds();
    });
}

/// Hit-test using bounds cached on the main thread. Safe to call from the input hook thread.
pub fn is_point_on_recording_bar(x: f64, y: f64) -> bool {
    let bounds = RECORDING_BAR_BOUNDS
        .read()
        .ok()
        .and_then(|guard| *guard);
    let Some((left, top, right, bottom)) = bounds else {
        return false;
    };

    x >= left && x < right && y >= top && y < bottom
}

pub fn restore_main_window_from_recording(app: &AppHandle) {
    let app_for_restore = app.clone();
    run_on_main_thread_blocking(app, move || {
        let Some(main) = app_for_restore.get_webview_window(MAIN_WINDOW_LABEL) else {
            return;
        };
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    });
}

pub fn minimize_main_window(app: &AppHandle) {
    let app_for_minimize = app.clone();
    run_on_main_thread_blocking(app, move || {
        if let Some(main) = app_for_minimize.get_webview_window(MAIN_WINDOW_LABEL) {
            let _ = main.minimize();
        }
    });
}

fn show_recording_bar_inner(app: &AppHandle) -> Result<(), String> {
    let bar = recording_bar_window(app)?;
    position_recording_bar(app, &bar)?;
    bar.set_size(tauri::LogicalSize::new(RECORDING_BAR_WIDTH, 56.0))
        .map_err(|error| error.to_string())?;
    let _ = bar.set_always_on_top(true);
    bar.show().map_err(|error| error.to_string())?;
    cache_recording_bar_bounds(&bar);
    Ok(())
}

fn cache_recording_bar_bounds(bar: &WebviewWindow) {
    let bounds = recording_bar_screen_bounds(bar).or_else(|| {
        let position = bar.outer_position().ok()?;
        let size = bar.outer_size().ok()?;
        Some((
            position.x as f64,
            position.y as f64,
            position.x as f64 + size.width as f64,
            position.y as f64 + size.height as f64,
        ))
    });

    if let Ok(mut guard) = RECORDING_BAR_BOUNDS.write() {
        *guard = bounds;
    }
}

fn clear_recording_bar_bounds() {
    if let Ok(mut guard) = RECORDING_BAR_BOUNDS.write() {
        *guard = None;
    }
}

fn recording_bar_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window(RECORDING_BAR_LABEL)
        .ok_or_else(|| "找不到录制悬浮栏窗口".to_string())
}

fn position_recording_bar(app: &AppHandle, bar: &WebviewWindow) -> Result<(), String> {
    let monitor = app
        .primary_monitor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "找不到主显示器".to_string())?;
    let scale = monitor.scale_factor();
    let monitor_size = monitor.size();
    let monitor_pos = monitor.position();
    let x = monitor_pos.x as f64 / scale
        + (monitor_size.width as f64 / scale - RECORDING_BAR_WIDTH) / 2.0;
    let y = monitor_pos.y as f64 / scale + 20.0;

    bar.set_position(tauri::LogicalPosition::new(x, y))
        .map_err(|error| error.to_string())
}

fn run_on_main_thread_blocking<F: FnOnce() + Send + 'static>(app: &AppHandle, task: F) {
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    if app
        .run_on_main_thread(move || {
            task();
            let _ = done_tx.send(());
        })
        .is_err()
    {
        return;
    }
    let _ = done_rx.recv_timeout(std::time::Duration::from_secs(2));
}

fn run_on_main_thread_result<T, F>(app: &AppHandle, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = done_tx.send(task());
    })
    .map_err(|error| error.to_string())?;
    done_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| "显示录制栏超时".to_string())?
}

#[cfg(target_os = "windows")]
fn recording_bar_screen_bounds(bar: &WebviewWindow) -> Option<(f64, f64, f64, f64)> {
    use std::ffi::c_void;

    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let hwnd = HWND(bar.hwnd().ok()?.0 as *mut c_void);
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return None;
    }

    Some((
        rect.left as f64,
        rect.top as f64,
        rect.right as f64,
        rect.bottom as f64,
    ))
}

#[cfg(not(target_os = "windows"))]
fn recording_bar_screen_bounds(_bar: &WebviewWindow) -> Option<(f64, f64, f64, f64)> {
    None
}
