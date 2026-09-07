use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use image::RgbaImage;

const FRAME_INTERVAL: Duration = Duration::from_millis(100);

/// A single monitor frame kept in memory so click handling never waits for a full screenshot.
#[derive(Clone)]
pub struct CachedScreenFrame {
    pub image: Arc<RgbaImage>,
    pub display_x: i32,
    pub display_y: i32,
    pub display_width: u32,
    pub display_height: u32,
}

impl CachedScreenFrame {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.display_x as f64
            && x < self.display_x as f64 + self.display_width as f64
            && y >= self.display_y as f64
            && y < self.display_y as f64 + self.display_height as f64
    }
}

pub struct ScreenshotCache {
    frames: RwLock<Vec<CachedScreenFrame>>,
    running: AtomicBool,
}

impl ScreenshotCache {
    pub fn new() -> Self {
        Self {
            frames: RwLock::new(Vec::new()),
            running: AtomicBool::new(false),
        }
    }

    pub fn start(self: &Arc<Self>) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let cache = self.clone();
        thread::spawn(move || {
            while cache.running.load(Ordering::SeqCst) {
                cache.refresh();
                thread::sleep(FRAME_INTERVAL);
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn frame_at(&self, x: f64, y: f64) -> Option<CachedScreenFrame> {
        self.frames
            .read()
            .ok()?
            .iter()
            .find(|frame| frame.contains(x, y))
            .cloned()
    }

    fn refresh(&self) {
        let Ok(screens) = screenshots::Screen::all() else {
            return;
        };

        let frames = screens
            .into_iter()
            .filter_map(|screen| {
                let info = screen.display_info;
                let image = screen.capture().ok()?;
                Some(CachedScreenFrame {
                    image: Arc::new(image),
                    display_x: info.x,
                    display_y: info.y,
                    display_width: info.width,
                    display_height: info.height,
                })
            })
            .collect();

        if let Ok(mut current) = self.frames.write() {
            *current = frames;
        }
    }
}

impl Default for ScreenshotCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_frame_containing_the_click() {
        let cache = ScreenshotCache {
            frames: RwLock::new(vec![CachedScreenFrame {
                image: Arc::new(RgbaImage::new(1920, 1080)),
                display_x: -1920,
                display_y: 0,
                display_width: 1920,
                display_height: 1080,
            }]),
            running: AtomicBool::new(false),
        };

        assert!(cache.frame_at(-100.0, 200.0).is_some());
        assert!(cache.frame_at(10.0, 200.0).is_none());
    }
}
