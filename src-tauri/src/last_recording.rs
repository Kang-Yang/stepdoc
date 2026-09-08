//! 临时调试辅助：把最后一次录制结果落到磁盘，下次启动时重新读入。
//!
//! 仅用于开发期调试 GIF 导出等流程，避免每次都要手动点一遍录制。属功能层面
//! 的临时代码，验证结束后应整体移除。

use crate::models::RecordedStep;
use crate::utils::app_root;

fn cache_file() -> std::path::PathBuf {
    app_root().join("last-recording.json")
}

/// 停止录制时把步骤序列化保存，供下一次启动时恢复。
pub fn save(steps: &[RecordedStep]) {
    let Ok(json) = serde_json::to_string(steps) else {
        return;
    };
    let file = cache_file();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(file, json);
}

/// 读取上次保存的录制步骤；没有则返回空列表。
pub fn load() -> Vec<RecordedStep> {
    std::fs::read(cache_file())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}