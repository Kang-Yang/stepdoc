use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::constants::{EVENT_HOTKEY_RECORDING_TOGGLE, MAIN_WINDOW_LABEL};
use crate::models::AppState;

/// 在每次按下已注册的全局热键时运行。
///
/// 实际的开始/停止流程位于主窗口（它拥有录制选项、"替换已有步骤"的确认逻辑和忙碌状态守卫），
/// 因此这里只是把请求以事件形式转交出去。空闲时先会把主窗口带回前台，这样即使用户将其最小化，
/// 任何确认对话框也能显示出来。
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

/// 校验并解析形如 `Ctrl+Alt+KeyR` 的快捷键描述（修饰键加一个 W3C `KeyboardEvent.code`，
/// 这正是设置界面所捕获并存储的内容）。
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

/// 注册（或清除）全局录制热键。在应用启动时用持久化的值调用一次，
/// 之后每当用户在设置中修改热键时也会再次调用。
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

    // 总是先释放上一次的绑定，这样重新注册同一个组合是幂等的
    // （React StrictMode 在开发构建中会执行启动注册副作用两次）。
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
        // 尽力而为：恢复上一次绑定，让应用仍保有可用的热键。
        if let Some(previous) = previous {
            let _ = manager.register(previous);
            *registered = Some(previous);
        }
        return Err(format!("注册快捷键失败，可能已被其他程序占用：{error}"));
    }

    *registered = Some(parsed);
    Ok(())
}
