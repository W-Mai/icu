use crate::converter::ImageFormatCategory;
use crate::image_viewer::model::{
    ConvertParams, Frame, FrameSource, ImageFormat, ImageItem, SelectionTarget, ViewerState,
    WorkspaceId,
};
use eframe::egui::{Color32, DroppedFile};
use icu_lib::EncoderParams;
use icu_lib::endecoder::{EnDecoder, ImageInfo};
use icu_lib::image::AnimationDecoder;
use icu_lib::midata::MiData;
use image::codecs::gif::{GifEncoder, Repeat};
use image::codecs::webp::WebPDecoder;
use image::{Delay, Frame as EncodedFrame, RgbaImage};
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

fn is_webp(data: &[u8]) -> bool {
    data.get(..4) == Some(b"RIFF") && data.get(8..12) == Some(b"WEBP")
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
    } else if is_webp(data) {
        let decoder = WebPDecoder::new(Cursor::new(data)).ok()?;
        if !decoder.has_animation() {
            return None;
        }
        decoder.into_frames().collect_frames().ok()?
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GifRepeat {
    Infinite,
    Finite(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GifExportOptions {
    pub interval: Duration,
    pub repeat: GifRepeat,
}

impl Default for GifExportOptions {
    fn default() -> Self {
        Self {
            interval: Duration::from_millis(100),
            repeat: GifRepeat::Infinite,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportTarget {
    Entry(WorkspaceId),
    Frame {
        collection: WorkspaceId,
        index: usize,
    },
}

#[derive(Clone)]
pub struct ExportPlan {
    pub label: String,
    pub items: Vec<ImageItem>,
}

pub fn export_plan(state: &ViewerState, target: ExportTarget) -> Option<ExportPlan> {
    match target {
        ExportTarget::Frame { collection, index } => {
            if let Some((_, _, item)) = state
                .group_members(collection)
                .and_then(|members| members.into_iter().nth(index))
            {
                return Some(ExportPlan {
                    label: state.group_label(collection)?.to_string(),
                    items: vec![item],
                });
            }
            let image = state.item(collection)?.as_image()?.clone();
            let FrameSource::Animated { frames, .. } = image.frames else {
                return None;
            };
            let frame = frames.get(index)?.clone();
            Some(ExportPlan {
                label: format!("{}-{}", image.path, index + 1),
                items: vec![ImageItem {
                    path: format!("{}-{}", image.path, index + 1),
                    info: image.info,
                    width: frame.width,
                    height: frame.height,
                    frames: FrameSource::single(frame.pixels, frame.width, frame.height),
                    midata: None,
                    expanded: false,
                }],
            })
        }
        ExportTarget::Entry(id) => {
            if let Some(members) = state.group_members(id) {
                return Some(ExportPlan {
                    label: state.group_label(id)?.to_string(),
                    items: members.into_iter().map(|(_, _, item)| item).collect(),
                });
            }
            let item = state.item(id)?.as_image()?.clone();
            Some(ExportPlan {
                label: Path::new(&item.path)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                items: vec![item],
            })
        }
    }
}

pub fn export_target_from_selection(state: &ViewerState) -> Option<ExportTarget> {
    match state.primary_target? {
        SelectionTarget::Entry(id) => Some(ExportTarget::Entry(id)),
        SelectionTarget::Frame { collection, index } => {
            Some(ExportTarget::Frame { collection, index })
        }
    }
}

pub fn save_export_plan(plan: &ExportPlan, params: &ConvertParams) {
    let has_animation =
        plan.items.len() > 1 || plan.items.iter().any(|item| item.frame_count() > 1);
    if has_animation
        && matches!(
            params.output_format,
            ImageFormat::GIF | ImageFormat::APNG | ImageFormat::WEBP
        )
    {
        let options = GifExportOptions {
            interval: Duration::from_millis(params.gif_interval_ms.max(1) as u64),
            repeat: params
                .gif_repeat
                .map_or(GifRepeat::Infinite, GifRepeat::Finite),
        };
        let encoded = match params.output_format {
            ImageFormat::GIF => encode_gif_frames(&plan.items, options).map(|data| ("gif", data)),
            ImageFormat::APNG => {
                encode_apng_frames(&plan.items, options).map(|data| ("apng", data))
            }
            ImageFormat::WEBP => {
                encode_webp_frames(&plan.items, options).map(|data| ("webp", data))
            }
            _ => unreachable!("animation export format was checked above"),
        };
        match encoded {
            Ok((extension, data)) => save_export_bytes(&plan.label, extension, data),
            Err(error) => log::error!("Failed to encode animation {}: {error}", plan.label),
        }
    } else {
        save_images(&plan.items, params);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn save_export_bytes(label: &str, extension: &str, data: Vec<u8>) {
    if let Some(path) = rfd::FileDialog::new()
        .set_file_name(format!("{label}.{extension}"))
        .save_file()
    {
        if let Err(error) = std::fs::write(path, data) {
            log::error!("Failed to save animation: {error}");
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn save_export_bytes(label: &str, extension: &str, data: Vec<u8>) {
    use eframe::wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(body) = document.body() else { return };
    let array = js_sys::Array::new();
    array.push(&js_sys::Uint8Array::from(data.as_slice()));
    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence(&array) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };
    let Ok(anchor) = document.create_element("a") else {
        return;
    };
    let Ok(anchor) = anchor.dyn_into::<web_sys::HtmlAnchorElement>() else {
        return;
    };
    anchor.set_href(&url);
    anchor.set_download(&format!("{label}.{extension}"));
    let _ = body.append_child(&anchor);
    anchor.click();
    let _ = body.remove_child(&anchor);
    let _ = web_sys::Url::revoke_object_url(&url);
}

pub fn encode_gif_frames(
    items: &[ImageItem],
    options: GifExportOptions,
) -> Result<Vec<u8>, String> {
    let mut frames = Vec::new();
    for item in items {
        match &item.frames {
            FrameSource::Single {
                pixels,
                width,
                height,
            } => {
                frames.push(encoded_frame(pixels, *width, *height, options.interval)?);
            }
            FrameSource::Animated {
                frames: item_frames,
                ..
            } => {
                for frame in item_frames {
                    frames.push(encoded_frame(
                        &frame.pixels,
                        frame.width,
                        frame.height,
                        if frame.delay.is_zero() {
                            options.interval
                        } else {
                            frame.delay
                        },
                    )?);
                }
            }
        }
    }
    if frames.is_empty() {
        return Err("Cannot encode an empty GIF".to_string());
    }

    let mut output = Cursor::new(Vec::new());
    let mut encoder = GifEncoder::new(&mut output);
    encoder
        .set_repeat(match options.repeat {
            GifRepeat::Infinite => Repeat::Infinite,
            GifRepeat::Finite(count) => Repeat::Finite(count),
        })
        .map_err(|error| error.to_string())?;
    encoder
        .encode_frames(frames)
        .map_err(|error| error.to_string())?;
    drop(encoder);
    Ok(output.into_inner())
}

fn encode_apng_frames(items: &[ImageItem], options: GifExportOptions) -> Result<Vec<u8>, String> {
    let frames = animation_frames(items);
    let first = frames.first().ok_or("Cannot encode an empty APNG")?;
    let mut output = Cursor::new(Vec::new());
    let mut encoder = png::Encoder::new(&mut output, first.width, first.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .set_animated(
            frames.len() as u32,
            match options.repeat {
                GifRepeat::Infinite => 0,
                GifRepeat::Finite(count) => count as u32,
            },
        )
        .map_err(|error| error.to_string())?;
    let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
    for frame in frames {
        let delay = if frame.delay.is_zero() {
            options.interval
        } else {
            frame.delay
        };
        let millis = delay.as_millis().clamp(1, u16::MAX as u128) as u16;
        writer
            .set_frame_delay(millis, 1000)
            .map_err(|error| error.to_string())?;
        let pixels = frame
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_array())
            .collect::<Vec<_>>();
        writer
            .write_image_data(&pixels)
            .map_err(|error| error.to_string())?;
    }
    drop(writer);
    Ok(output.into_inner())
}

pub fn encode_webp_frames(
    items: &[ImageItem],
    options: GifExportOptions,
) -> Result<Vec<u8>, String> {
    let loop_count = match options.repeat {
        GifRepeat::Infinite => 0,
        GifRepeat::Finite(count) => count,
    };
    super::webp_animation::encode(&animation_frames(items), options.interval, loop_count)
}

fn animation_frames(items: &[ImageItem]) -> Vec<Frame> {
    items
        .iter()
        .flat_map(|item| match &item.frames {
            FrameSource::Single {
                pixels,
                width,
                height,
            } => vec![Frame {
                pixels: pixels.clone(),
                width: *width,
                height: *height,
                left: 0,
                top: 0,
                delay: Duration::ZERO,
            }],
            FrameSource::Animated { frames, .. } => frames.clone(),
        })
        .collect()
}

fn encoded_frame(
    pixels: &[Color32],
    width: u32,
    height: u32,
    delay: Duration,
) -> Result<EncodedFrame, String> {
    let bytes = pixels
        .iter()
        .flat_map(|pixel| pixel.to_array())
        .collect::<Vec<_>>();
    let image = RgbaImage::from_raw(width, height, bytes)
        .ok_or_else(|| format!("Invalid RGBA frame dimensions: {width}x{height}"))?;
    Ok(EncodedFrame::from_parts(
        image,
        0,
        0,
        Delay::from_saturating_duration(delay),
    ))
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
    if output_format == ImageFormat::LVGL && !params.color_format.supports_lvgl() {
        return Err(format!(
            "{:?} is not supported by LVGL output",
            params.color_format
        ));
    }
    if output_format == ImageFormat::LVGL
        && params.compression == crate::image_viewer::model::LvglCompression::LZ4
        && params.lvgl_version != crate::image_viewer::model::LvglVersion::V9
    {
        return Err("LVGL LZ4 compression requires LVGL v9".to_string());
    }
    let preserve_indexed = output_format == ImageFormat::LVGL
        || (output_format == ImageFormat::PNG
            && params.png_color_mode == crate::image_viewer::model::PngColorMode::Preserve);
    let midata = if preserve_indexed {
        match &image_item.midata {
            Some(MiData::INDEXED(indexed)) => MiData::INDEXED(indexed.clone()),
            _ if output_format == ImageFormat::PNG => {
                return Err("PNG preserve mode requires indexed image data".to_string());
            }
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
        png_color_mode: match params.png_color_mode {
            crate::image_viewer::model::PngColorMode::Rgba => icu_lib::PngColorMode::Rgba,
            crate::image_viewer::model::PngColorMode::Rgb => icu_lib::PngColorMode::Rgb,
            crate::image_viewer::model::PngColorMode::Preserve => icu_lib::PngColorMode::Preserve,
            crate::image_viewer::model::PngColorMode::Indexed1 => icu_lib::PngColorMode::Indexed(1),
            crate::image_viewer::model::PngColorMode::Indexed2 => icu_lib::PngColorMode::Indexed(2),
            crate::image_viewer::model::PngColorMode::Indexed4 => icu_lib::PngColorMode::Indexed(4),
            crate::image_viewer::model::PngColorMode::Indexed8 => icu_lib::PngColorMode::Indexed(8),
        },
        png_compression: match params.png_compression {
            crate::image_viewer::model::PngCompression::Fast => icu_lib::PngCompression::Fast,
            crate::image_viewer::model::PngCompression::Balanced => {
                icu_lib::PngCompression::Balanced
            }
            crate::image_viewer::model::PngCompression::Best => icu_lib::PngCompression::Best,
        },
        jpeg_quality: params.jpeg_quality,
        jpeg_background: params.jpeg_background,
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::AnimationDecoder;

    fn animation_item() -> ImageItem {
        ImageItem {
            path: "animation".to_string(),
            info: ImageInfo {
                width: 2,
                height: 1,
                data_size: 0,
                format: "animation".to_string(),
                other_info: serde_json::Value::Null,
            },
            width: 2,
            height: 1,
            frames: FrameSource::animated(vec![
                Frame {
                    pixels: vec![Color32::RED, Color32::RED],
                    width: 2,
                    height: 1,
                    left: 0,
                    top: 0,
                    delay: Duration::from_millis(80),
                },
                Frame {
                    pixels: vec![Color32::BLUE, Color32::BLUE],
                    width: 2,
                    height: 1,
                    left: 0,
                    top: 0,
                    delay: Duration::from_millis(120),
                },
            ]),
            midata: None,
            expanded: false,
        }
    }

    #[test]
    fn viewer_converts_jpeg_to_lvgl_v9_lz4() {
        let source = image::RgbaImage::from_fn(8, 8, |x, y| {
            image::Rgba([x as u8 * 16, y as u8 * 16, (x + y) as u8 * 8, 255])
        });
        let mut jpeg = Vec::new();
        let rgb = image::DynamicImage::ImageRgba8(source).to_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 90)
            .encode_image(&rgb)
            .unwrap();
        let dropped = DroppedFile {
            name: "source.jpg".to_string(),
            bytes: Some(std::sync::Arc::from(jpeg)),
            ..Default::default()
        };
        let item = decode_dropped_file(&dropped, ImageFormatCategory::Auto).unwrap();
        let mut params = ConvertParams::default();
        params.output_format = ImageFormat::LVGL;
        params.lvgl_version = crate::image_viewer::model::LvglVersion::V9;
        params.color_format = crate::image_viewer::model::LvglColorFormat::RGB565;
        params.compression = crate::image_viewer::model::LvglCompression::LZ4;

        let (encoded, extension) = convert_image(&item, &params).unwrap();

        assert_eq!(extension, "bin");
        let header = icu_lib::endecoder::lvgl::ImageHeader::parse(&encoded).unwrap();
        assert_eq!(header.version(), icu_lib::endecoder::lvgl::LVGLVersion::V9);
        assert!(icu_lib::endecoder::lvgl::has_flag(
            header.flags(),
            icu_lib::endecoder::lvgl::HeaderFlag::COMPRESSED,
        ));

        params.lvgl_version = crate::image_viewer::model::LvglVersion::V8;
        assert_eq!(
            convert_image(&item, &params).unwrap_err(),
            "LVGL LZ4 compression requires LVGL v9"
        );
    }

    #[test]
    fn viewer_convert_maps_png_and_jpeg_parameters() {
        let source = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 0, 0, 0]));
        let item = single_image_item(
            "source.png".to_string(),
            ImageInfo {
                width: 8,
                height: 8,
                data_size: 0,
                format: "image/png".to_string(),
                other_info: serde_json::Value::Null,
            },
            source.clone(),
            Some(MiData::RGBA(source)),
        );

        let mut params = ConvertParams::default();
        params.output_format = ImageFormat::PNG;
        params.png_color_mode = crate::image_viewer::model::PngColorMode::Rgb;
        let (png, _) = convert_image(&item, &params).unwrap();
        let png = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
        assert_eq!(png.info().color_type, png::ColorType::Rgb);

        params.output_format = ImageFormat::JPEG;
        params.jpeg_quality = 100;
        params.jpeg_background = [10, 120, 240];
        let (jpeg, _) = convert_image(&item, &params).unwrap();
        let pixel = *image::load_from_memory(&jpeg)
            .unwrap()
            .to_rgb8()
            .get_pixel(0, 0);
        assert!((i16::from(pixel[0]) - 10).abs() <= 8);
        assert!((i16::from(pixel[1]) - 120).abs() <= 8);
        assert!((i16::from(pixel[2]) - 240).abs() <= 8);
    }

    #[test]
    fn viewer_png_preserve_keeps_indexed_source() {
        let indexed = icu_lib::midata::IndexedImageData {
            rgba: image::RgbaImage::from_vec(2, 1, vec![1, 2, 3, 255, 9, 8, 7, 64]).unwrap(),
            palette: vec![[1, 2, 3, 255], [9, 8, 7, 64]],
            indexes: vec![0, 1],
            bpp: 1,
            width: 2,
            height: 1,
        };
        let item = image_item_from_midata(
            "indexed.bin".to_string(),
            ImageInfo {
                width: 2,
                height: 1,
                data_size: 0,
                format: "indexed".to_string(),
                other_info: serde_json::Value::Null,
            },
            MiData::INDEXED(indexed),
        )
        .unwrap();
        let mut params = ConvertParams::default();
        params.output_format = ImageFormat::PNG;
        params.png_color_mode = crate::image_viewer::model::PngColorMode::Preserve;

        let (encoded, _) = convert_image(&item, &params).unwrap();
        let decoder = png::Decoder::new(Cursor::new(encoded)).read_info().unwrap();
        assert_eq!(
            decoder.info().palette.as_deref(),
            Some(&[1, 2, 3, 9, 8, 7][..])
        );
        assert_eq!(decoder.info().trns.as_deref(), Some(&[255, 64][..]));

        params.output_format = ImageFormat::LVGL;
        params.lvgl_version = crate::image_viewer::model::LvglVersion::V9;
        params.color_format = crate::image_viewer::model::LvglColorFormat::ARGB8888;
        params.compression = crate::image_viewer::model::LvglCompression::LZ4;
        let (encoded, extension) = convert_image(&item, &params).unwrap();
        assert_eq!(extension, "bin");
        assert!(icu_lib::endecoder::lvgl::has_flag(
            icu_lib::endecoder::lvgl::ImageHeader::parse(&encoded)
                .unwrap()
                .flags(),
            icu_lib::endecoder::lvgl::HeaderFlag::COMPRESSED,
        ));
    }

    #[test]
    fn gif_animation_round_trip_preserves_frame_count() {
        let data = encode_gif_frames(&[animation_item()], GifExportOptions::default()).unwrap();
        let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(data)).unwrap();
        assert_eq!(decoder.into_frames().collect_frames().unwrap().len(), 2);
    }

    #[test]
    fn apng_animation_round_trip_preserves_frame_count() {
        let data = encode_apng_frames(&[animation_item()], GifExportOptions::default()).unwrap();
        let decoder = image::codecs::png::PngDecoder::new(Cursor::new(data)).unwrap();
        assert!(decoder.is_apng().unwrap());
        assert_eq!(
            decoder
                .apng()
                .unwrap()
                .into_frames()
                .collect_frames()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn webp_animation_round_trip_uses_animated_frame_source() {
        let data = encode_webp_frames(&[animation_item()], GifExportOptions::default()).unwrap();
        let item = decode_animation("animation.webp", &data).unwrap();
        assert_eq!(item.frame_count(), 2);
        let FrameSource::Animated { frames, .. } = item.frames else {
            panic!("animated WebP must use an animated frame source");
        };
        assert_eq!(frames[0].pixels, vec![Color32::RED, Color32::RED]);
        assert_eq!(frames[1].pixels, vec![Color32::BLUE, Color32::BLUE]);
        assert_eq!(frames[0].delay, Duration::from_millis(80));
        assert_eq!(frames[1].delay, Duration::from_millis(120));
    }

    #[test]
    fn static_webp_does_not_enter_animation_path() {
        use image::ImageEncoder;

        let pixels = [255, 0, 0, 255];
        let mut data = Vec::new();
        image::codecs::webp::WebPEncoder::new_lossless(&mut data)
            .write_image(&pixels, 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        assert!(decode_animation("static.webp", &data).is_none());
    }
}
