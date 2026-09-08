use base64::{engine::general_purpose::STANDARD, Engine};
use color_quant::NeuQuant;
use gif::{DisposalMethod, Encoder as GifEncoder, Frame as GifFrame, Repeat};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use std::collections::HashMap;

use crate::constants::{GIF_EXPORT_MAX_WIDTH, GIF_FRAME_DELAY_MS};
use crate::models::RecordedStep;

struct GifFrameInput {
    image: DynamicImage,
}

pub fn render_gif(steps: &[RecordedStep]) -> Result<Vec<u8>, String> {
    let mut frames: Vec<GifFrameInput> = Vec::new();

    for step in steps {
        let Some(base64) = step.image_base64.as_ref() else {
            continue;
        };
        let image = decode_step_image(base64)?;
        frames.push(GifFrameInput {
            image: fit_step_image_for_gif(&image, GIF_EXPORT_MAX_WIDTH),
        });
    }

    if frames.is_empty() {
        return Err("没有带截图的步骤，无法生成动图".into());
    }

    let canvas_w = frames
        .iter()
        .map(|frame| frame.image.dimensions().0)
        .max()
        .unwrap_or(GIF_EXPORT_MAX_WIDTH);
    let canvas_h = frames
        .iter()
        .map(|frame| frame.image.dimensions().1)
        .max()
        .unwrap_or(360);

    let composed: Vec<RgbaImage> = frames
        .iter()
        .map(|frame| compose_gif_frame(&frame.image, canvas_w, canvas_h))
        .collect();

    encode_gif(&composed, GIF_FRAME_DELAY_MS)
}

/// 用所有帧共享的单个全局调色板、Floyd–Steinberg 抖动和 `DisposalMethod::Keep` 写入每一帧。
///
/// image 0.24 的 `GifEncoder` 在每一帧上都硬编码了 `DisposalMethod::Background`，并给每帧单独
/// 一个调色板。背景清除加上隐式的"清除为背景"（此处未定义）会让许多查看器在帧间闪烁；
/// 每帧独立的调色板还会让相同的颜色在各帧间漂移。把所有帧统一到一个调色板能保持颜色稳定，
/// 而且由于 UI 截图大多共享同一组调色板，通常还能缩小文件体积。不做抖动的最邻近颜色映射
/// 会把渐变和抗锯齿边缘变成生硬的分层色带——也就是经典的"截图 GIF 看起来很丑"的瑕疵——
/// 因此改用误差扩散来映射像素。
fn encode_gif(frames: &[RgbaImage], delay_ms: u32) -> Result<Vec<u8>, String> {
    let (width, height) = frames.first().map(|frame| frame.dimensions()).unwrap_or((1, 1));
    let width = u16::try_from(width).map_err(|_| "GIF 画面过宽".to_string())?;
    let height = u16::try_from(height).map_err(|_| "GIF 画面过高".to_string())?;

    let mut sampled = Vec::new();
    for frame in frames {
        sampled.extend_from_slice(frame.as_raw());
    }
    let quantizer = NeuQuant::new(10, 256, &sampled);
    let palette = quantizer.color_map_rgb();
    let indexed_frames = dither_frames(frames, &quantizer, &palette, width as usize, height as usize);

    let mut output = Vec::new();
    let result = (|| -> Result<(), String> {
        let mut encoder = GifEncoder::new(&mut output, width, height, &palette)
            .map_err(|error| format!("初始化 GIF 编码器失败: {error}"))?;
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|error| format!("设置 GIF 循环失败: {error}"))?;

        // GIF 延迟以厘秒为单位存储。
        let delay = (delay_ms / 10) as u16;
        for indices in &indexed_frames {
            let gif_frame = GifFrame {
                delay,
                dispose: DisposalMethod::Keep,
                width,
                height,
                buffer: std::borrow::Cow::Borrowed(indices.as_slice()),
                ..Default::default()
            };
            encoder
                .write_frame(&gif_frame)
                .map_err(|error| format!("写入 GIF 帧失败: {error}"))?;
        }
        Ok(())
    })();
    result?;

    Ok(output)
}

/// 用 Floyd–Steinberg 误差扩散（蛇形扫描，ffmpeg/gifski 一类编码器的默认抖动方式）把每一帧映射到
/// 调色板索引。颜色已落在调色板条目上的像素不会累积误差，因此平坦的 UI 区域能保持完全干净，
/// 而渐变、窗口阴影和抗锯齿文字会把量化误差扩散到邻近像素，而不是形成硬性色带。
fn dither_frames(
    frames: &[RgbaImage],
    quantizer: &NeuQuant,
    palette: &[u8],
    width: usize,
    height: usize,
) -> Vec<Vec<u8>> {
    let mut work = vec![0.0f32; width * height * 3];
    let mut nearest: HashMap<u32, u8> = HashMap::new();

    frames
        .iter()
        .map(|frame| {
            for (dst, px) in work.chunks_exact_mut(3).zip(frame.as_raw().chunks_exact(4)) {
                dst[0] = px[0] as f32;
                dst[1] = px[1] as f32;
                dst[2] = px[2] as f32;
            }

            let mut indices = vec![0u8; width * height];
            for y in 0..height {
                let left_to_right = y % 2 == 0;
                for step in 0..width {
                    let x = if left_to_right { step } else { width - 1 - step };
                    let i = y * width + x;
                    let (r, g, b) = {
                        let px = &work[i * 3..i * 3 + 3];
                        (
                            px[0].clamp(0.0, 255.0),
                            px[1].clamp(0.0, 255.0),
                            px[2].clamp(0.0, 255.0),
                        )
                    };
                    let key = ((r as u8 as u32) << 16) | ((g as u8 as u32) << 8) | b as u8 as u32;
                    let idx = *nearest.entry(key).or_insert_with(|| {
                        let rgba = [r as u8, g as u8, b as u8, 255];
                        quantizer.index_of(&rgba) as u8
                    });
                    indices[i] = idx;

                    let mapped = &palette[idx as usize * 3..idx as usize * 3 + 3];
                    let (er, eg, eb) = (
                        r - mapped[0] as f32,
                        g - mapped[1] as f32,
                        b - mapped[2] as f32,
                    );
                    let dir = if left_to_right { 1i32 } else { -1i32 };
                    let mut push = |dx: i32, dy: i32, weight: f32| {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                            return;
                        }
                        let j = (ny as usize * width + nx as usize) * 3;
                        work[j] += er * weight;
                        work[j + 1] += eg * weight;
                        work[j + 2] += eb * weight;
                    };
                    push(dir, 0, 7.0 / 16.0);
                    push(-dir, 1, 3.0 / 16.0);
                    push(0, 1, 5.0 / 16.0);
                    push(dir, 1, 1.0 / 16.0);
                }
            }
            indices
        })
        .collect()
}

fn decode_step_image(base64: &str) -> Result<DynamicImage, String> {
    let bytes = STANDARD
        .decode(base64)
        .map_err(|error| error.to_string())?;
    image::load_from_memory(&bytes).map_err(|error| error.to_string())
}

fn fit_step_image_for_gif(image: &DynamicImage, max_width: u32) -> DynamicImage {
    let (width, height) = image.dimensions();
    if width <= max_width {
        return image.clone();
    }

    let new_height = (height as f64 * max_width as f64 / width as f64).round() as u32;
    image.resize_exact(max_width, new_height, image::imageops::FilterType::Lanczos3)
}

fn compose_gif_frame(image: &DynamicImage, canvas_w: u32, canvas_h: u32) -> RgbaImage {
    let mut canvas =
        RgbaImage::from_pixel(canvas_w, canvas_h, Rgba([248, 250, 252, 255]));
    let (image_w, image_h) = image.dimensions();
    let x = (canvas_w.saturating_sub(image_w)) / 2;
    let y = (canvas_h.saturating_sub(image_h)) / 2;
    image::imageops::overlay(&mut canvas, image, x.into(), y.into());

    canvas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_gif_frame_has_expected_dimensions() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(320, 200, Rgba([0, 0, 0, 255])));
        let frame = compose_gif_frame(&image, 640, 200);
        assert_eq!(frame.dimensions(), (640, 200));
    }

    /// 抖动必须让平坦区域保持无噪声（那里的量化误差为零），同时把渐变展开到多个调色板条目，
    /// 而不是形成硬性色带。
    #[test]
    fn dither_keeps_flats_clean_and_spreads_gradients() {
        let mut frame = RgbaImage::new(64, 16);
        for y in 0..16 {
            for x in 0..64 {
let value = if x < 32 {
                        26 // 左半部分平坦的深色背景
                    } else {
                        (x * 4) as u8 // 右半部分递增的渐变
                    };
                frame.put_pixel(x, y, Rgba([value, value, value, 255]));
            }
        }
        let quantizer = NeuQuant::new(1, 256, frame.as_raw());
        let palette = quantizer.color_map_rgb();
        let indexed = dither_frames(&[frame], &quantizer, &palette, 64, 16);
        let indices = &indexed[0];

        let flat: std::collections::HashSet<u8> = indices[..32].iter().copied().collect();
        assert_eq!(flat.len(), 1, "flat region must map to exactly one palette entry");

        let gradient: std::collections::HashSet<u8> = indices[32..].iter().copied().collect();
        assert!(gradient.len() >= 4, "gradient must spread over multiple palette entries, got {gradient:?}");
    }

    /// Disposal 必须为 Keep（无背景闪烁），延迟必须能经受住厘秒换算，循环必须为无限循环，
    /// 并且每一帧都必须按顺序存在。
    #[test]
    fn encodes_keep_disposal_infinite_loop_and_delay() {
        let frames = vec![
            RgbaImage::from_pixel(8, 4, Rgba([10, 10, 10, 255])),
            RgbaImage::from_pixel(8, 4, Rgba([20, 20, 20, 255])),
            RgbaImage::from_pixel(8, 4, Rgba([30, 30, 30, 255])),
        ];
        let bytes = encode_gif(&frames, 1800).expect("encode");
        assert_eq!(&bytes[0..6], b"GIF89a");

        let mut offset = 13;
        let gct_size = 3 * (2 << (bytes[10] & 7));
        offset += gct_size;
        let mut gces = Vec::new();
        while offset + 8 < bytes.len() {
            if bytes[offset] == 0x3B {
                break;
            }
            if bytes[offset] == 0x21 && bytes[offset + 1] == 0xF9 {
                let packed = bytes[offset + 3];
                let delay = (bytes[offset + 4] as u16) | ((bytes[offset + 5] as u16) << 8);
                gces.push((packed, delay));
                offset += 8;
            } else if bytes[offset] == 0x21 {
                offset += 2;
                while bytes[offset] != 0 {
                    offset += bytes[offset] as usize + 1;
                }
                offset += 1;
            } else if bytes[offset] == 0x2C {
                let flags = bytes[offset + 9];
                offset += 10;
                if flags & 0x80 != 0 {
                    offset += 2 << (flags & 7);
                }
                offset += 1;
                while bytes[offset] != 0 {
                    offset += bytes[offset] as usize + 1;
                }
                offset += 1;
            } else {
                break;
            }
        }

        assert_eq!(gces.len(), 3, "expected one GCE per frame");
        for (packed, delay) in &gces {
            assert_eq!((packed >> 2) & 7, 1, "expected DisposalMethod::Keep (1)");
            assert_eq!(*delay, 180, "1800ms must round-trip as 180 centiseconds");
            assert_eq!(packed & 1, 0, "no transparency expected");
        }
    }
}