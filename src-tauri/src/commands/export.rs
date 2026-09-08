use tauri::AppHandle;

use crate::export::{render_docx, render_gif};
use crate::models::RecordedStep;
use crate::utils::{normalize_title, save_with_dialog, timestamped_filename};

#[tauri::command]
pub async fn export_word(
    app: AppHandle,
    title: String,
    steps: Vec<RecordedStep>,
) -> Result<Option<String>, String> {
    let title = normalize_title(&title);
    let render_title = title.clone();
    let bytes = tauri::async_runtime::spawn_blocking(move || render_docx(&render_title, &steps))
        .await
        .map_err(|error| error.to_string())??;
    save_with_dialog(&app, &timestamped_filename(&title, "docx"), "Word 文档", "docx", &bytes)
}

#[tauri::command]
pub async fn export_gif(
    app: AppHandle,
    title: String,
    steps: Vec<RecordedStep>,
) -> Result<Option<String>, String> {
    let title = normalize_title(&title);
    let bytes = tauri::async_runtime::spawn_blocking(move || render_gif(&steps))
        .await
        .map_err(|error| error.to_string())??;
    save_with_dialog(&app, &timestamped_filename(&title, "gif"), "GIF 动图", "gif", &bytes)
}
