#[cfg(target_os = "windows")]
pub fn cursor_position() -> Option<(f64, f64)> {
    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    extern "system" {
        fn GetCursorPos(point: *mut Point) -> i32;
    }

    let mut point = Point { x: 0, y: 0 };
    unsafe {
        if GetCursorPos(&mut point) != 0 {
            return Some((point.x as f64, point.y as f64));
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn cursor_position() -> Option<(f64, f64)> {
    None
}
