use crate::ocr;

#[tauri::command]
pub fn open_ocr_debug_folder() -> Result<(), String> {
    ocr::open_last_debug_dir()
}
