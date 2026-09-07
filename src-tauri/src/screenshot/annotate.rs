use image::RgbaImage;

/// StepDoc brand teal — matches `--brand-light` in the UI.
const RING: [u8; 4] = [20, 184, 166, 255];
/// Deeper teal for the center focal dot.
const CORE: [u8; 4] = [15, 118, 110, 255];
/// Outer echo ring — matches recording-bar accent `#5eead4`.
const ECHO: [u8; 4] = [94, 234, 212, 255];
const HALO: [u8; 4] = [255, 255, 255, 255];
const SHADOW: [u8; 4] = [28, 35, 51, 255];

const ECHO_OUTER: i64 = 28;
const ECHO_INNER: i64 = 26;
const HALO_OUTER: i64 = 25;
const HALO_INNER: i64 = 22;
const RING_OUTER: i64 = 21;
const RING_INNER: i64 = 17;
const SHADOW_OUTER: i64 = 18;
const SHADOW_INNER: i64 = 17;
const CORE_RADIUS: i64 = 3;

/// Draws the StepDoc click marker: echo ring + halo + main ring + center dot.
pub fn annotate_click(image: &mut RgbaImage, x: f64, y: f64) {
    let cx = x.round() as i64;
    let cy = y.round() as i64;

    // Back to front — each layer adds contrast on varied backgrounds.
    draw_ring(image, cx, cy, ECHO_INNER, ECHO_OUTER, ECHO);
    draw_ring(image, cx, cy, HALO_INNER, HALO_OUTER, HALO);
    draw_ring(image, cx, cy, SHADOW_INNER, SHADOW_OUTER, SHADOW);
    draw_ring(image, cx, cy, RING_INNER, RING_OUTER, RING);
    draw_disk(image, cx, cy, CORE_RADIUS, CORE);
    draw_disk(image, cx, cy, 1, HALO);
}

fn draw_ring(image: &mut RgbaImage, cx: i64, cy: i64, inner: i64, outer: i64, color: [u8; 4]) {
    let inner_sq = inner * inner;
    let outer_sq = outer * outer;
    for dy in -outer..=outer {
        for dx in -outer..=outer {
            let dist_sq = dx * dx + dy * dy;
            if dist_sq > inner_sq && dist_sq <= outer_sq {
                put(image, cx + dx, cy + dy, color);
            }
        }
    }
}

fn draw_disk(image: &mut RgbaImage, cx: i64, cy: i64, radius: i64, color: [u8; 4]) {
    let radius_sq = radius * radius;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= radius_sq {
                put(image, cx + dx, cy + dy, color);
            }
        }
    }
}

fn put(image: &mut RgbaImage, x: i64, y: i64, color: [u8; 4]) {
    let (width, height) = image.dimensions();
    if x >= 0 && y >= 0 && x < width as i64 && y < height as i64 {
        image.put_pixel(x as u32, y as u32, image::Rgba(color));
    }
}
