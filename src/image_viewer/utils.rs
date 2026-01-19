use crate::image_viewer::model::{ConvertParams, ImageItem};
use eframe::egui::{Color32, DroppedFile};
use icu_lib::midata::MiData;
use icu_lib::EncoderParams;
use std::path::Path;

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

pub fn get_system_locale() -> String {
    let locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
    locale.replace('_', "-")
}

pub fn convert_image(
    image_item: &ImageItem,
    params: &ConvertParams,
) -> Result<(Vec<u8>, String), String> {
    let midata = MiData::from_rgba(
        image_item.width,
        image_item.height,
        image_item.image_data.iter().flat_map(|x| x.to_array()).collect::<Vec<u8>>(),
    ).ok_or("Failed to create MiData")?;

    let encoder_params = EncoderParams {
        lvgl_version: params.lvgl_version.into(),
        color_format: params.color_format.into(),
        stride_align: params.stride_align as u32,
        dither: if params.dither { Some(1) } else { None },
        compress: params.compression.into(),
        ..Default::default()
    };

    let output_format = params.output_format;

    let encoder = output_format.get_endecoder();
    let data = encoder.encode(&midata, encoder_params);
    let ext = output_format.get_file_extension().to_string();

    Ok((data, ext))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_images(items: &[ImageItem], params: &ConvertParams) {
    let folder = rfd::FileDialog::new().pick_folder();
    if let Some(folder) = folder {
        for item in items {
            if let Ok((data, ext)) = convert_image(item, params) {
                let file_name = Path::new(&item.path)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy();
                let new_path = folder.join(format!("{}.{}", file_name, ext));
                if let Err(e) = std::fs::write(&new_path, data) {
                    log::error!("Failed to save file: {}", e);
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn save_images(items: &[ImageItem], params: &ConvertParams) {
    use eframe::wasm_bindgen::JsCast;

    for item in items {
        if let Ok((data, ext)) = convert_image(item, params) {
            let file_name = Path::new(&item.path)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            let file_name = format!("{}.{}", file_name, ext);

            let window = web_sys::window().expect("window not found");
            let document = window.document().expect("document not found");
            let body = document.body().expect("body not found");

            let uint8_array = unsafe { js_sys::Uint8Array::view(&data) };
            let array = js_sys::Array::new();
            array.push(&uint8_array);
            let blob_options = web_sys::BlobPropertyBag::new();
            blob_options.set_type("application/octet-stream");
            let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(
                &array,
                &blob_options,
            )
            .expect("failed to create blob");

            let url = web_sys::Url::create_object_url_with_blob(&blob)
                .expect("failed to create object url");

            let a = document
                .create_element("a")
                .expect("failed to create anchor")
                .dyn_into::<web_sys::HtmlAnchorElement>()
                .expect("failed to cast to anchor");

            a.set_href(&url);
            a.set_download(&file_name);
            a.style().set_property("display", "none").ok();

            body.append_child(&a).ok();
            a.click();
            body.remove_child(&a).ok();
            web_sys::Url::revoke_object_url(&url).ok();
        }
    }
}
