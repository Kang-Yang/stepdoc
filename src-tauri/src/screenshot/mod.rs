mod annotate;
mod cache;
mod capture;
mod crop;
mod encode;

pub use annotate::annotate_click;
pub use cache::{CachedScreenFrame, ScreenshotCache};
pub use capture::{capture_click_assets, capture_click_assets_from_frame, ClickCapture};
pub use crop::HEURISTIC_REGION_LABELS;
