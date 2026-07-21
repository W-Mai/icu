use crate::image_viewer::model::ImageItem;
use eframe::egui::Color32;
use icu_lib::endecoder::ImageInfo;
use icu_lib::endecoder::utils::diff::{ImageDiffResult, blend_color32, diff_image as compute_diff};
use icu_lib::image::{Pixel, RgbaImage};
use icu_lib::midata::MiData;
use icu_lib::postprocess::{DiffOverlay, OverlayStack};

pub fn diff_image(
    img1: &ImageItem,
    img2: &ImageItem,
    diff_blend: f32,
    diff_tolerance: f32,
    only_show_diff: bool,
) -> Option<(ImageItem, ImageDiffResult)> {
    let (img1_pixels, img1_width, img1_height) = img1.current_pixels();
    let (img2_pixels, img2_width, img2_height) = img2.current_pixels();
    let m1 = MiData::from_rgba(
        img1_width,
        img1_height,
        img1_pixels
            .iter()
            .flat_map(|x| x.to_array())
            .collect::<Vec<u8>>(),
    )?;
    let m2 = MiData::from_rgba(
        img2_width,
        img2_height,
        img2_pixels
            .iter()
            .flat_map(|x| x.to_array())
            .collect::<Vec<u8>>(),
    )?;

    let diff_result = compute_diff(&m1, &m2)?;
    let (w, h) = diff_result.size();

    let base = if only_show_diff {
        RgbaImage::new(w, h)
    } else {
        let m1_img = match &m1 {
            MiData::RGBA(i) => i,
            _ => return None,
        };
        let m2_img = match &m2 {
            MiData::RGBA(i) => i,
            _ => return None,
        };
        let mut blended = RgbaImage::new(w, h);
        for ((p1, p2), out) in m1_img
            .pixels()
            .zip(m2_img.pixels())
            .zip(blended.pixels_mut())
        {
            *out = blend_color32(p1, p2, diff_blend).to_rgba();
        }
        blended
    };

    let mut stack = OverlayStack::new(base);
    stack.push(Box::new(DiffOverlay::new(
        diff_result.clone(),
        diff_tolerance,
        diff_blend,
    )));
    let composited = stack.composite().clone();

    let image_data: Vec<Color32> = composited
        .chunks(4)
        .map(|pixel| Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]))
        .collect();

    Some((
        ImageItem {
            path: "".to_string(),
            info: ImageInfo {
                width: w,
                height: h,
                data_size: 0,
                format: "diff".to_string(),
                other_info: serde_json::Value::Null,
            },
            width: w,
            height: h,
            frames: crate::image_viewer::model::FrameSource::single(image_data, w, h),
            midata: None,
            expanded: false,
        },
        diff_result,
    ))
}
