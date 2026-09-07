use std::time::Duration;

pub const CLICK_DEBOUNCE_MS: u128 = 500;
pub const CLICK_DEBOUNCE_DISTANCE: f64 = 12.0;
/// JPEG quality for stored step screenshots. Below ~90, ClearType text shows visible ringing
/// that only gets worse when the frame is downscaled and quantized into the GIF.
pub const JPEG_QUALITY: u8 = 90;
/// Width cap for the exported GIF canvas. Screens at or below this keep their native pixels —
/// same as screen recorders — so UI text stays sharp; only 2K/4K captures get downscaled.
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
/// Fired on the main window when the user presses the global recording hotkey; the window
/// owns the start/stop flow (options, confirm dialog, busy guards) and reacts to it.
pub const EVENT_HOTKEY_RECORDING_TOGGLE: &str = "hotkey-recording-toggle";

/// How long `stop_recording` waits for in-flight capture/OCR work before emitting the final
/// steps. Generous on purpose: rapid clicking easily stacks minutes of CPU-bound OCR, and
/// giving up early is what left placeholder descriptions on screen after the progress bar
/// ended while the real results trickled in seconds later.
pub const STOP_DRAIN_TIMEOUT: Duration = Duration::from_secs(300);
