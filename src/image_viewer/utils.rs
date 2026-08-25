use crate::converter::ImageFormatCategory;
use crate::image_viewer::model::{ConvertParams, Frame, FrameSource, ImageFormat, ImageItem};
use eframe::egui::{Color32, DroppedFile};
use icu_lib::EncoderParams;
use icu_lib::endecoder::{EnDecoder, ImageInfo};
use icu_lib::image::AnimationDecoder;
use icu_lib::midata::MiData;
use std::io::Cursor;
use std::path::Path;
use std::time::Duration;

pub fn process_images_with_format(
    files: &[DroppedFile],
    input_format: ImageFormatCategory,
) -> Vec<ImageItem> {
    files
        .iter()
        .filter_map(|file| decode_dropped_file(file, input_format))
        .collect()
}

fn decode_dropped_file(file: &DroppedFile, input_format: ImageFormatCategory) -> Option<ImageItem> {
    let file_path_info = if let Some(path) = &file.path {
        path.display().to_string()
    } else if !file.name.is_empty() {
        file.name.clone()
    } else {
        return None;
    };

    let data = match &file.bytes {
        Some(bytes) => bytes.to_vec(),
        None => std::fs::read(&file_path_info).ok()?,
    };

    if input_format != ImageFormatCategory::LVGL_V9
        && let Some(item) = decode_animation(&file_path_info, &data)
    {
        return Some(item);
    }

    let coder: &dyn EnDecoder = match input_format {
        ImageFormatCategory::Auto => icu_lib::endecoder::find_endecoder(&data)?,
        ImageFormatCategory::Common => &icu_lib::endecoder::common::AutoDetect {},
        ImageFormatCategory::LVGL_V9 => &icu_lib::endecoder::lvgl::LVGL {},
    };
    let mi_data = coder.decode(data.clone());
    image_item_from_midata(file_path_info, coder.info(&data), mi_data)
}

fn decode_animation(path: &str, data: &[u8]) -> Option<ImageItem> {
    let frames = if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        let decoder =
            icu_lib::image::codecs::gif::GifDecoder::new(Cursor::new(data.to_vec())).ok()?;
        decoder.into_frames().collect_frames().ok()?
    } else if data.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]) {
        let decoder =
            icu_lib::image::codecs::png::PngDecoder::new(Cursor::new(data.to_vec())).ok()?;
        if !decoder.is_apng().ok()? {
            return None;
        }
        decoder.apng().ok()?.into_frames().collect_frames().ok()?
    } else {
        return None;
    };

    if frames.len() <= 1 {
        return None;
    }

    let frames = frames
        .into_iter()
        .map(frame_from_image_frame)
        .collect::<Vec<_>>();
    let width = frames.first().map(|f| f.width).unwrap_or(0);
    let height = frames.first().map(|f| f.height).unwrap_or(0);
    let format = icu_lib::image::guess_format(data)
        .map(|f| f.to_mime_type().to_string())
        .unwrap_or_else(|_| "image".to_string());

    Some(ImageItem {
        path: path.to_string(),
        info: ImageInfo {
            width,
            height,
            data_size: data.len() as u32,
            format,
            other_info: serde_json::Value::Null,
        },
        width,
        height,
        frames: FrameSource::animated(frames),
        midata: None,
        expanded: false,
    })
}

fn image_item_from_midata(path: String, info: ImageInfo, mi_data: MiData) -> Option<ImageItem> {
    let midata_clone = mi_data.clone();
    match mi_data {
        MiData::RGBA(img_buffer) => Some(single_image_item(
            path,
            info,
            img_buffer,
            Some(midata_clone),
        )),
        MiData::GRAY(_) => None,
        MiData::PATH(scene_data) => {
            let (w, h) =
                icu_lib::endecoder::mirui::scene_render::scene_dimensions(&scene_data.scene)
                    .unwrap_or((256, 256));
            let img =
                icu_lib::endecoder::mirui::scene_render::render_scene(&scene_data.scene, w, h);
            Some(single_image_item(path, info, img, Some(midata_clone)))
        }
        MiData::FONT(font_data) => {
            let img = match font_data {
                icu_lib::midata::FontData::Mirx(f) => {
                    icu_lib::endecoder::mirui::font_render::render_font_atlas(&f)
                }
                icu_lib::midata::FontData::MirxBundle(fonts) => {
                    icu_lib::endecoder::mirui::font_render::render_font_atlas(fonts.first()?)
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
            Some(single_image_item(path, info, img, Some(midata_clone)))
        }
        MiData::INDEXED(indexed) => {
            let width = indexed.width;
            let height = indexed.height;
            let pixels = color32_from_rgba(indexed.rgba.chunks(4));
            Some(ImageItem {
                path,
                info,
                width,
                height,
                frames: FrameSource::single(pixels, width, height),
                midata: Some(midata_clone),
                expanded: false,
            })
        }
    }
}

pub fn single_image_item(
    path: String,
    info: ImageInfo,
    img: icu_lib::image::RgbaImage,
    midata: Option<MiData>,
) -> ImageItem {
    let width = img.width();
    let height = img.height();
    let pixels = color32_from_rgba(img.chunks(4));
    ImageItem {
        path,
        info,
        width,
        height,
        frames: FrameSource::single(pixels, width, height),
        midata,
        expanded: false,
    }
}

pub fn color32_from_rgba<'a>(chunks: impl Iterator<Item = &'a [u8]>) -> Vec<Color32> {
    chunks
        .map(|pixel| Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]))
        .collect()
}

fn frame_from_image_frame(frame: icu_lib::image::Frame) -> Frame {
    let delay = delay_to_duration(frame.delay());
    let left = frame.left();
    let top = frame.top();
    let buffer = frame.into_buffer();
    let width = buffer.width();
    let height = buffer.height();
    let pixels = color32_from_rgba(buffer.chunks(4));
    Frame {
        pixels,
        width,
        height,
        left,
        top,
        delay,
    }
}

fn delay_to_duration(delay: icu_lib::image::Delay) -> Duration {
    let (numer, denom) = delay.numer_denom_ms();
    if denom == 0 {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(numer as f64 / denom as f64 / 1000.0)
    }
}

pub fn get_system_locale() -> String {
    let locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
    locale.replace('_', "-")
}

pub fn convert_image(
    image_item: &ImageItem,
    params: &ConvertParams,
) -> Result<(Vec<u8>, String), String> {
    let output_format = params.output_format;
    let midata = if output_format == ImageFormat::LVGL {
        match &image_item.midata {
            Some(MiData::INDEXED(indexed)) => MiData::INDEXED(indexed.clone()),
            _ => {
                let (pixels, width, height) = image_item.current_pixels();
                MiData::from_rgba(
                    width,
                    height,
                    pixels.iter().flat_map(|pixel| pixel.to_array()).collect(),
                )
                .ok_or("Failed to create MiData")?
            }
        }
    } else {
        let (pixels, width, height) = image_item.current_pixels();
        MiData::from_rgba(
            width,
            height,
            pixels.iter().flat_map(|pixel| pixel.to_array()).collect(),
        )
        .ok_or("Failed to create MiData")?
    };

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
    if data.is_empty() {
        return Err(format!(
            "{:?} does not support this image representation",
            output_format
        ));
    }
    let ext = output_format.get_file_extension().to_string();

    Ok((data, ext))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_images(items: &[ImageItem], params: &ConvertParams) {
    let folder = rfd::FileDialog::new().pick_folder();
    if let Some(folder) = folder {
        for item in items {
            match convert_image(item, params) {
                Ok((data, ext)) => {
                    let file_name = Path::new(&item.path)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let new_path = folder.join(format!("{}.{}", file_name, ext));
                    if let Err(error) = std::fs::write(&new_path, data) {
                        log::error!("Failed to save {}: {error}", new_path.display());
                    }
                }
                Err(error) => log::error!("Failed to convert {}: {error}", item.path),
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn save_images(items: &[ImageItem], params: &ConvertParams) {
    use eframe::wasm_bindgen::JsCast;

    for item in items {
        match convert_image(item, params) {
            Ok((data, ext)) => {
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
                let blob =
                    web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &blob_options)
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
            Err(error) => log::error!("Failed to convert {}: {error}", item.path),
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn pick_files_web(
    pending: std::rc::Rc<std::cell::RefCell<Vec<DroppedFile>>>,
    ctx: eframe::egui::Context,
) {
    use eframe::wasm_bindgen::{JsCast, JsValue, closure::Closure};
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
