use base64::{engine::general_purpose::STANDARD, Engine};
use image::codecs::jpeg::JpegEncoder;

use crate::constants::JPEG_QUALITY;

pub fn encode_base64_jpeg(image: image::RgbaImage) -> Result<String, String> {
    let rgb = image::DynamicImage::ImageRgba8(image).into_rgb8();
    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY);
    encoder
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ColorType::Rgb8,
        )
        .map_err(|error| error.to_string())?;
    Ok(STANDARD.encode(bytes))
}
