use image::{DynamicImage, RgbaImage};

/// 为 OCR 准备一张裁剪图。RapidOCR 的 rec-only 模式在内部会把图像缩放到固定高度，
/// 所以这里不要预先放大——那样只会增加一次多余的插值处理。
pub fn prepare_ocr_input(image: &RgbaImage) -> DynamicImage {
    DynamicImage::ImageRgba8(image.clone())
}
