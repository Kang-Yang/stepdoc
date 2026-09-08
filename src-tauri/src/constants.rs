use std::time::Duration;

pub const CLICK_DEBOUNCE_MS: u128 = 500;
pub const CLICK_DEBOUNCE_DISTANCE: f64 = 12.0;
/// 存储步骤截图所用的 JPEG 质量。低于约 90 时，ClearType 文本会出现明显的振铃伪影，
/// 而当帧被缩放到 GIF 并量化后，这种现象只会更严重。
pub const JPEG_QUALITY: u8 = 90;
/// 导出 GIF 画布的宽度上限。等于或低于该值的屏幕保留其原生像素——与录屏工具一致——
/// 因此界面文本依然清晰；只有 2K/4K 的捕获才会被缩小。
pub const GIF_EXPORT_MAX_WIDTH: u32 = 1920;
pub const GIF_FRAME_DELAY_MS: u32 = 1800;

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const RECORDING_BAR_LABEL: &str = "recording-bar";
pub const RECORDING_BAR_WIDTH: f64 = 400.0;

pub const EVENT_RECORDING_STARTED: &str = "recording-started";
pub const EVENT_RECORDING_FINALIZING: &str = "recording-finalizing";
pub const EVENT_RECORDING_STOPPED: &str = "recording-stopped";
pub const EVENT_RECORDING_PAUSED: &str = "recording-paused";
pub const EVENT_RECORDING_RESUMED: &str = "recording-resumed";
pub const EVENT_RECORDING_STEP: &str = "recording-step";
pub const EVENT_RECORDING_STEP_UPDATED: &str = "recording-step-updated";
pub const EVENT_OCR_PROGRESS: &str = "ocr-progress";
/// 当用户按下全局录制快捷键时，在主窗口上触发；窗口负责发起/停止的完整流程（选项、
/// 确认对话框、忙碌状态保护），并对此事件作出响应。
pub const EVENT_HOTKEY_RECORDING_TOGGLE: &str = "hotkey-recording-toggle";

/// `stop_recording` 在发出最终步骤之前，等待进行中的捕获/OCR 工作所耗的时间。
/// 这里刻意留得很宽裕：快速点击很容易堆积数分钟的 CPU 密集型 OCR，而如果过早放弃，
/// 就会在进度条结束后，真实结果还要过几秒才陆续到位时，把占位描述留在屏幕上。
pub const STOP_DRAIN_TIMEOUT: Duration = Duration::from_secs(300);
