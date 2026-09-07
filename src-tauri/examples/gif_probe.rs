//! Dev probe: render a realistic 1920×1080 dark-mode step set through `render_gif` and write
//! the resulting GIF to a path, so the encoder's disposal/palette/quality can be inspected with
//! external tools (PIL, browsers).
//!
//! Usage: cargo run --release --example gif_probe -- <output.gif>

use std::path::PathBuf;

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{DynamicImage, Rgba, RgbaImage};
use stepdoc_lib::export::render_gif;
use stepdoc_lib::models::RecordedStep;

fn main() {
    let out: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("stepdoc-gif-probe.gif"));

    let steps = (1..=3)
        .map(|index| RecordedStep {
            id: format!("s{index}"),
            event_type: "left-click".to_string(),
            description: format!("第 {index} 步：点击「示例按钮 {index}」"),
            coordinates: Some(vec![600.0, 360.0]),
            image_base64: Some(encode(synthetic_frame(index))),
            timestamp: "2026-09-07T12:00:00".to_string(),
        })
        .collect::<Vec<_>>();

    let bytes = render_gif(&steps).expect("render_gif");
    std::fs::write(&out, &bytes).expect("write gif");
    println!("wrote {} ({} bytes, {}/3 steps)", out.display(), bytes.len(), steps.len());
    println!("dims = {}", gif_dims(&bytes).unwrap_or_default());
}

fn encode(image: RgbaImage) -> String {
    // `write_to` needs `Write + Seek`, so buffer through a `Cursor` instead of a bare `Vec`.
    let mut bytes = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("png encode");
    STANDARD.encode(bytes.into_inner())
}

fn gif_dims(bytes: &[u8]) -> Option<String> {
    if &bytes[0..6] == b"GIF89a" || &bytes[0..6] == b"GIF87a" {
        Some(format!("{}x{}", u16::from_le_bytes([bytes[6], bytes[7]]), u16::from_le_bytes([bytes[8], bytes[9]])))
    } else {
        None
    }
}

/// A dark-mode screenshot with a menu bar, a body with a couple of bright glyphs, and a marker
/// dot indicating where the "click" landed (different per step) to mimic real recordings.
fn synthetic_frame(step: usize) -> RgbaImage {
    let (w, h) = (1920u32, 1080u32);
    let bg = Rgba([26, 27, 38, 255]);
    let bar = Rgba([40, 42, 54, 255]);
    let light = Rgba([248, 250, 252, 255]);
    let accent = Rgba([255, 121, 198, 255]);
    let mut image = RgbaImage::from_pixel(w, h, bg);
    for y in 0..60 {
        for x in 0..w {
            image.put_pixel(x, y, bar);
        }
    }
    draw_text_row(&mut image, &["文件", "编辑", "选择", "查看", "运行"], 24, 18, light);
    draw_text_row(&mut image, &[&format!("步骤 {step}")], 24, 200, accent);
    // bright body block standing in for the clicked control
    let bx = 600 + (step as u32 - 1) * 40;
    for y in 600..660 {
        for x in bx..bx + 80 {
            image.put_pixel(x, y, light);
        }
    }
    // click marker
    for &(mx, my) in &[(960u32, 540u32)] {
        for dy in 0..8 {
            for dx in 0..8 {
                image.put_pixel(mx + dx, my + dy, accent);
            }
        }
    }
    image
}

fn draw_text_row(image: &mut RgbaImage, labels: &[&str], _size: u32, y: u32, color: Rgba<u8>) {
    let mut x = 30u32;
    for label in labels {
        for (i, _) in label.chars().enumerate() {
            let glyph_w = 28u32;
            for gy in 0..28 {
                for gx in 0..glyph_w {
                    if (gx + gy) % 3 == 0 || (gx as i32 - gy as i32).abs() < 6 {
                        image.put_pixel(x + gx, y + gy, color);
                    }
                }
            }
            x += glyph_w;
            let _ = i;
        }
        x += 30;
    }
}
