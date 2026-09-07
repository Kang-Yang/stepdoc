use std::path::PathBuf;

/// Persistent app data root, e.g. `%LOCALAPPDATA%\stepdoc` on Windows.
pub fn app_root() -> PathBuf {
    app_data_base().join("stepdoc")
}

/// RapidOCR model cache (persistent).
pub fn ocr_models_dir() -> PathBuf {
    app_root().join("ocr-models")
}

/// OCR debug dumps when debug mode is enabled (persistent).
pub fn ocr_debug_dir() -> PathBuf {
    app_root().join("ocr-debug")
}

/// Short-lived OCR frame files; kept in system temp for automatic cleanup.
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
