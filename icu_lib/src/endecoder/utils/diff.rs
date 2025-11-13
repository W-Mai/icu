use crate::midata::MiData;
use image::{Pixel, Rgba, RgbaImage};

const RED: Rgba<u8> = Rgba([0xFF, 0x00, 0x00, 0xFF]);

pub struct ImageDiffPixel {
    pub pos: (u32, u32),
    pub color_lhs: Rgba<u8>,
    pub color_rhs: Rgba<u8>,
    pub diff: [f32; 4],
}

impl ImageDiffPixel {
    pub fn new(pos: (u32, u32), color_lhs: Rgba<u8>, color_rhs: Rgba<u8>) -> Self {
        let mut s = Self {
            pos,
            color_lhs,
            color_rhs,
            diff: [0.; 4],
        };

        s.diff = s.calc_diff();
        s
    }

    fn calc_diff(&self) -> [f32; 4] {
        [
            self.color_lhs[0] as f32 - self.color_rhs[0] as f32,
            self.color_lhs[1] as f32 - self.color_rhs[1] as f32,
            self.color_lhs[2] as f32 - self.color_rhs[2] as f32,
            self.color_lhs[3] as f32 - self.color_rhs[3] as f32,
        ]
    }
}

pub struct ImageDiffResult {
    size: (u32, u32),
    diffs: Vec<ImageDiffPixel>,
    min_diff: f32,
    max_diff: f32,
}

impl Default for ImageDiffResult {
    fn default() -> Self {
        Self::new((0, 0), Vec::new(), f32::MIN, f32::MAX)
    }
}

impl ImageDiffResult {
    pub fn new(size: (u32, u32), diffs: Vec<ImageDiffPixel>, min_diff: f32, max_diff: f32) -> Self {
        Self {
            size,
            diffs,
            min_diff,
            max_diff,
        }
    }
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn diffs(&self) -> &Vec<ImageDiffPixel> {
        &self.diffs
    }

    pub fn diff_filter(&self, tolerance: f32) -> impl Iterator<Item = &ImageDiffPixel> + use<'_> {
        self.diffs
            .iter()
            .filter(move |x| x.diff.iter().any(|&d| d.abs() >= tolerance))
    }

    pub fn max_diff(&self) -> f32 {
        self.max_diff
    }

    pub fn min_diff(&self) -> f32 {
        self.min_diff
    }

    pub fn render_diff_mask(&self, tolerance: f32, color: Rgba<u8>) -> RgbaImage {
        let (width, height) = self.size();
        let mut img = RgbaImage::new(width, height);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 0]);
        }
        for diff_pixel in self.diff_filter(tolerance) {
            let (x, y) = diff_pixel.pos;
            img.put_pixel(x, y, color);
        }
        img
    }
}

pub fn blend_color32(
    c1: &impl Pixel<Subpixel = u8>,
    c2: &impl Pixel<Subpixel = u8>,
    t: f32,
) -> impl Pixel<Subpixel = u8> {
    let t = t.clamp(0., 1.0);
    let c1 = c1.to_rgba();
    let c2 = c2.to_rgba();
    Rgba::from([
        (c1[0] as f32 * (1.0 - t) + c2[0] as f32 * t) as u8,
        (c1[1] as f32 * (1.0 - t) + c2[1] as f32 * t) as u8,
        (c1[2] as f32 * (1.0 - t) + c2[2] as f32 * t) as u8,
        (c1[3] as f32 * (1.0 - t) + c2[3] as f32 * t) as u8,
    ])
}

// fn color_diff_f32(c1: Color32, c2: Color32) -> f32 {
//     // Simple Euclidean distance in RGBA space, normalized to [0,1]
//     let dr = c1.r() as f32 - c2.r() as f32;
//     let dg = c1.g() as f32 - c2.g() as f32;
//     let db = c1.b() as f32 - c2.b() as f32;
//     let da = c1.a() as f32 - c2.a() as f32;
//
//     (dr * dr + dg * dg + db * db + da * da).sqrt() / (4.0 * 255.0)
// }

pub fn color_diff_f32(c1: &impl Pixel<Subpixel = u8>, c2: &impl Pixel<Subpixel = u8>) -> f32 {
    let a32 = c1.to_rgba();
    let b32 = c2.to_rgba();

    let dr = a32[0].abs_diff(b32[0]);
    let dg = a32[1].abs_diff(b32[1]);
    let db = a32[2].abs_diff(b32[2]);
    let da = a32[3].abs_diff(b32[3]);

    dr.max(dg).min(db).max(da) as f32 / (4.0 * 255.0)
}

pub fn diff_image(img1: &MiData, img2: &MiData) -> Option<ImageDiffResult> {
    match (img1, img2) {
        (MiData::RGBA(img1), MiData::RGBA(img2)) => {
            if img1 != img2 && img1.width() == img2.width() && img1.height() == img2.height() {
                // Only diff same size
                let mut diff_data = Vec::with_capacity(img1.pixels().len());
                let mut min_diff = f32::MAX;
                let mut max_diff = f32::MIN;
                // First pass: find min/max diff (in absolute pixel diff)
                for ((p1_i, p1), p2) in img1.pixels().enumerate().zip(img2.pixels()) {
                    let diff_pixel = ImageDiffPixel::new(
                        (p1_i as u32 % img1.width(), p1_i as u32 / img1.width()),
                        *p1,
                        *p2,
                    );
                    let diff = diff_pixel.diff;
                    let d = diff
                        .into_iter()
                        .map(f32::abs)
                        .reduce(f32::max)
                        .unwrap_or(0.);
                    if d < min_diff {
                        min_diff = d;
                    }
                    if d > max_diff {
                        max_diff = d;
                    }
                    diff_data.push(diff_pixel);
                }
                Some(ImageDiffResult::new(
                    (img1.width(), img1.height()),
                    diff_data,
                    min_diff,
                    max_diff,
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn blend_diff_image(
    img1: &MiData,
    img2: &MiData,
    diff_blend: f32,
    diff_tolerance: f32,
    only_show_diff: bool,
) -> Option<(MiData, ImageDiffResult)> {
    let diff_result = diff_image(img1, img2)?;
    let min_diff = diff_result.min_diff() as f32;
    let max_diff = diff_result.max_diff() as f32;
    let mut diff_mask = diff_result.render_diff_mask(diff_tolerance, RED);

    if only_show_diff {
        return Some((MiData::RGBA(diff_mask), diff_result));
    }

    match (img1, img2) {
        (MiData::RGBA(img1), MiData::RGBA(img2)) => {
            for ((p1, p2), diff_pixel) in
                img1.pixels().zip(img2.pixels()).zip(diff_mask.pixels_mut())
            {
                let blended = blend_color32(p1, p2, diff_blend);
                *diff_pixel = blended.to_rgba();
            }

            for pixel in diff_result.diff_filter(diff_tolerance) {
                let (x, y) = pixel.pos;
                let p = diff_mask.get_pixel(x, y);

                let t = (diff_blend - 0.5).abs() / 0.5;

                let blended = blend_color32(&RED, p, t).to_rgba();
                diff_mask.put_pixel(x, y, blended);
            }

            Some((MiData::RGBA(diff_mask), diff_result))
        }
        _ => None,
    }
}
