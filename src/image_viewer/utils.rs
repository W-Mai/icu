use crate::image_viewer::model::{ConvertParams, ImageItem};
use eframe::egui::{Color32, DroppedFile};
use icu_lib::EncoderParams;
use icu_lib::midata::MiData;
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

            let midata_clone = mi_data.clone();
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
                        midata: Some(midata_clone),
                    })
                }
                MiData::GRAY(_) => None,
                MiData::PATH(scene_data) => {
                    let (w, h) = icu_lib::endecoder::mirui::scene_render::scene_dimensions(
                        &scene_data.scene,
                    )
                    .unwrap_or((256, 256));
                    let img = icu_lib::endecoder::mirui::scene_render::render_scene(
                        &scene_data.scene,
                        w,
                        h,
                    );
                    let width = img.width();
                    let height = img.height();
                    let image_data = img
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
                        midata: Some(midata_clone),
                    })
                }
                MiData::FONT(font_data) => {
                    let img = match font_data {
                        icu_lib::midata::FontData::Mirx(f) => {
                            icu_lib::endecoder::mirui::font_render::render_font_atlas(&f)
                        }
                        icu_lib::midata::FontData::MirxBundle(fonts) => {
                            if let Some(f) = fonts.first() {
                                icu_lib::endecoder::mirui::font_render::render_font_atlas(f)
                            } else {
                                return None;
                            }
                        }
                        icu_lib::midata::FontData::FreeType(f) => {
                            icu_lib::endecoder::mirui::font_render::render_freetype_glyphs(
                                &f,
                                icu_lib::mirx::Color {
                                    r: 200,
                                    g: 200,
                                    b: 200,
                                    a: 255,
                                },
                            )
                        }
                    };
                    let width = img.width();
                    let height = img.height();
                    let image_data = img
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
                        midata: Some(midata_clone),
                    })
                }
                MiData::INDEXED(indexed) => {
                    let width = indexed.width;
                    let height = indexed.height;
                    let image_data = indexed
                        .rgba
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
                        midata: Some(midata_clone),
                    })
                }
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
        image_item
            .image_data
            .iter()
            .flat_map(|x| x.to_array())
            .collect::<Vec<u8>>(),
    )
    .ok_or("Failed to create MiData")?;

    let encoder_params = EncoderParams {
        lvgl_version: params.lvgl_version.into(),
        color_format: params.color_format.into(),
        stride_align: params.stride_align as u32,
        dither: if params.dither {
            Some(params.dither_level)
        } else {
            None
        },
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
            let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &blob_options)
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

#[cfg(target_arch = "wasm32")]
pub fn pick_files_web(
    pending: std::rc::Rc<std::cell::RefCell<Vec<DroppedFile>>>,
    ctx: eframe::egui::Context,
) {
    use eframe::wasm_bindgen::{closure::Closure, JsCast, JsValue};
    use std::rc::Rc;

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    let input = match document.create_element("input") {
        Ok(el) => match el.dyn_into::<web_sys::HtmlInputElement>() {
            Ok(i) => i,
            Err(_) => return,
        },
        Err(_) => return,
    };
    input.set_type("file");
    input.set_multiple(true);

    let pending = Rc::new(pending);
    let ctx = Rc::new(ctx);
    let input_for_cb = input.clone();

    let on_change = Closure::<dyn FnMut(_)>::new(move |_evt: web_sys::Event| {
        let files = match input_for_cb.files() {
            Some(f) => f,
            None => return,
        };
        let pending = pending.clone();
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut out = Vec::new();
            for i in 0..files.length() {
                if let Some(file) = files.get(i) {
                    let name = file.name();
                    let buf = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
                        .await
                        .ok()
                        .and_then(|b| js_sys::Uint8Array::new(&b).to_vec().try_into().ok())
                        .map(|v: std::borrow::Cow<'_, [u8]>| v.into_owned())
                        .unwrap_or_default();
                    out.push(DroppedFile {
                        name: name.into(),
                        bytes: Some(std::sync::Arc::from(buf)),
                        ..Default::default()
                    });
                }
            }
            if !out.is_empty() {
                pending.borrow_mut().extend(out);
                ctx.request_repaint();
            }
        });
    });

    let _ = input.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
    on_change.forget();

    if let Some(body) = document.body() {
        let _ = body.append_child(&input);
    }
    input.style().set_property("display", "none").ok();
    input.click();
    let _ = JsValue::from(input);
}
