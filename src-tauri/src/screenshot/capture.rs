use super::encode::encode_base64_jpeg;
use super::{annotate_click, crop::plan_ocr_crops, CachedScreenFrame};

pub struct ClickCapture {
    pub display_base64: String,
    pub ocr_crops: Vec<image::RgbaImage>,
    pub region_labels: Vec<String>,
    pub crop_origins: Vec<(u32, u32)>,
    pub click_x: f64,
    pub click_y: f64,
    pub capture_source: String,
}

pub fn capture_click_assets(x: f64, y: f64) -> Result<ClickCapture, String> {
    let screen =
        screenshots::Screen::from_point(x as i32, y as i32).map_err(|error| error.to_string())?;
    let info = screen.display_info;
    let image = screen.capture().map_err(|error| error.to_string())?;

    build_click_assets(image, x, y, info.x, info.y, info.width, info.height)
}

pub fn capture_click_assets_from_frame(
    frame: &CachedScreenFrame,
    x: f64,
    y: f64,
) -> Result<ClickCapture, String> {
    let mut capture = build_click_assets(
        (*frame.image).clone(),
        x,
        y,
        frame.display_x,
        frame.display_y,
        frame.display_width,
        frame.display_height,
    )?;
    capture.capture_source = "cached-click".to_string();
    Ok(capture)
}

fn build_click_assets(
    mut image: image::RgbaImage,
    x: f64,
    y: f64,
    display_x: i32,
    display_y: i32,
    display_width: u32,
    display_height: u32,
) -> Result<ClickCapture, String> {
    let planned = plan_ocr_crops(
        &image,
        x,
        y,
        display_x,
        display_y,
        display_width,
        display_height,
    );

    annotate_click(&mut image, planned.mark_x, planned.mark_y);

    Ok(ClickCapture {
        display_base64: encode_base64_jpeg(image)?,
        ocr_crops: planned.crops,
        region_labels: planned.labels,
        crop_origins: planned.crop_origins,
        click_x: planned.click_x,
        click_y: planned.click_y,
        capture_source: planned.capture_source,
    })
}
