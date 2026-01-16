use crate::image_viewer::model::ImageItem;
use eframe::egui::{Color32, DroppedFile};
use icu_lib::midata::MiData;

pub fn process_images(files: &[DroppedFile]) -> Vec<ImageItem> {
    files
        .iter()
        .map_while(|file| {
            let file_path_info = if let Some(path) = &file.path {
                path.display().to_string()
            } else if !file.name.is_empty() {
                file.name.clone()
            } else {
                return None;
            };

            let (mi_data, image_info) = match &file.bytes {
                Some(bytes) => {
                    if let Some(coder) = icu_lib::endecoder::find_endecoder(bytes) {
                        (coder.decode(bytes.to_vec()), coder.info(bytes))
                    } else {
                        return None;
                    }
                }
                None => {
                    let data = std::fs::read(&file_path_info);
                    match data {
                        Ok(data) => {
                            if let Some(coder) = icu_lib::endecoder::find_endecoder(&data) {
                                (coder.decode(data.clone()), coder.info(&data))
                            } else {
                                return None;
                            }
                        }
                        _ => return None,
                    }
                }
            };

            match mi_data {
                MiData::RGBA(img_buffer) => {
                    let width = img_buffer.width();
                    let height = img_buffer.height();
                    let image_data = img_buffer
                        .chunks(4)
                        .map(|pixel| {
                            Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3])
                        })
                        .collect::<Vec<Color32>>();

                    Some(ImageItem {
                        path: file_path_info,
                        info: image_info,
                        width,
                        height,
                        image_data,
                    })
                }
                MiData::GRAY(_) => None,
                MiData::PATH => None,
            }
        })
        .collect()
}
