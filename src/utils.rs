use super::image_shower::ImageItem;
use eframe::egui::Color32;
use icu_lib::midata::MiData;

pub fn diff_image(
    img1: &ImageItem,
    img2: &ImageItem,
    diff_blend: f32,
    diff_tolerance: f32,
    only_show_diff: bool,
) -> Option<(ImageItem, f32, f32)> {
    let (diff, diff_min, diff_max) = icu_lib::endecoder::utils::diff::diff_image(
        &MiData::from_rgba(
            img1.width,
            img1.height,
            img1.image_data
                .iter()
                .flat_map(|x| x.to_array())
                .collect::<Vec<u8>>(),
        )?,
        &MiData::from_rgba(
            img2.width,
            img2.height,
            img2.image_data
                .iter()
                .flat_map(|x| x.to_array())
                .collect::<Vec<u8>>(),
        )?,
        diff_blend,
        diff_tolerance,
        only_show_diff,
    )?;

    match diff {
        MiData::RGBA(rgba) => {
            let rgba = rgba.to_vec();
            Some((
                ImageItem {
                    path: "".to_string(),
                    width: img1.width,
                    height: img2.height,
                    image_data: rgba
                        .to_vec()
                        .chunks(4)
                        .map(|pixel| {
                            Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3])
                        })
                        .collect::<Vec<Color32>>(),
                },
                diff_min,
                diff_max,
            ))
        }
        _ => None,
    }
}
