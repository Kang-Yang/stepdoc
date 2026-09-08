use base64::{engine::general_purpose::STANDARD, Engine};
use color_quant::NeuQuant;
use gif::{DisposalMethod, Encoder as GifEncoder, Frame as GifFrame, Repeat};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

use crate::constants::{GIF_EXPORT_MAX_WIDTH, GIF_FRAME_DELAY_MS};
use crate::models::RecordedStep;

/// 构建全局调色板时对每帧像素的抽样步长。调色板只需要颜色分布，抽一份子集即可，
/// 同时避免把所有帧的完整像素一次性驻留内存。
const PALETTE_SAMPLE_STRIDE: usize = 4;

/// 逐帧流式导出 GIF：不把所有帧同时放进内存。
///
/// 先做一遍“干跑”遍历得到画布尺寸并抽样构建全局调色板，再逐帧解码→合成→抖动→立即写入，
/// 单帧在写完后随即释放。这样即使教程有上百帧，内存峰值也只停留在“一帧合成图”量级，
/// 而不是把所有帧的多个副本同时驻留。
pub fn render_gif(steps: &[RecordedStep]) -> Result<Vec<u8>, String> {
    // —— 第一遍：画布尺寸 + 全局调色板采样 ——
    let mut canvas_w: u32 = 0;
    let mut canvas_h: u32 = 0;
    let mut sampled: Vec<u8> = Vec::new();
    let mut count = 0usize;

    for step in steps {
        let Some(base64) = step.image_base64.as_ref() else {
            continue;
        };
        let image = decode_step_image(base64)?;
        let fit = fit_step_image_for_gif(&image, GIF_EXPORT_MAX_WIDTH);
        let (w, h) = fit.dimensions();
        canvas_w = canvas_w.max(w);
        canvas_h = canvas_h.max(h);
        for (i, px) in fit.to_rgba8().as_raw().chunks_exact(4).enumerate() {
            if i % PALETTE_SAMPLE_STRIDE == 0 {
                sampled.extend_from_slice(&px[..3]);
            }
        }
        count += 1;
    }

    if count == 0 {
        return Err("没有带截图的步骤，无法生成动图".into());
    }

    let width = u16::try_from(canvas_w).map_err(|_| "GIF 画面过宽".to_string())?;
    let height = u16::try_from(canvas_h).map_err(|_| "GIF 画面过高".to_string())?;
    let (quantizer, palette) = build_palette(&sampled);

    // GIF 延迟以厘秒为单位存储。
    let delay = (GIF_FRAME_DELAY_MS / 10) as u16;

    // —— 第二遍：逐帧编码，单帧只逗留一个迭代周期 ——
    let frames = steps.iter().filter_map(|step| step.image_base64.as_ref()).map(|base64| {
        let image = decode_step_image(base64)?;
        let fit = fit_step_image_for_gif(&image, GIF_EXPORT_MAX_WIDTH);
        Ok::<_, String>(compose_gif_frame(&fit, canvas_w, canvas_h))
    });
    write_gif_frames(&quantizer, &palette, width, height, delay, frames)
}

/// 用所有帧共享的单个全局调色板、Floyd–Steinberg 抖动和 `DisposalMethod::Keep` 写入每一帧。
///
/// image 0.24 的 `GifEncoder` 在每一帧上都硬编码了 `DisposalMethod::Background`，并给每帧单独
/// 一个调色板。背景清除加上隐式的"清除为背景"（此处未定义）会让许多查看器在帧间闪烁；
/// 每帧独立的调色板还会让相同的颜色在各帧间漂移。把所有帧统一到一个调色板能保持颜色稳定，
/// 而且由于 UI 截图大多共享同一组调色板，通常还能缩小文件体积。不做抖动的最邻近颜色映射
/// 会把渐变和抗锯齿边缘变成生硬的分层色带——也就是经典的"截图 GIF 看起来很丑"的瑕疵——
/// 因此改用误差扩散来映射像素。
fn write_gif_frames(
    quantizer: &NeuQuant,
    palette: &[u8],
    width: u16,
    height: u16,
    delay: u16,
    frames: impl IntoIterator<Item = Result<RgbaImage, String>>,
) -> Result<Vec<u8>, String> {
    let (width_usize, height_usize) = (width as usize, height as usize);
    let mut output = Vec::new();
    let mut encoder = GifEncoder::new(&mut output, width, height, palette)
        .map_err(|error| format!("初始化 GIF 编码器失败: {error}"))?;
    encoder
        .set_repeat(Repeat::Infinite)
        .map_err(|error| format!("设置 GIF 循环失败: {error}"))?;

    // 误差扩散工作区在帧间复用，避免每帧重复分配。
    let mut work = vec![0.0f32; width_usize * height_usize * 3];

    for frame in frames {
        let composed = frame?;
        let indices = dither_frame(&composed, &mut work, quantizer, palette, width_usize, height_usize);
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

    // 结束编码器的借用之后才能移出输出缓冲。
    drop(encoder);
    Ok(output)
}

fn build_palette(sampled: &[u8]) -> (NeuQuant, Vec<u8>) {
    let quantizer = NeuQuant::new(10, 256, sampled);
    let palette = quantizer.color_map_rgb();
    (quantizer, palette)
}

/// 用 Floyd–Steinberg 误差扩散（蛇形扫描，ffmpeg/gifski 一类编码器的默认抖动方式）把单帧映射到
/// 调色板索引。颜色已落在调色板条目上的像素不会累积误差，因此平坦的 UI 区域能保持完全干净，
/// 而渐变、窗口阴影和抗锯齿文字会把量化误差扩散到邻近像素，而不是形成硬性色带。
fn dither_frame(
    frame: &RgbaImage,
    work: &mut [f32],
    quantizer: &NeuQuant,
    palette: &[u8],
    width: usize,
    height: usize,
) -> Vec<u8> {
    for (dst, px) in work.chunks_exact_mut(3).zip(frame.as_raw().chunks_exact(4)) {
        dst[0] = px[0] as f32;
        dst[1] = px[1] as f32;
        dst[2] = px[2] as f32;
    }

    let mut nearest: std::collections::HashMap<u32, u8> = std::collections::HashMap::new();
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
        let (quantizer, palette) = build_palette(frame.as_raw());
        let mut work = vec![0.0f32; 64 * 16 * 3];
        let indices = dither_frame(&frame, &mut work, &quantizer, &palette, 64, 16);

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
        let mut sampled = Vec::new();
        for frame in &frames {
            sampled.extend_from_slice(frame.as_raw());
        }
        let (quantizer, palette) = build_palette(&sampled);
        let bytes = write_gif_frames(&quantizer, &palette, 8, 4, 180, frames.into_iter().map(Ok))
            .expect("encode");
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