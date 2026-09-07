use rdev::Button;

use crate::screenshot::CachedScreenFrame;

pub enum CaptureJob {
    Click {
        button: Button,
        x: f64,
        y: f64,
        cached_frame: Option<CachedScreenFrame>,
    },
}
