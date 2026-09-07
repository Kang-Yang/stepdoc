use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::constants::{EVENT_HOTKEY_RECORDING_TOGGLE, MAIN_WINDOW_LABEL};
use crate::models::AppState;

/// Runs on every press of the registered global hotkey.
///
/// The actual start/stop flow lives in the main window (it owns the recording options, the
/// "replace existing steps" confirmation and the busy guards), so this only hands the request
/// over as an event. When idle, the main window is brought back first so any confirmation
/// dialog becomes visible even if the user minimized it.
pub fn handle_hotkey_pressed(app: &AppHandle) {
    let recording = app.state::<AppState>().recording.load(Ordering::SeqCst);
    if !recording {
        if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
    let _ = app.emit_to(MAIN_WINDOW_LABEL, EVENT_HOTKEY_RECORDING_TOGGLE, ());
}

/// Validates and parses a shortcut description like `Ctrl+Alt+KeyR` (modifiers plus a W3C
/// `KeyboardEvent.code`, which is what the settings UI captures and stores).
fn parse_shortcut(spec: &str) -> Result<Shortcut, String> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err("快捷键不能为空".into());
    }

    let tokens: Vec<&str> = trimmed
        .split('+')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect();
    let (key_token, modifier_tokens) = tokens
        .split_last()
        .ok_or_else(|| format!("快捷键格式不正确：{spec}"))?;

    let upper_key = key_token.to_ascii_uppercase();
    let is_function_key = upper_key.len() >= 2
        && upper_key.starts_with('F')
        && upper_key.chars().skip(1).all(|byte| byte.is_ascii_digit());
    if modifier_tokens.is_empty() && !is_function_key {
        return Err(
            "快捷键需要至少一个修饰键（Ctrl / Alt / Shift / Win），或使用 F1~F12 功能键".into(),
        );
    }

    trimmed
        .parse::<Shortcut>()
        .map_err(|error| format!("无法识别的快捷键「{spec}」：{error}"))
}

/// Registers (or clears) the global recording hotkey. Called at app startup with the persisted
/// value and again whenever the user changes it in settings.
#[tauri::command]
pub fn set_recording_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    shortcut: Option<String>,
) -> Result<(), String> {
    let manager = app.global_shortcut();
    let mut registered = state
        .registered_hotkey
        .lock()
        .map_err(|error| error.to_string())?;

    // Always drop the previous binding first so re-registering the same combination is
    // idempotent (React StrictMode runs the startup registration effect twice in dev builds).
    let previous = registered.take();
    if let Some(previous) = previous {
        let _ = manager.unregister(previous);
    }

    let Some(spec) = shortcut.as_deref().map(str::trim).filter(|spec| !spec.is_empty()) else {
        // 清除快捷键
        return Ok(());
    };

    let parsed = parse_shortcut(spec)?;
    if let Err(error) = manager.register(parsed) {
        // Best effort: restore the previous binding so the app keeps a working hotkey.
        if let Some(previous) = previous {
            let _ = manager.register(previous);
            *registered = Some(previous);
        }
        return Err(format!("注册快捷键失败，可能已被其他程序占用：{error}"));
    }

    *registered = Some(parsed);
    Ok(())
}
