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
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashSet;
use std::io::Cursor;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::path::{Component, Path};
use std::time::Duration;

pub fn process_images_with_format(
    files: &[DroppedFile],
    input_format: ImageFormatCategory,
) -> Vec<ImageItem> {
    #[cfg(not(target_arch = "wasm32"))]
    let expanded_files = {
        let paths = files
            .iter()
            .filter_map(|file| file.path.clone())
            .collect::<Vec<_>>();
        let mut expanded = expand_native_input_paths(&paths);
        expanded.extend(files.iter().filter(|file| file.path.is_none()).cloned());
        expanded
    };
    #[cfg(target_arch = "wasm32")]
    let expanded_files = files.to_vec();

    expanded_files
        .iter()
        .filter_map(|file| decode_dropped_file(file, input_format))
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn expand_native_input_paths(paths: &[PathBuf]) -> Vec<DroppedFile> {
    fn visit(path: &Path, files: &mut Vec<PathBuf>) {
        let Ok(metadata) = std::fs::symlink_metadata(path) else {
            return;
        };
        if metadata.file_type().is_symlink() {
            return;
        }
        if metadata.is_file() {
            files.push(path.to_path_buf());
            return;
        }
        if !metadata.is_dir() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            visit(&entry.path(), files);
        }
    }

    let mut files = Vec::new();
    for path in paths {
        visit(path, &mut files);
    }
    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    files.dedup();
    files
        .into_iter()
        .map(|path| DroppedFile {
            path: Some(path),
            ..Default::default()
        })
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

pub(crate) fn straight_rgba_from_color32(pixels: &[Color32]) -> Vec<u8> {
    pixels
        .iter()
        .flat_map(Color32::to_srgba_unmultiplied)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportMode {
    SingleFile,
    AllFiles,
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
pub struct ExportSource {
    pub input_name: String,
    pub relative_path: Option<String>,
    pub image: ImageItem,
}

#[derive(Clone)]
pub struct ExportRequest {
    pub mode: ExportMode,
    pub targets: Vec<ExportSource>,
    pub params: ConvertParams,
}

fn export_name_and_relative_path(path: &str) -> (String, Option<String>) {
    let path = Path::new(path);
    let input_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let relative_path = (!path.is_absolute())
        .then(|| path.parent())
        .flatten()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.to_string_lossy().into_owned());
    (input_name, relative_path)
}

fn frame_source(name: String, image: ImageItem) -> ExportSource {
    let (input_name, relative_path) = export_name_and_relative_path(&name);
    ExportSource {
        input_name,
        relative_path,
        image,
    }
}

fn animation_frame_source(
    source_path: &str,
    index: usize,
    frame_count: usize,
    image: ImageItem,
) -> ExportSource {
    frame_source(animation_frame_name(source_path, index, frame_count), image)
}

fn animation_frame_name(path: &str, index: usize, frame_count: usize) -> String {
    let path = Path::new(path);
    let width = frame_count.to_string().len().max(2);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let suffix = format!("-{number:0width$}", number = index + 1);
    let file_name = path.extension().map_or_else(
        || format!("{stem}{suffix}"),
        |extension| format!("{stem}{suffix}.{}", extension.to_string_lossy()),
    );
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or(file_name.clone(), |parent| {
            parent.join(file_name).to_string_lossy().into_owned()
        })
}

fn single_export_source(state: &ViewerState, target: ExportTarget) -> Option<ExportSource> {
    match target {
        ExportTarget::Entry(id) => {
            let image = state.item(id)?.as_image()?.clone();
            let name = state.group_label(id).unwrap_or(&image.path);
            let (input_name, relative_path) = export_name_and_relative_path(name);
            Some(ExportSource {
                input_name,
                relative_path,
                image,
            })
        }
        ExportTarget::Frame { collection, index } => {
            if let Some((_, name, image)) = state
                .group_members(collection)
                .and_then(|members| members.into_iter().nth(index))
            {
                return Some(frame_source(name, image));
            }
            let (_, image) = state.frame_snapshots(collection)?.into_iter().nth(index)?;
            let frame_count = state.item(collection)?.as_image()?.frame_count();
            Some(animation_frame_source(
                &state.item(collection)?.as_image()?.path,
                index,
                frame_count,
                image,
            ))
        }
    }
}

fn all_export_sources(state: &ViewerState) -> Vec<ExportSource> {
    state
        .selected_ids
        .iter()
        .copied()
        .flat_map(|id| {
            if let Some(members) = state.group_members(id) {
                return members
                    .into_iter()
                    .map(|(_, name, image)| frame_source(name, image))
                    .collect::<Vec<_>>();
            }
            if let Some(frames) = state.frame_snapshots(id) {
                let Some(source) = state.item(id).and_then(|item| item.as_image()) else {
                    return Vec::new();
                };
                let source_path = source.path.clone();
                let frame_count = source.frame_count();
                return frames
                    .into_iter()
                    .enumerate()
                    .map(|(index, (_, image))| {
                        animation_frame_source(&source_path, index, frame_count, image)
                    })
                    .collect();
            }
            state
                .item(id)
                .and_then(|item| item.as_image())
                .cloned()
                .map(|image| {
                    let (input_name, relative_path) = export_name_and_relative_path(&image.path);
                    vec![ExportSource {
                        input_name,
                        relative_path,
                        image,
                    }]
                })
                .unwrap_or_default()
        })
        .collect()
}

pub fn resolve_export_request(
    state: &ViewerState,
    mode: ExportMode,
    single_target: Option<ExportTarget>,
    params: &ConvertParams,
) -> Result<ExportRequest, String> {
    let targets = match mode {
        ExportMode::SingleFile => {
            let target = single_target
                .ok_or_else(|| "Single-file export requires one target".to_string())?;
            let selected_id = match target {
                ExportTarget::Entry(id) => id,
                ExportTarget::Frame { collection, .. } => collection,
            };
            if state.selected_ids.len() != 1 || !state.selected_ids.contains(&selected_id) {
                return Err("Single-file export requires exactly one selected source".to_string());
            }
            vec![
                single_export_source(state, target)
                    .ok_or_else(|| "Single-file export target is not available".to_string())?,
            ]
        }
        ExportMode::AllFiles => all_export_sources(state),
    };
    if targets.is_empty() {
        return Err("Export request has no image sources".to_string());
    }
    Ok(ExportRequest {
        mode,
        targets,
        params: params.clone(),
    })
}

pub fn export_target_from_selection(state: &ViewerState) -> Option<ExportTarget> {
    match state.primary_target? {
        SelectionTarget::Entry(id) => Some(ExportTarget::Entry(id)),
        SelectionTarget::Frame { collection, index } => {
            Some(ExportTarget::Frame { collection, index })
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn save_export_request(request: &ExportRequest) {
    if request.mode != ExportMode::SingleFile || request.targets.len() != 1 {
        log::error!("Native single-file export requires exactly one export source");
        return;
    }
    let source = &request.targets[0];
    let (data, extension) = match encode_export_source_with_params(source, &request.params) {
        Ok(encoded) => encoded,
        Err(error) => {
            log::error!("Failed to encode {}: {error}", source.input_name);
            return;
        }
    };
    let default_name = format!(
        "{}.{}",
        Path::new(&source.input_name)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy(),
        extension
    );
    if let Some(path) = rfd::FileDialog::new()
        .set_file_name(default_name)
        .save_file()
        && let Err(error) = write_native_export(&path, data, &extension)
    {
        log::error!("Failed to save native export: {error}");
    }
}

#[cfg(all(not(target_arch = "wasm32"), test))]
pub fn save_export_request_to_path(
    request: &ExportRequest,
    path: &Path,
) -> Result<std::path::PathBuf, String> {
    if request.mode != ExportMode::SingleFile || request.targets.len() != 1 {
        return Err("Native single-file export requires exactly one export source".to_string());
    }
    let source = &request.targets[0];
    let (data, extension) = encode_export_source_with_params(source, &request.params)?;
    write_native_export(path, data, &extension)
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, PartialEq, Eq)]
pub enum NativeBatchExportError {
    InvalidRequest(String),
    InvalidSourcePath { source: String, reason: String },
    Encode { source: String, error: String },
    CreateDirectory { path: PathBuf, error: String },
    Write { path: PathBuf, error: String },
}

#[cfg(not(target_arch = "wasm32"))]
impl std::fmt::Display for NativeBatchExportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(error) => formatter.write_str(error),
            Self::InvalidSourcePath { source, reason } => {
                write!(formatter, "Invalid export path for {source}: {reason}")
            }
            Self::Encode { source, error } => {
                write!(formatter, "Failed to encode {source}: {error}")
            }
            Self::CreateDirectory { path, error } => {
                write!(formatter, "Failed to create {}: {error}", path.display())
            }
            Self::Write { path, error } => {
                write!(formatter, "Failed to write {}: {error}", path.display())
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl std::error::Error for NativeBatchExportError {}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn save_all_export_request(request: &ExportRequest) {
    let Some(directory) = rfd::FileDialog::new().pick_folder() else {
        return;
    };
    if let Err(error) = save_export_request_to_directory(request, &directory) {
        log::error!("Failed to save native batch export: {error}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_export_request_to_directory(
    request: &ExportRequest,
    directory: &Path,
) -> Result<Vec<PathBuf>, NativeBatchExportError> {
    if request.mode != ExportMode::AllFiles {
        return Err(NativeBatchExportError::InvalidRequest(
            "Native batch export requires all-files mode".to_string(),
        ));
    }
    if request.targets.is_empty() {
        return Err(NativeBatchExportError::InvalidRequest(
            "Native batch export requires at least one export source".to_string(),
        ));
    }

    let mut used_paths = HashSet::new();
    let mut encoded = Vec::with_capacity(request.targets.len());
    for source in &request.targets {
        let relative_directory = safe_relative_directory(source)?;
        let (data, extension) =
            encode_export_source_with_params(source, &request.params).map_err(|error| {
                NativeBatchExportError::Encode {
                    source: source.input_name.clone(),
                    error,
                }
            })?;
        let stem = Path::new(&source.input_name)
            .file_stem()
            .ok_or_else(|| NativeBatchExportError::InvalidSourcePath {
                source: source.input_name.clone(),
                reason: "file name has no stem".to_string(),
            })?
            .to_string_lossy();
        let base_name = format!("{stem}.{extension}");
        let relative_path = unique_batch_path(
            directory,
            &relative_directory,
            &base_name,
            &extension,
            &mut used_paths,
        );
        encoded.push((relative_path, data, base_name, extension));
    }

    let mut outputs = Vec::with_capacity(encoded.len());
    for (mut relative_path, data, base_name, extension) in encoded {
        let relative_parent = relative_path.parent().unwrap_or(Path::new(""));
        ensure_native_export_directory(directory, relative_parent)?;
        let mut data = Some(data);
        let output_path = loop {
            let candidate = directory.join(&relative_path);
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(mut file) => {
                    file.write_all(&data.take().expect("encoded bytes are written once"))
                        .map_err(|error| NativeBatchExportError::Write {
                            path: candidate.clone(),
                            error: error.to_string(),
                        })?;
                    break candidate;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    relative_path = unique_batch_path(
                        directory,
                        relative_path.parent().unwrap_or(Path::new("")),
                        &base_name,
                        &extension,
                        &mut used_paths,
                    );
                }
                Err(error) => {
                    return Err(NativeBatchExportError::Write {
                        path: candidate,
                        error: error.to_string(),
                    });
                }
            }
        };
        outputs.push(output_path);
    }
    Ok(outputs)
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_native_export_directory(
    root: &Path,
    relative: &Path,
) -> Result<(), NativeBatchExportError> {
    let root_metadata = std::fs::symlink_metadata(root).map_err(|error| {
        NativeBatchExportError::CreateDirectory {
            path: root.to_path_buf(),
            error: error.to_string(),
        }
    })?;
    if !root_metadata.file_type().is_dir() {
        return Err(NativeBatchExportError::CreateDirectory {
            path: root.to_path_buf(),
            error: "export root is not a directory or is a symlink".to_string(),
        });
    }

    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(NativeBatchExportError::InvalidSourcePath {
                source: relative.display().to_string(),
                reason: "relative directory contains an unsafe component".to_string(),
            });
        };
        current.push(name);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(NativeBatchExportError::CreateDirectory {
                    path: current,
                    error: "export directory contains a symlink".to_string(),
                });
            }
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => {
                return Err(NativeBatchExportError::CreateDirectory {
                    path: current,
                    error: "export path component is not a directory".to_string(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&current).map_err(|error| {
                    NativeBatchExportError::CreateDirectory {
                        path: current.clone(),
                        error: error.to_string(),
                    }
                })?;
            }
            Err(error) => {
                return Err(NativeBatchExportError::CreateDirectory {
                    path: current,
                    error: error.to_string(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn safe_relative_directory(source: &ExportSource) -> Result<PathBuf, NativeBatchExportError> {
    let input_path = Path::new(&source.input_name);
    if source.input_name.is_empty()
        || input_path.is_absolute()
        || input_path.components().count() != 1
        || !matches!(input_path.components().next(), Some(Component::Normal(_)))
    {
        return Err(NativeBatchExportError::InvalidSourcePath {
            source: source.input_name.clone(),
            reason: "input name must be one relative file name".to_string(),
        });
    }

    let mut safe = PathBuf::new();
    if let Some(relative_path) = &source.relative_path {
        let path = Path::new(relative_path);
        if path.is_absolute()
            || path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(NativeBatchExportError::InvalidSourcePath {
                source: source.input_name.clone(),
                reason: format!("relative directory escapes export root: {relative_path}"),
            });
        }
        safe.push(path);
    }
    Ok(safe)
}

#[cfg(not(target_arch = "wasm32"))]
fn unique_batch_path(
    root: &Path,
    relative_directory: &Path,
    base_name: &str,
    extension: &str,
    used_paths: &mut HashSet<PathBuf>,
) -> PathBuf {
    let stem = Path::new(base_name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    for index in 1.. {
        let file_name = if index == 1 {
            base_name.to_string()
        } else {
            format!("{stem}-{index}.{extension}")
        };
        let candidate = relative_directory.join(file_name);
        if !used_paths.contains(&candidate) && !root.join(&candidate).exists() {
            used_paths.insert(candidate.clone());
            return candidate;
        }
    }
    unreachable!("an unused numeric file suffix always exists")
}

#[cfg(not(target_arch = "wasm32"))]
fn write_native_export(
    path: &Path,
    data: Vec<u8>,
    extension: &str,
) -> Result<std::path::PathBuf, String> {
    let output_path = path.with_extension(extension);
    std::fs::write(&output_path, data)
        .map_err(|error| format!("{}: {error}", output_path.display()))?;
    Ok(output_path)
}

fn encode_export_source_with_params(
    source: &ExportSource,
    params: &ConvertParams,
) -> Result<(Vec<u8>, String), String> {
    let has_animation = source.image.frame_count() > 1;
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
        return match params.output_format {
            ImageFormat::GIF => encode_gif_frames(std::slice::from_ref(&source.image), options)
                .map(|data| (data, "gif".to_string())),
            ImageFormat::APNG => encode_apng_frames(std::slice::from_ref(&source.image), options)
                .map(|data| (data, "apng".to_string())),
            ImageFormat::WEBP => encode_webp_frames(std::slice::from_ref(&source.image), options)
                .map(|data| (data, "webp".to_string())),
            _ => unreachable!("animation export format was checked above"),
        };
    }
    convert_image(&source.image, params)
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

#[cfg(target_arch = "wasm32")]
fn zip_source_path(
    source: &ExportSource,
    extension: &str,
    used: &mut std::collections::HashSet<String>,
) -> Result<String, String> {
    let name = Path::new(&source.input_name);
    if source.input_name.is_empty()
        || name.is_absolute()
        || name.components().count() != 1
        || !matches!(name.components().next(), Some(Component::Normal(_)))
    {
        return Err(format!("Invalid export file name: {}", source.input_name));
    }
    let mut directory = String::new();
    if let Some(relative) = &source.relative_path {
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(format!("Invalid export directory: {relative}"));
        }
        directory = path.to_string_lossy().replace('\\', "/");
    }
    let stem = name.file_stem().unwrap().to_string_lossy();
    for index in 1.. {
        let file = if index == 1 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem}-{index}.{extension}")
        };
        let candidate = if directory.is_empty() {
            file
        } else {
            format!("{directory}/{file}")
        };
        if used.insert(candidate.clone()) {
            return Ok(candidate);
        }
    }
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
fn crc32(data: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & (!((crc & 1).wrapping_sub(1))));
        }
    }
    !crc
}

#[cfg(target_arch = "wasm32")]
fn build_stored_zip(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut archive = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let name = name.as_bytes();
        let offset = u32::try_from(archive.len()).map_err(|_| "ZIP is too large".to_string())?;
        let size = u32::try_from(data.len()).map_err(|_| "ZIP entry is too large".to_string())?;
        let checksum = crc32(data);
        archive.extend_from_slice(&0x04034b50u32.to_le_bytes());
        archive.extend_from_slice(&20u16.to_le_bytes());
        archive.extend_from_slice(&0x0800u16.to_le_bytes());
        archive.extend_from_slice(&0u16.to_le_bytes());
        archive.extend_from_slice(&0u16.to_le_bytes());
        archive.extend_from_slice(&0u16.to_le_bytes());
        archive.extend_from_slice(&checksum.to_le_bytes());
        archive.extend_from_slice(&size.to_le_bytes());
        archive.extend_from_slice(&size.to_le_bytes());
        archive.extend_from_slice(
            &(u16::try_from(name.len()).map_err(|_| "ZIP name is too long".to_string())?)
                .to_le_bytes(),
        );
        archive.extend_from_slice(&0u16.to_le_bytes());
        archive.extend_from_slice(name);
        archive.extend_from_slice(data);

        central.extend_from_slice(&0x02014b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&checksum.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(
            &(u16::try_from(name.len()).map_err(|_| "ZIP name is too long".to_string())?)
                .to_le_bytes(),
        );
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name);
    }
    let central_offset =
        u32::try_from(archive.len()).map_err(|_| "ZIP is too large".to_string())?;
    let central_size = u32::try_from(central.len()).map_err(|_| "ZIP is too large".to_string())?;
    archive.extend_from_slice(&central);
    archive.extend_from_slice(&0x06054b50u32.to_le_bytes());
    archive.extend_from_slice(&0u16.to_le_bytes());
    archive.extend_from_slice(&0u16.to_le_bytes());
    let count = u16::try_from(entries.len()).map_err(|_| "Too many ZIP entries".to_string())?;
    archive.extend_from_slice(&count.to_le_bytes());
    archive.extend_from_slice(&count.to_le_bytes());
    archive.extend_from_slice(&central_size.to_le_bytes());
    archive.extend_from_slice(&central_offset.to_le_bytes());
    archive.extend_from_slice(&0u16.to_le_bytes());
    Ok(archive)
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub fn save_all_export_request(request: &ExportRequest) -> Result<(), String> {
    if request.mode != ExportMode::AllFiles || request.targets.is_empty() {
        return Err("Web batch export requires all-files mode with sources".to_string());
    }
    let mut used = std::collections::HashSet::new();
    let mut entries = Vec::with_capacity(request.targets.len());
    for source in &request.targets {
        let (data, extension) = encode_export_source_with_params(source, &request.params)?;
        let relative = zip_source_path(source, &extension, &mut used)?;
        entries.push((relative, data));
    }
    save_export_bytes("export", "zip", build_stored_zip(&entries)?);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub fn save_export_request(request: &ExportRequest) {
    if request.mode != ExportMode::SingleFile || request.targets.len() != 1 {
        log::error!("Web single-file export requires exactly one export source");
        return;
    }
    let source = &request.targets[0];
    let (data, extension) = match encode_export_source_with_params(source, &request.params) {
        Ok(encoded) => encoded,
        Err(error) => {
            log::error!("Failed to encode {}: {error}", source.input_name);
            return;
        }
    };
    let label = Path::new(&source.input_name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    save_export_bytes(&label, &extension, data);
}

pub fn save_resolved_export_request(request: &ExportRequest) {
    #[cfg(not(target_arch = "wasm32"))]
    match request.mode {
        ExportMode::SingleFile => save_export_request(request),
        ExportMode::AllFiles => save_all_export_request(request),
    }
    #[cfg(target_arch = "wasm32")]
    match request.mode {
        ExportMode::SingleFile => save_export_request(request),
        ExportMode::AllFiles => {
            if let Err(error) = save_all_export_request(request) {
                log::error!("Failed to save web batch export: {error}");
            }
        }
    }
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
        writer
            .write_image_data(&frame.rgba)
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

fn animation_frames(items: &[ImageItem]) -> Vec<super::webp_animation::ExportFrame> {
    items
        .iter()
        .flat_map(|item| match &item.frames {
            FrameSource::Single {
                pixels,
                width,
                height,
            } => {
                let rgba = match &item.midata {
                    Some(MiData::INDEXED(indexed))
                        if indexed.width == *width && indexed.height == *height =>
                    {
                        indexed.rgba.as_raw().clone()
                    }
                    _ => straight_rgba_from_color32(pixels),
                };
                vec![super::webp_animation::ExportFrame {
                    rgba,
                    width: *width,
                    height: *height,
                    left: 0,
                    top: 0,
                    delay: Duration::ZERO,
                }]
            }
            FrameSource::Animated { frames, .. } => frames
                .iter()
                .map(|frame| super::webp_animation::ExportFrame {
                    rgba: straight_rgba_from_color32(&frame.pixels),
                    width: frame.width,
                    height: frame.height,
                    left: frame.left,
                    top: frame.top,
                    delay: frame.delay,
                })
                .collect(),
        })
        .collect()
}

fn encoded_frame(
    pixels: &[Color32],
    width: u32,
    height: u32,
    delay: Duration,
) -> Result<EncodedFrame, String> {
    let bytes = straight_rgba_from_color32(pixels);
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
    if output_format == ImageFormat::APNG {
        return Err("APNG output requires an animated export target".to_string());
    }
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
                MiData::from_rgba(width, height, straight_rgba_from_color32(pixels))
                    .ok_or("Failed to create MiData")?
            }
        }
    } else {
        match &image_item.midata {
            Some(MiData::INDEXED(indexed)) => MiData::RGBA(indexed.rgba.clone()),
            _ => {
                let (pixels, width, height) = image_item.current_pixels();
                MiData::from_rgba(width, height, straight_rgba_from_color32(pixels))
                    .ok_or("Failed to create MiData")?
            }
        }
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
pub fn install_web_directory_drop(
    pending: std::rc::Rc<std::cell::RefCell<Vec<DroppedFile>>>,
    ctx: eframe::egui::Context,
) {
    use eframe::wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use std::rc::Rc;

    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let pending = Rc::new(pending);
    let ctx = Rc::new(ctx);
    let on_drag_over = Closure::<dyn FnMut(_)>::new(move |event: web_sys::Event| {
        event.prevent_default();
    });
    let _ = document
        .add_event_listener_with_callback("dragover", on_drag_over.as_ref().unchecked_ref());
    on_drag_over.forget();

    let on_drop = Closure::<dyn FnMut(_)>::new(move |event: web_sys::Event| {
        event.prevent_default();
        event.stop_propagation();
        let Some(data_transfer) =
            js_sys::Reflect::get(event.as_ref(), &JsValue::from_str("dataTransfer")).ok()
        else {
            return;
        };
        let script = js_sys::Function::new_with_args(
            "dt",
            r#"return (async () => {
                const readEntries = reader => new Promise(resolve => {
                    const all = [];
                    const read = () => reader.readEntries(entries => {
                        if (!entries.length) return resolve(all);
                        all.push(...entries);
                        read();
                    }, () => resolve(all));
                    read();
                });
                const readEntry = (entry, prefix) => new Promise(resolve => {
                    if (entry.isFile) {
                        entry.file(file => file.arrayBuffer().then(buffer => resolve([{
                            name: prefix + file.name,
                            bytes: new Uint8Array(buffer)
                        }])).catch(() => resolve([])), () => resolve([]));
                        return;
                    }
                    if (!entry.isDirectory) return resolve([]);
                    readEntries(entry.createReader()).then(async entries => {
                        const result = [];
                        for (const child of entries)
                            result.push(...await readEntry(child, prefix + entry.name + '/'));
                        resolve(result);
                    });
                });
                const readHandle = async (handle, prefix) => {
                    if (handle.kind === 'file') {
                        const file = await handle.getFile();
                        return [{ name: prefix + file.name, bytes: new Uint8Array(await file.arrayBuffer()) }];
                    }
                    const result = [];
                    for await (const child of handle.values())
                        result.push(...await readHandle(child, prefix + handle.name + '/'));
                    return result;
                };
                const result = [];
                for (const item of dt.items) {
                    let handle = null;
                    try { handle = await item.getAsFileSystemHandle?.(); } catch (_) {}
                    if (handle) result.push(...await readHandle(handle, ''));
                    else {
                        const entry = item.webkitGetAsEntry?.();
                        if (entry) result.push(...await readEntry(entry, ''));
                        else {
                            const file = item.getAsFile?.();
                            if (file) result.push({ name: file.name, bytes: new Uint8Array(await file.arrayBuffer()) });
                        }
                    }
                }
                return result;
            })()"#,
        );
        let Ok(promise) = script.call1(&JsValue::NULL, &data_transfer) else {
            return;
        };
        let pending = pending.clone();
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(value) =
                wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await
            else {
                return;
            };
            let Some(files) = value.dyn_ref::<js_sys::Array>() else {
                return;
            };
            let mut output = Vec::new();
            for value in files.iter() {
                let Ok(name) = js_sys::Reflect::get(&value, &JsValue::from_str("name"))
                    .and_then(|value| value.as_string().ok_or(JsValue::UNDEFINED))
                else {
                    continue;
                };
                let Ok(bytes) = js_sys::Reflect::get(&value, &JsValue::from_str("bytes")) else {
                    continue;
                };
                output.push(DroppedFile {
                    name,
                    bytes: Some(std::sync::Arc::from(
                        js_sys::Uint8Array::new(&bytes).to_vec(),
                    )),
                    ..Default::default()
                });
            }
            if !output.is_empty() {
                pending.borrow_mut().extend(output);
                ctx.request_repaint();
            }
        });
    });
    let _ = document.add_event_listener_with_callback("drop", on_drop.as_ref().unchecked_ref());
    on_drop.forget();
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
    let _ = input.set_attribute("webkitdirectory", "");
    let _ = input.set_attribute("directory", "");

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
                    let name = js_sys::Reflect::get(
                        file.as_ref(),
                        &eframe::wasm_bindgen::JsValue::from_str("webkitRelativePath"),
                    )
                    .ok()
                    .and_then(|path| path.as_string())
                    .filter(|path| !path.is_empty())
                    .unwrap_or_else(|| file.name());
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
    use crate::image_viewer::model::SidebarItem;
    use image::AnimationDecoder;

    const SEMI_TRANSPARENT_RGBA: [u8; 8] = [255, 230, 27, 135, 41, 203, 77, 16];

    fn semi_transparent_indexed_item() -> ImageItem {
        let rgba = image::RgbaImage::from_raw(2, 1, SEMI_TRANSPARENT_RGBA.to_vec()).unwrap();
        image_item_from_midata(
            "semi-transparent-indexed.png".to_string(),
            ImageInfo {
                width: 2,
                height: 1,
                data_size: 0,
                format: "indexed".to_string(),
                other_info: serde_json::Value::Null,
            },
            MiData::INDEXED(icu_lib::midata::IndexedImageData {
                rgba,
                palette: vec![[255, 230, 27, 135], [41, 203, 77, 16]],
                indexes: vec![0, 1],
                bpp: 1,
                width: 2,
                height: 1,
            }),
        )
        .unwrap()
    }

    fn rgba_bytes(data: MiData) -> Vec<u8> {
        match data {
            MiData::RGBA(image) => image.into_raw(),
            MiData::INDEXED(indexed) => indexed.rgba.into_raw(),
            _ => panic!("expected decoded RGBA pixels"),
        }
    }

    fn png_chunks(data: &[u8]) -> Vec<&[u8]> {
        let mut chunks = Vec::new();
        let mut offset = 8;
        while offset + 12 <= data.len() {
            let length = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            let chunk_end = offset + 12 + length;
            assert!(chunk_end <= data.len());
            chunks.push(&data[offset + 4..offset + 8]);
            offset = chunk_end;
        }
        assert_eq!(offset, data.len());
        chunks
    }

    fn image_item(path: &str) -> ImageItem {
        ImageItem {
            path: path.to_string(),
            info: ImageInfo {
                width: 1,
                height: 1,
                data_size: 4,
                format: "rgba".to_string(),
                other_info: serde_json::Value::Null,
            },
            width: 1,
            height: 1,
            frames: FrameSource::single(vec![Color32::BLACK], 1, 1),
            midata: None,
            expanded: false,
        }
    }

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
    fn single_file_request_resolves_one_static_source() {
        let mut state = ViewerState::default();
        let ids =
            state.insert_and_select_first([SidebarItem::Image(image_item("assets/icon.png"))]);
        let params = ConvertParams::default();

        let request = resolve_export_request(
            &state,
            ExportMode::SingleFile,
            Some(ExportTarget::Entry(ids[0])),
            &params,
        )
        .unwrap();

        assert_eq!(request.mode, ExportMode::SingleFile);
        assert_eq!(request.targets.len(), 1);
        assert_eq!(request.targets[0].input_name, "icon.png");
        assert_eq!(request.targets[0].relative_path.as_deref(), Some("assets"));
        assert_eq!(request.targets[0].image.path, "assets/icon.png");
        assert_eq!(request.params.output_format, params.output_format);
    }

    #[test]
    fn group_request_is_one_logical_source_or_stable_member_sources() {
        let mut state = ViewerState::default();
        let member_ids = state.insert_and_select_first([
            SidebarItem::Image(image_item("walk-left.png")),
            SidebarItem::Image(image_item("walk-right.png")),
        ]);
        assert!(state.toggle_selection(member_ids[1]));
        let group_id = state.group_selected().unwrap();
        let params = ConvertParams::default();

        let single = resolve_export_request(
            &state,
            ExportMode::SingleFile,
            Some(ExportTarget::Entry(group_id)),
            &params,
        )
        .unwrap();
        assert_eq!(single.targets.len(), 1);
        assert_eq!(single.targets[0].image.frame_count(), 2);

        let all = resolve_export_request(&state, ExportMode::AllFiles, None, &params).unwrap();
        assert_eq!(
            all.targets
                .iter()
                .map(|source| source.input_name.as_str())
                .collect::<Vec<_>>(),
            vec!["walk-left.png", "walk-right.png"]
        );
    }

    #[test]
    fn animation_request_expands_frames_and_resolves_explicit_frame() {
        let mut state = ViewerState::default();
        let id = state.insert_and_select_first([SidebarItem::Image(animation_item())])[0];
        let params = ConvertParams::default();

        let all = resolve_export_request(&state, ExportMode::AllFiles, None, &params).unwrap();
        assert_eq!(all.targets.len(), 2);
        assert_eq!(
            all.targets
                .iter()
                .map(|source| (source.input_name.as_str(), source.image.frame_count()))
                .collect::<Vec<_>>(),
            vec![("animation-01", 1), ("animation-02", 1)]
        );

        assert!(state.select_frame(id, 1));
        let single = resolve_export_request(
            &state,
            ExportMode::SingleFile,
            Some(ExportTarget::Frame {
                collection: id,
                index: 1,
            }),
            &params,
        )
        .unwrap();
        assert_eq!(single.targets.len(), 1);
        assert_eq!(single.targets[0].input_name, "animation-02");
        assert_eq!(single.targets[0].image.frame_count(), 1);
    }

    #[test]
    fn mixed_all_files_uses_stable_selected_ids_not_primary_target() {
        let mut state = ViewerState::default();
        let ids = state.insert_and_select_first([
            SidebarItem::Image(image_item("static.png")),
            SidebarItem::Image(animation_item()),
        ]);
        assert!(state.toggle_selection(ids[1]));
        assert_eq!(state.primary_target, Some(SelectionTarget::Entry(ids[1])));
        let params = ConvertParams::default();

        let all = resolve_export_request(&state, ExportMode::AllFiles, None, &params).unwrap();
        assert_eq!(
            all.targets
                .iter()
                .map(|source| source.input_name.as_str())
                .collect::<Vec<_>>(),
            vec!["static.png", "animation-01", "animation-02"]
        );

        assert_eq!(
            resolve_export_request(
                &state,
                ExportMode::SingleFile,
                Some(ExportTarget::Entry(ids[1])),
                &params,
            )
            .err()
            .unwrap(),
            "Single-file export requires exactly one selected source"
        );
    }

    #[cfg(all(not(target_arch = "wasm32"), unix))]
    #[test]
    fn native_input_expansion_is_sorted_deduplicated_and_does_not_follow_symlinks() {
        let root = std::env::temp_dir().join(format!("icu-native-input-{}", std::process::id()));
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join("z.png"), b"z").unwrap();
        std::fs::write(nested.join("a.png"), b"a").unwrap();
        let linked = root.join("linked");
        std::os::unix::fs::symlink(&nested, &linked).unwrap();

        let files = expand_native_input_paths(&[root.clone(), root.clone()]);
        let paths = files
            .iter()
            .map(|file| file.path.as_ref().unwrap())
            .collect::<Vec<_>>();
        let expected_nested = nested.join("a.png");
        let expected_root = root.join("z.png");
        assert_eq!(paths, vec![&expected_nested, &expected_root]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn process_images_decodes_files_from_native_directories() {
        let root = std::env::temp_dir().join(format!(
            "icu-native-directory-decode-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("pixel.png");
        image::RgbaImage::from_pixel(1, 1, image::Rgba([12, 34, 56, 255]))
            .save(&path)
            .unwrap();

        let items = process_images_with_format(
            &[DroppedFile {
                path: Some(root.clone()),
                ..Default::default()
            }],
            ImageFormatCategory::Auto,
        );

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, path.display().to_string());
        assert_eq!(items[0].width, 1);
        assert_eq!(items[0].height, 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_single_file_export_writes_encoded_bytes_with_format_extension() {
        let mut state = ViewerState::default();
        let id = state.insert_and_select_first([SidebarItem::Image(image_item("source.input"))])[0];
        let mut params = ConvertParams::default();
        params.output_format = ImageFormat::PNG;
        let request = resolve_export_request(
            &state,
            ExportMode::SingleFile,
            Some(ExportTarget::Entry(id)),
            &params,
        )
        .unwrap();

        let root = std::env::temp_dir().join(format!("icu-native-export-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let requested_path = root.join("chosen-name.wrong");
        let output_path = save_export_request_to_path(&request, &requested_path).unwrap();

        assert_eq!(output_path, root.join("chosen-name.png"));
        let bytes = std::fs::read(&output_path).unwrap();
        assert_eq!(bytes[..8], [137, 80, 78, 71, 13, 10, 26, 10]);
        assert!(image::load_from_memory(&bytes).is_ok());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_single_file_export_returns_encoding_errors_before_writing() {
        let source = ExportSource {
            input_name: "source.png".to_string(),
            relative_path: None,
            image: image_item("source.png"),
        };
        let mut params = ConvertParams::default();
        params.output_format = ImageFormat::APNG;
        let request = ExportRequest {
            mode: ExportMode::SingleFile,
            targets: vec![source],
            params,
        };
        let error = save_export_request_to_path(
            &request,
            &std::env::temp_dir().join("icu-native-export-error.png"),
        )
        .unwrap_err();
        assert_eq!(error, "APNG output requires an animated export target");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_batch_export_preserves_paths_collisions_and_existing_files() {
        let mut state = ViewerState::default();
        let ids = state.insert_and_select_first([
            SidebarItem::Image(image_item("first.png")),
            SidebarItem::Image(image_item("second.png")),
        ]);
        let mut params = ConvertParams::default();
        params.output_format = ImageFormat::PNG;
        let request = ExportRequest {
            mode: ExportMode::AllFiles,
            targets: ids
                .into_iter()
                .map(|_| ExportSource {
                    input_name: "icon.input".to_string(),
                    relative_path: Some("themes/dark".to_string()),
                    image: image_item("icon.input"),
                })
                .collect(),
            params,
        };
        let root = std::env::temp_dir().join(format!("icu-native-batch-{}", std::process::id()));
        let nested = root.join("themes/dark");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("icon.png"), b"existing").unwrap();

        let outputs = save_export_request_to_directory(&request, &root).unwrap();

        assert_eq!(
            outputs,
            vec![nested.join("icon-2.png"), nested.join("icon-3.png")]
        );
        assert_eq!(std::fs::read(nested.join("icon.png")).unwrap(), b"existing");
        assert!(outputs.iter().all(|path| image::open(path).is_ok()));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(all(not(target_arch = "wasm32"), unix))]
    #[test]
    fn native_batch_export_does_not_follow_existing_symlink() {
        let mut state = ViewerState::default();
        let _id = state.insert_and_select_first([SidebarItem::Image(image_item("source.png"))])[0];
        let params = ConvertParams::default();
        let request = ExportRequest {
            mode: ExportMode::AllFiles,
            targets: vec![ExportSource {
                input_name: "source.png".to_string(),
                relative_path: None,
                image: image_item("source.png"),
            }],
            params,
        };
        let root = std::env::temp_dir().join(format!("icu-native-symlink-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let protected = root.join("protected.png");
        std::fs::write(&protected, b"protected").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&protected, root.join("source.png")).unwrap();

        let outputs = save_export_request_to_directory(&request, &root).unwrap();

        assert_eq!(std::fs::read(&protected).unwrap(), b"protected");
        assert_ne!(outputs[0], root.join("source.png"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(all(not(target_arch = "wasm32"), unix))]
    #[test]
    fn native_batch_export_rejects_symlinked_output_directory() {
        let params = ConvertParams::default();
        let request = ExportRequest {
            mode: ExportMode::AllFiles,
            targets: vec![ExportSource {
                input_name: "source.png".to_string(),
                relative_path: Some("linked".to_string()),
                image: image_item("source.png"),
            }],
            params,
        };
        let root =
            std::env::temp_dir().join(format!("icu-native-dir-symlink-{}", std::process::id()));
        let outside =
            std::env::temp_dir().join(format!("icu-native-dir-outside-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();

        assert!(matches!(
            save_export_request_to_directory(&request, &root).unwrap_err(),
            NativeBatchExportError::CreateDirectory { .. }
        ));
        assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(outside).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_batch_export_rejects_escape_before_writing() {
        let mut state = ViewerState::default();
        let _id = state.insert_and_select_first([SidebarItem::Image(image_item("source.png"))])[0];
        let params = ConvertParams::default();
        let request = ExportRequest {
            mode: ExportMode::AllFiles,
            targets: vec![ExportSource {
                input_name: "source.png".to_string(),
                relative_path: Some("../outside".to_string()),
                image: image_item("source.png"),
            }],
            params,
        };
        let root = std::env::temp_dir().join(format!("icu-native-escape-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();

        assert!(matches!(
            save_export_request_to_directory(&request, &root).unwrap_err(),
            NativeBatchExportError::InvalidSourcePath { .. }
        ));
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_batch_export_encodes_all_sources_before_writing() {
        let mut params = ConvertParams::default();
        params.output_format = ImageFormat::APNG;
        let request = ExportRequest {
            mode: ExportMode::AllFiles,
            targets: vec![
                ExportSource {
                    input_name: "animation.gif".to_string(),
                    relative_path: None,
                    image: animation_item(),
                },
                ExportSource {
                    input_name: "static.png".to_string(),
                    relative_path: None,
                    image: image_item("static.png"),
                },
            ],
            params,
        };
        let root = std::env::temp_dir().join(format!("icu-native-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();

        assert!(matches!(
            save_export_request_to_directory(&request, &root).unwrap_err(),
            NativeBatchExportError::Encode { .. }
        ));
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
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
    fn semi_transparent_indexed_exports_preserve_straight_rgba() {
        let item = semi_transparent_indexed_item();
        let mut params = ConvertParams::default();

        params.output_format = ImageFormat::PNG;
        params.png_color_mode = crate::image_viewer::model::PngColorMode::Rgba;
        let (png, _) = convert_image(&item, &params).unwrap();
        assert_eq!(
            image::load_from_memory(&png).unwrap().to_rgba8().into_raw(),
            SEMI_TRANSPARENT_RGBA
        );

        let apng = encode_apng_frames(&[item.clone()], GifExportOptions::default()).unwrap();
        let apng = image::codecs::png::PngDecoder::new(Cursor::new(apng))
            .unwrap()
            .apng()
            .unwrap()
            .into_frames()
            .collect_frames()
            .unwrap();
        assert_eq!(apng[0].buffer().as_raw(), &SEMI_TRANSPARENT_RGBA);

        let webp = encode_webp_frames(&[item.clone()], GifExportOptions::default()).unwrap();
        let webp = image::codecs::webp::WebPDecoder::new(Cursor::new(webp))
            .unwrap()
            .into_frames()
            .collect_frames()
            .unwrap();
        assert_eq!(webp[0].buffer().as_raw(), &SEMI_TRANSPARENT_RGBA);

        params.output_format = ImageFormat::LVGL;
        params.lvgl_version = crate::image_viewer::model::LvglVersion::V9;
        params.color_format = crate::image_viewer::model::LvglColorFormat::ARGB8888;
        let (lvgl, _) = convert_image(&item, &params).unwrap();
        assert_eq!(
            rgba_bytes(icu_lib::endecoder::lvgl::LVGL {}.decode(lvgl)),
            SEMI_TRANSPARENT_RGBA
        );

        params.output_format = ImageFormat::MIRX;
        params.color_format = crate::image_viewer::model::LvglColorFormat::RGBA8888;
        let (mirx, _) = convert_image(&item, &params).unwrap();
        assert_eq!(
            rgba_bytes(icu_lib::endecoder::mirui::Mirx {}.decode(mirx)),
            SEMI_TRANSPARENT_RGBA
        );

        let jpeg_rgba = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 230, 27, 135]));
        let jpeg_item = image_item_from_midata(
            "semi-transparent-indexed-jpeg.png".to_string(),
            ImageInfo {
                width: 8,
                height: 8,
                data_size: 0,
                format: "indexed".to_string(),
                other_info: serde_json::Value::Null,
            },
            MiData::INDEXED(icu_lib::midata::IndexedImageData {
                rgba: jpeg_rgba,
                palette: vec![[255, 230, 27, 135]],
                indexes: vec![0; 64],
                bpp: 1,
                width: 8,
                height: 8,
            }),
        )
        .unwrap();
        params.output_format = ImageFormat::JPEG;
        params.jpeg_quality = 100;
        params.jpeg_background = [0, 0, 0];
        let (jpeg, _) = convert_image(&jpeg_item, &params).unwrap();
        let jpeg = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        let pixel = jpeg.get_pixel(4, 4);
        for (actual, expected) in pixel.0.into_iter().zip([135, 122, 14]) {
            assert!((i16::from(actual) - expected).abs() <= 8);
        }
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
        assert_eq!(&data[..8], b"\x89PNG\r\n\x1a\n");
        let chunks = png_chunks(&data);
        assert!(chunks.iter().any(|chunk| *chunk == b"acTL"));
        assert_eq!(chunks.iter().filter(|chunk| **chunk == *b"fcTL").count(), 2);
        assert!(chunks.iter().any(|chunk| *chunk == b"fdAT"));

        let decoder = image::codecs::png::PngDecoder::new(Cursor::new(data.clone())).unwrap();
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

        let dropped = DroppedFile {
            name: "animation.apng".to_string(),
            bytes: Some(data.into()),
            ..Default::default()
        };
        assert_eq!(
            decode_dropped_file(&dropped, ImageFormatCategory::Auto)
                .unwrap()
                .frame_count(),
            2
        );
    }

    #[test]
    fn static_apng_conversion_is_rejected_instead_of_writing_fake_apng() {
        let mut params = ConvertParams::default();
        params.output_format = ImageFormat::APNG;
        let error = convert_image(&semi_transparent_indexed_item(), &params).unwrap_err();
        assert_eq!(error, "APNG output requires an animated export target");
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
