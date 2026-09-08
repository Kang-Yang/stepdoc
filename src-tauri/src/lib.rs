mod capture;
mod commands;
mod constants;
// 公开给 `examples/` 下的开发探针使用，这些探针在合成数据上测试导出流水线。
pub mod export;
// 公开以便探针能为导出流水线构造 RecordedStep 实例。
pub mod models;
// 公开给 `examples/` 下的开发探针使用，这些探针重放已保存的调试裁剪图。
pub mod ocr;
mod platform;
mod screenshot;
mod utils;
mod window;

use tauri::Manager;

use models::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // 强制单实例运行：再次启动 exe 时，把新实例的参数转交给已运行的进程，而不是再拉起一个
        //（否则会抢占全局快捷键、复制录制状态），然后把已存在的主窗口重新带到前台。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window(constants::MAIN_WINDOW_LABEL) {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        commands::hotkey::handle_hotkey_pressed(app);
                    }
                })
                .build(),
        )
        .manage(AppState::default())
        // 关闭主窗口是用户退出 StepDoc 的方式。显式调用退出，而不是依赖“最后一个窗口
        // 关闭时退出”：否则任何仍存活但隐藏的窗口都会在后台留下一个幽灵进程。
        .on_window_event(|window, event| {
            if window.label() == constants::MAIN_WINDOW_LABEL
                && matches!(event, tauri::WindowEvent::Destroyed)
            {
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::pause_recording,
            commands::recording::resume_recording,
            commands::recording::get_recording_status,
            commands::recording::clear_steps,
            commands::hotkey::set_recording_hotkey,
            commands::export::export_word,
            commands::export::export_gif,
            commands::ocr_debug::open_ocr_debug_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running StepDoc");
}
