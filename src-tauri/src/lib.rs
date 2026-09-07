mod capture;
mod commands;
mod constants;
// Public for dev probes under `examples/` that exercise the export pipeline on synthetic data.
pub mod export;
// Public so probes can construct RecordedStep instances for the export pipeline.
pub mod models;
// Public for dev probes under `examples/` that replay saved debug crops.
pub mod ocr;
mod platform;
mod screenshot;
mod utils;
mod window;

use models::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
