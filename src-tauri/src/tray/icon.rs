use tauri::image::Image;

// The BrandMark waveform as a macOS menu-bar template image: the shape lives in
// the alpha channel (RGB stays black) so macOS tints it for light/dark menu bars.
// Coordinates mirror the in-app `BrandMark` SVG — a 16-unit viewBox with five
// round-capped strokes (two dots are zero-length segments).
const SIZE: u32 = 36;
const VIEW_BOX: f32 = 16.0;
const STROKE_RADIUS: f32 = 0.75;

/// A stroke as `(x1, y1, x2, y2)` in viewBox coordinates; dots are zero-length.
type Segment = (f32, f32, f32, f32);

const SEGMENTS: [Segment; 5] = [
    (2.0, 8.0, 2.0, 8.0),
    (5.0, 5.5, 5.0, 10.5),
    (8.0, 3.5, 8.0, 12.5),
    (11.0, 5.5, 11.0, 10.5),
    (14.0, 8.0, 14.0, 8.0),
];

pub(super) fn template_icon() -> Image<'static> {
    let units_per_px = VIEW_BOX / SIZE as f32;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    for (index, pixel) in rgba.chunks_exact_mut(4).enumerate() {
        let px = (index as u32 % SIZE) as f32 + 0.5;
        let py = (index as u32 / SIZE) as f32 + 0.5;
        let dist = SEGMENTS
            .iter()
            .map(|&segment| distance_to_segment(px * units_per_px, py * units_per_px, segment))
            .fold(f32::INFINITY, f32::min);
        let coverage = ((STROKE_RADIUS - dist) / units_per_px + 0.5).clamp(0.0, 1.0);
        pixel[3] = (coverage * 255.0).round() as u8;
    }
    Image::new_owned(rgba, SIZE, SIZE)
}

fn distance_to_segment(px: f32, py: f32, (x1, y1, x2, y2): Segment) -> f32 {
    let (dx, dy) = (x2 - x1, y2 - y1);
    let len_sq = dx * dx + dy * dy;
    let t = if len_sq <= f32::EPSILON {
        0.0
    } else {
        (((px - x1) * dx + (py - y1) * dy) / len_sq).clamp(0.0, 1.0)
    };
    let (cx, cy) = (x1 + t * dx, y1 + t * dy);
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}
