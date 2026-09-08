use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

pub fn normalize_title(title: &str) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        "操作教程".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let invalid = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
    let cleaned: String = name
        .chars()
        .map(|ch| if invalid.contains(&ch) { '_' } else { ch })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "stepdoc".to_string()
    } else {
        trimmed.chars().take(80).collect()
    }
}

/// 默认导出文件名：`标题_YYYYMMDD_HHMMSS.ext`。定宽时间戳让文件按名称排序即等于按
/// 时间排序，且同一标题的重复导出也不会互相冲突。
pub fn timestamped_filename(title: &str, extension: &str) -> String {
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    format!("{}_{}.{}", sanitize_filename(title), stamp, extension)
}

pub fn save_with_dialog(
    app: &AppHandle,
    default_name: &str,
    filter_name: &str,
    extension: &str,
    contents: &[u8],
) -> Result<Option<String>, String> {
    let picked = app
        .dialog()
        .file()
        .set_title("保存教程")
        .set_file_name(default_name)
        .add_filter(filter_name, &[extension])
        .blocking_save_file();

    let Some(file_path) = picked else {
        return Ok(None);
    };

    let path = file_path.into_path().map_err(|error| error.to_string())?;
    std::fs::write(&path, contents).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}
