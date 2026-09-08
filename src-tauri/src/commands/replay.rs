use crate::models::RecordedStep;

/// 临时调试命令：返回上一次保存的录制步骤，供前端启动时恢复。
#[tauri::command]
pub fn load_last_recording() -> Result<Vec<RecordedStep>, String> {
    Ok(crate::last_recording::load())
}