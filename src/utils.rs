use super::image_shower::ImageItem;
use eframe::egui::Color32;

pub fn blend_color32(c1: Color32, c2: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (c1.r() as f32 * (1.0 - t) + c2.r() as f32 * t) as u8,
        (c1.g() as f32 * (1.0 - t) + c2.g() as f32 * t) as u8,
        (c1.b() as f32 * (1.0 - t) + c2.b() as f32 * t) as u8,
        (c1.a() as f32 * (1.0 - t) + c2.a() as f32 * t) as u8,
    )
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

pub fn color_diff_f32(c1: Color32, c2: Color32) -> f32 {
    // Simple Euclidean distance in RGBA space, normalized to [0,1]
    let dr = c1.r().abs_diff(c2.r());
    let dg = c1.g().abs_diff(c2.g());
    let db = c1.b().abs_diff(c2.b());
    let da = c1.a().abs_diff(c2.a());

    dr.max(dg).min(db).max(da) as f32 / (4.0 * 255.0)
}

pub fn diff_image(
    img1: &ImageItem,
    img2: &ImageItem,
    diff_blend: f32,
    diff_tolerance: f32,
    only_show_diff: bool,
) -> Option<(ImageItem, f32, f32)> {
    if img1.image_data != img2.image_data && img1.width == img2.width && img1.height == img2.height
    {
        // Only diff same size
        let mut diff_data = Vec::with_capacity(img1.image_data.len());
        let blend = diff_blend;
        let tolerance = diff_tolerance; // Now in pixel diff units (0~1020)
        let mut min_diff = f32::MAX;
        let mut max_diff = f32::MIN;
        // First pass: find min/max diff (in absolute pixel diff)
        for (p1, p2) in img1.image_data.iter().zip(&img2.image_data) {
            let d = color_diff_f32(*p1, *p2) * 255.0 * 4.0;
            if d < min_diff {
                min_diff = d;
            }
            if d > max_diff {
                max_diff = d;
            }
        }
        // Second pass: apply tolerance (tolerance is absolute pixel diff)
        for (p1, p2) in img1.image_data.iter().zip(&img2.image_data) {
            let d = color_diff_f32(*p1, *p2) * 255.0 * 4.0;
            if d < tolerance {
                if only_show_diff {
                    diff_data.push(Color32::TRANSPARENT);
                } else {
                    diff_data.push(*p1);
                }
            } else if only_show_diff {
                diff_data.push(Color32::RED);
            } else if blend <= 0.0 {
                diff_data.push(*p1);
            } else if blend >= 1.0 {
                diff_data.push(*p2);
            } else {
                let blended = if blend < 0.5 {
                    let t = blend / 0.5;
                    blend_color32(*p1, Color32::RED, t)
                } else {
                    let t = (blend - 0.5) / 0.5;
                    blend_color32(Color32::RED, *p2, t)
                };
                diff_data.push(blended);
            }
        }
        Some((
            ImageItem {
                path: format!("diff: {} <-> {}", img1.path, img2.path),
                width: img1.width,
                height: img1.height,
                image_data: diff_data,
            },
            min_diff,
            max_diff,
        ))
    } else {
        None
    }
}
