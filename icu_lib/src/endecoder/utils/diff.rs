use crate::midata::MiData;
use image::{Pixel, Rgba, RgbaImage};

const RED: Rgba<u8> = Rgba([0xFF, 0x00, 0x00, 0xFF]);

pub fn blend_color32(
    c1: &impl Pixel<Subpixel = u8>,
    c2: &impl Pixel<Subpixel = u8>,
    t: f32,
) -> impl Pixel<Subpixel = u8> {
    let t = t.clamp(0.0, 1.0);
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

pub fn diff_image(
    img1: &MiData,
    img2: &MiData,
    diff_blend: f32,
    diff_tolerance: f32,
    only_show_diff: bool,
) -> Option<(MiData, f32, f32)> {
    match (img1, img2) {
        (MiData::RGBA(img1), MiData::RGBA(img2)) => {
            if img1 != img2 && img1.width() == img2.width() && img1.height() == img2.height() {
                // Only diff same size
                let mut diff_data = Vec::with_capacity(img1.pixels().len());
                let blend = diff_blend;
                let tolerance = diff_tolerance; // Now in pixel diff units (0~1020)
                let mut min_diff = f32::MAX;
                let mut max_diff = f32::MIN;
                // First pass: find min/max diff (in absolute pixel diff)
                for (p1, p2) in img1.pixels().zip(img2.pixels()) {
                    let d = color_diff_f32(p1, p2) * 255.0 * 4.0;
                    if d < min_diff {
                        min_diff = d;
                    }
                    if d > max_diff {
                        max_diff = d;
                    }
                }
                // Second pass: apply tolerance (tolerance is absolute pixel diff)
                for (p1, p2) in img1.pixels().zip(img2.pixels()) {
                    let d = color_diff_f32(p1, p2) * 255.0 * 4.0;
                    if d < tolerance {
                        if only_show_diff {
                            diff_data.push(Rgba::from([0, 0, 0, 0]));
                        } else {
                            diff_data.push(*p1);
                        }
                    } else if only_show_diff {
                        diff_data.push(RED);
                    } else if blend <= 0.0 {
                        diff_data.push(*p1);
                    } else if blend >= 1.0 {
                        diff_data.push(*p2);
                    } else {
                        let blended = if blend < 0.5 {
                            let t = blend / 0.5;
                            blend_color32(p1, &RED, t)
                        } else {
                            let t = (blend - 0.5) / 0.5;
                            blend_color32(&RED, p2, t)
                        };
                        diff_data.push(blended.to_rgba());
                    }
                }

                let diff_img = RgbaImage::from_vec(
                    img1.width(),
                    img1.height(),
                    diff_data
                        .iter()
                        .flat_map(|x| x.channels())
                        .cloned()
                        .collect::<Vec<u8>>(),
                )?;

                Some((MiData::RGBA(diff_img), min_diff, max_diff))
            } else {
                None
            }
        }
        _ => None,
    }
}
