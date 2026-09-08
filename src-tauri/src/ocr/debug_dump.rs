use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use image::{DynamicImage, ImageFormat, RgbaImage};
use serde_json::json;

use crate::ocr::region::prepare_ocr_input;
use crate::ocr::report::OcrPipelineReport;
use crate::utils::{now_timestamp, ocr_debug_dir};

static LAST_DEBUG_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// 保留最近的调试会话数量，更早的会被清理，防止磁盘无限膨胀。
const DEBUG_SESSIONS_KEEP: usize = 8;

/// 为一次录制会话创建新的调试文件夹。
pub fn begin_debug_session() -> Result<PathBuf, String> {
    let root = ocr_debug_dir();
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;

    cleanup_old_sessions(&root, DEBUG_SESSIONS_KEEP);

    let folder_name = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let session_dir = root.join(folder_name);
    fs::create_dir_all(&session_dir).map_err(|error| error.to_string())?;

    let session_meta = json!({
        "startedAt": now_timestamp(),
        "stepCount": 0,
    });
    fs::write(
        session_dir.join("session.json"),
        serde_json::to_string_pretty(&session_meta).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;

    write_latest_pointer(&root, &session_dir)?;

    if let Ok(mut last) = LAST_DEBUG_DIR.lock() {
        *last = Some(session_dir.clone());
    }

    Ok(session_dir)
}

/// 保留最近的 `keep` 个调试会话目录，更早的按目录名（时间戳）排序后删除。
fn cleanup_old_sessions(root: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut sessions: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|entry| entry.path())
        .collect();
    sessions.sort();
    let remove = sessions.len().saturating_sub(keep);
    for path in sessions.into_iter().take(remove) {
        let _ = fs::remove_dir_all(path);
    }
}

pub fn dump_ocr_debug(
    session_dir: &Path,
    step_no: usize,
    step_id: &str,
    crops: &[RgbaImage],
    region_labels: &[String],
    report: &OcrPipelineReport,
) -> Result<PathBuf, String> {
    let step_dir = session_dir.join(format!("step-{step_no:02}"));
    if step_dir.exists() {
        fs::remove_dir_all(&step_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&step_dir).map_err(|error| error.to_string())?;

    fs::write(step_dir.join("step-id.txt"), step_id).map_err(|error| error.to_string())?;

    for (index, crop) in crops.iter().enumerate() {
        let label = region_labels
            .get(index)
            .map(String::as_str)
            .unwrap_or("region");
        save_png(crop, &step_dir.join(format!("{index:02}-{label}-crop.png")))?;

        let ocr_input = prepare_ocr_input(crop);
        save_dynamic_png(
            &ocr_input,
            &step_dir.join(format!("{index:02}-{label}-ocr-input.png")),
        )?;
    }

    let summary_path = step_dir.join("summary.json");
    let json = serde_json::to_string_pretty(report).map_err(|error| error.to_string())?;
    fs::write(&summary_path, json).map_err(|error| error.to_string())?;

    update_session_step_count(session_dir, step_no)?;

    if let Ok(mut last) = LAST_DEBUG_DIR.lock() {
        *last = Some(session_dir.to_path_buf());
    }

    Ok(step_dir)
}

pub fn last_debug_dir() -> Option<PathBuf> {
    LAST_DEBUG_DIR.lock().ok().and_then(|guard| guard.clone())
}

pub fn open_last_debug_dir() -> Result<(), String> {
    let dir = last_debug_dir()
        .or_else(read_latest_pointer)
        .ok_or_else(|| "还没有 OCR 调试记录，请先开启调试并完成一次录制".to_string())?;

    if !dir.exists() {
        return Err("OCR 调试目录不存在，请重新录制".to_string());
    }

    open_in_file_manager(&dir)
}

fn update_session_step_count(session_dir: &Path, step_no: usize) -> Result<(), String> {
    let session_path = session_dir.join("session.json");
    let mut session_meta = if session_path.exists() {
        let raw = fs::read_to_string(&session_path).map_err(|error| error.to_string())?;
        serde_json::from_str(&raw).unwrap_or_else(|_| {
            json!({
                "startedAt": now_timestamp(),
                "stepCount": 0,
            })
        })
    } else {
        json!({
            "startedAt": now_timestamp(),
            "stepCount": 0,
        })
    };

    if let Some(obj) = session_meta.as_object_mut() {
        let current = obj
            .get("stepCount")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as usize;
        obj.insert("stepCount".into(), json!(current.max(step_no)));
        obj.insert("updatedAt".into(), json!(now_timestamp()));
    }

    fs::write(
        &session_path,
        serde_json::to_string_pretty(&session_meta).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn write_latest_pointer(root: &Path, session_dir: &Path) -> Result<(), String> {
    fs::write(root.join("LATEST.txt"), session_dir.display().to_string())
        .map_err(|error| error.to_string())
}

fn read_latest_pointer() -> Option<PathBuf> {
    read_latest_from_root(&ocr_debug_dir())
        .or_else(|| read_latest_from_root(&std::env::temp_dir().join("stepdoc-ocr-debug")))
}

fn read_latest_from_root(root: &Path) -> Option<PathBuf> {
    let latest = fs::read_to_string(root.join("LATEST.txt")).ok()?;
    let path = PathBuf::from(latest.trim());
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn save_png(image: &RgbaImage, path: &Path) -> Result<(), String> {
    image
        .save_with_format(path, ImageFormat::Png)
        .map_err(|error| error.to_string())
}

fn save_dynamic_png(image: &DynamicImage, path: &Path) -> Result<(), String> {
    image
        .save_with_format(path, ImageFormat::Png)
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn open_in_file_manager(path: &Path) -> Result<(), String> {
    std::process::Command::new("explorer")
        .arg(path)
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn open_in_file_manager(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "linux")]
    let program = "xdg-open";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let program = "xdg-open";

    std::process::Command::new(program)
        .arg(path)
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
}
