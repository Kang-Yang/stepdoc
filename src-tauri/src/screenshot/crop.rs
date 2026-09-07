use image::RgbaImage;

struct HeuristicCropSpec {
    width: u32,
    height: u32,
    offset_x: i64,
    offset_y: i64,
}

const HEURISTIC_CROP_REGIONS: &[HeuristicCropSpec] = &[
    HeuristicCropSpec {
        width: 240,
        height: 72,
        offset_x: 0,
        offset_y: 0,
    },
    HeuristicCropSpec {
        width: 280,
        height: 72,
        offset_x: 56,
        offset_y: 0,
    },
    HeuristicCropSpec {
        width: 220,
        height: 56,
        offset_x: 0,
        offset_y: -40,
    },
    HeuristicCropSpec {
        width: 220,
        height: 56,
        offset_x: 0,
        offset_y: 40,
    },
];

pub const HEURISTIC_REGION_LABELS: [&str; 4] = ["center", "right", "above", "below"];

pub struct PlannedOcrCrops {
    pub crops: Vec<RgbaImage>,
    pub labels: Vec<String>,
    /// Screenshot-space top-left origin of each crop, aligned with `crops`.
    pub crop_origins: Vec<(u32, u32)>,
    /// Screenshot-space absolute click point.
    pub click_x: f64,
    pub click_y: f64,
    pub mark_x: f64,
    pub mark_y: f64,
    pub capture_source: String,
}

pub fn plan_ocr_crops(
    image: &RgbaImage,
    x: f64,
    y: f64,
    display_x: i32,
    display_y: i32,
    display_width: u32,
    display_height: u32,
) -> PlannedOcrCrops {
    let scale_x = image.width() as f64 / display_width.max(1) as f64;
    let scale_y = image.height() as f64 / display_height.max(1) as f64;
    let mark_x = click_mark_x(x, display_x, scale_x);
    let mark_y = click_mark_y(y, display_y, scale_y);
    let center_x = mark_x.round() as i64;
    let center_y = mark_y.round() as i64;

    let mut crops = Vec::new();
    let mut crop_origins = Vec::new();
    for spec in HEURISTIC_CROP_REGIONS {
        let (crop, origin) = crop_at_with_origin(
            image,
            center_x + scale_value(spec.offset_x as f64, scale_x),
            center_y + scale_value(spec.offset_y as f64, scale_y),
            scaled_dimension(spec.width, scale_x),
            scaled_dimension(spec.height, scale_y),
        );
        crops.push(crop);
        crop_origins.push(origin);
    }
    let labels = HEURISTIC_REGION_LABELS
        .iter()
        .map(|label| (*label).to_string())
        .collect::<Vec<_>>();

    PlannedOcrCrops {
        labels,
        crops,
        crop_origins,
        click_x: mark_x,
        click_y: mark_y,
        mark_x,
        mark_y,
        capture_source: "click".to_string(),
    }
}

fn click_mark_x(x: f64, display_x: i32, scale_x: f64) -> f64 {
    (x - display_x as f64) * scale_x
}

fn click_mark_y(y: f64, display_y: i32, scale_y: f64) -> f64 {
    (y - display_y as f64) * scale_y
}

fn scale_value(value: f64, scale: f64) -> i64 {
    (value * scale).round() as i64
}

fn scaled_dimension(value: u32, scale: f64) -> u32 {
    (value as f64 * scale).round().max(1.0) as u32
}

#[cfg(test)]
fn crop_at(
    image: &RgbaImage,
    center_x: i64,
    center_y: i64,
    crop_width: u32,
    crop_height: u32,
) -> RgbaImage {
    crop_at_with_origin(image, center_x, center_y, crop_width, crop_height).0
}

fn crop_at_with_origin(
    image: &RgbaImage,
    center_x: i64,
    center_y: i64,
    crop_width: u32,
    crop_height: u32,
) -> (RgbaImage, (u32, u32)) {
    let (image_w, image_h) = image.dimensions();
    if image_w == 0 || image_h == 0 {
        return (RgbaImage::new(1, 1), (0, 0));
    }
    let crop_width = crop_width.min(image_w).max(1);
    let crop_height = crop_height.min(image_h).max(1);
    let half_w = crop_width as i64 / 2;
    let half_h = crop_height as i64 / 2;

    // Shift the requested crop inward at a screen edge instead of a shrinking the crop. Keeping the
    // target near the crop centre makes the OCR candidate-distance ranking reliable.
    let left = (center_x - half_w).clamp(0, image_w.saturating_sub(crop_width) as i64) as u32;
    let top = (center_y - half_h).clamp(0, image_h.saturating_sub(crop_height) as i64) as u32;

    let crop = image::imageops::crop_imm(image, left, top, crop_width, crop_height).to_image();
    (crop, (left, top))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn crops_are_centered_on_click() {
        let mut image = RgbaImage::new(400, 300);
        for y in 0..300 {
            for x in 0..400 {
                image.put_pixel(x, y, Rgba([x as u8, y as u8, 0, 255]));
            }
        }

        let planned = plan_ocr_crops(&image, 120.0, 90.0, 0, 0, 400, 300);
        assert_eq!(planned.crops.len(), 4);
        assert_eq!(planned.mark_x, 120.0);
        assert_eq!(planned.mark_y, 90.0);

        let center = &planned.crops[0];
        let (w, h) = center.dimensions();
        assert_eq!(w, 240);
        assert_eq!(h, 72);
        // center crop at (120,90) -> left=0 top=54
        assert_eq!(center.get_pixel(120, 36).0[0], 120);
        assert_eq!(center.get_pixel(120, 36).0[1], 90);
    }

    #[test]
    fn maps_click_using_actual_image_dimensions() {
        let image = RgbaImage::new(1500, 1000);

        let planned = plan_ocr_crops(&image, 120.0, 90.0, 0, 0, 1200, 800);

        assert_eq!(planned.mark_x, 150.0);
        assert_eq!(planned.mark_y, 112.5);
        assert_eq!(planned.crops[0].dimensions(), (300, 90));
    }

    #[test]
    fn keeps_the_requested_crop_size_at_screen_edges() {
        let image = RgbaImage::new(400, 300);

        let crop = crop_at(&image, 395, 295, 240, 72);

        assert_eq!(crop.dimensions(), (240, 72));
    }
}
