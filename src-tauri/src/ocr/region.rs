use image::{DynamicImage, RgbaImage};

/// Prepare a crop for OCR. RapidOCR rec-only mode resizes to a fixed height internally,
/// so avoid pre-upscaling here — it only adds an extra interpolation pass.
pub fn prepare_ocr_input(image: &RgbaImage) -> DynamicImage {
    DynamicImage::ImageRgba8(image.clone())
}
