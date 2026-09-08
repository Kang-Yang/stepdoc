//! 临时诊断：把采集链路的关键节点追加到 `%TEMP%\stepdoc-capture-debug.log`，
//! 用于定位"录制时点击没登记成步骤"的问题。验证完成后整体移除。

pub const ENABLED: bool = true;

pub fn log(line: impl AsRef<str>) {
    if !ENABLED {
        return;
    }
    use std::io::Write;
    let path = std::env::temp_dir().join("stepdoc-capture-debug.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "[{}] {}", now_label(), line.as_ref());
    }
}

fn now_label() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    format!("{ms}ms")
}