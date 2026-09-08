use std::path::PathBuf;

/// 持久化应用数据根目录，例如 Windows 下的 `%LOCALAPPDATA%\stepdoc`。
pub fn app_root() -> PathBuf {
    app_data_base().join("stepdoc")
}

/// RapidOCR 模型缓存（持久化）。
pub fn ocr_models_dir() -> PathBuf {
    app_root().join("ocr-models")
}

/// 调试模式开启时的 OCR 调试转储（持久化）。
pub fn ocr_debug_dir() -> PathBuf {
    app_root().join("ocr-debug")
}

/// 短生命周期的 OCR 帧文件；放在系统临时目录以便自动清理。
pub fn ocr_frames_dir() -> PathBuf {
    std::env::temp_dir().join("stepdoc-ocr-frames")
}

fn app_data_base() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
    }

    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Library/Application Support"))
            .unwrap_or_else(std::env::temp_dir);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(xdg);
        }
        return std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".local/share"))
            .unwrap_or_else(std::env::temp_dir);
    }
}
