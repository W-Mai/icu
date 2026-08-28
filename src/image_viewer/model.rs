pub use crate::converter::{
    ImageFormats as ImageFormat, LVGL_Version as LvglVersion,
    OutputColorFormats as LvglColorFormat, OutputCompressedMethod as LvglCompression,
};
use clap::ValueEnum;
use eframe::egui::{Color32, DroppedFile, TextureHandle, Vec2};
use icu_lib::endecoder::ImageInfo;
use icu_lib::endecoder::utils::diff::ImageDiffResult;
use icu_lib::midata::MiData;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::time::Duration;
use web_time::Instant;

#[derive(Clone, PartialEq)]
pub struct Frame {
    pub pixels: Vec<Color32>,
    pub width: u32,
    pub height: u32,
    pub left: u32,
    pub top: u32,
    pub delay: Duration,
}

#[derive(Clone, PartialEq)]
pub enum FrameSource {
    Single {
        pixels: Vec<Color32>,
        width: u32,
        height: u32,
    },
    Animated {
        frames: Vec<Frame>,
        current: usize,
        autoplay: bool,
        last_advance: Option<Instant>,
    },
}

impl FrameSource {
    pub fn single(pixels: Vec<Color32>, width: u32, height: u32) -> Self {
        Self::Single {
            pixels,
            width,
            height,
        }
    }

    pub fn animated(frames: Vec<Frame>) -> Self {
        Self::Animated {
            frames,
            current: 0,
            autoplay: true,
            last_advance: None,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ImageItem {
    pub path: String,
    pub info: ImageInfo,
    pub width: u32,
    pub height: u32,
    pub frames: FrameSource,
    pub midata: Option<MiData>,
    pub expanded: bool,
}

type SequenceKey = (String, String, String, String, u32, u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionTarget {
    Entry(WorkspaceId),
    Frame {
        collection: WorkspaceId,
        index: usize,
    },
}

fn sequence_key(path: &str, width: u32, height: u32) -> Option<(SequenceKey, u32)> {
    let path = Path::new(path);
    let stem = path.file_stem()?.to_string_lossy();
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let digit_end = stem.rfind(|c: char| c.is_ascii_digit())? + 1;
    let digit_start = stem[..digit_end]
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |index| index + 1);
    let digits = &stem[digit_start..digit_end];
    let prefix = &stem[..digit_start];
    let suffix = &stem[digit_end..];
    if prefix.is_empty() && suffix.is_empty() && extension.is_empty() {
        return None;
    }
    let number = digits.parse::<u32>().ok()?;
    let parent = path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_string_lossy()
        .into_owned();
    Some((
        (
            parent,
            prefix.to_string(),
            stem[digit_end..].to_string(),
            extension,
            width,
            height,
        ),
        number,
    ))
}

#[derive(Clone, PartialEq)]
struct SequenceMember {
    id: WorkspaceId,
    image: ImageItem,
    sequence_number: Option<u32>,
    digit_width: usize,
}

#[derive(Clone, Copy)]
struct GroupPlayback {
    current_member: Option<WorkspaceId>,
    autoplay: bool,
    expanded: bool,
}

#[derive(Clone)]
struct SequenceGroup {
    label: String,
    members: Vec<SequenceMember>,
    automatic: bool,
}

fn sequence_digits(path: &str) -> Option<(u32, usize)> {
    let stem = Path::new(path).file_stem()?.to_string_lossy();
    let digit_end = stem.rfind(|c: char| c.is_ascii_digit())? + 1;
    let digit_start = stem[..digit_end]
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |index| index + 1);
    let digits = &stem[digit_start..digit_end];
    Some((digits.parse().ok()?, digits.len()))
}

fn sequence_label(members: &[SequenceMember]) -> String {
    let Some(first) = members.first() else {
        return String::new();
    };
    let Some(last) = members.last() else {
        return first.image.path.clone();
    };
    let Some((key, first_number)) =
        sequence_key(&first.image.path, first.image.width, first.image.height)
    else {
        return format!("{} - {}", first.image.path, last.image.path);
    };
    let last_number = last.sequence_number.unwrap_or(first_number);
    let width = members
        .iter()
        .map(|member| member.digit_width)
        .max()
        .unwrap_or(1)
        .max(2);
    let range = format!("{first_number:0width$}-{last_number:0width$}");
    format!("{}{}{}", key.1, range, key.2)
}

fn playback_state(image: &ImageItem, members: &[SequenceMember]) -> GroupPlayback {
    let (current_member, autoplay) = match &image.frames {
        FrameSource::Animated {
            current, autoplay, ..
        } => (members.get(*current).map(|member| member.id), *autoplay),
        FrameSource::Single { .. } => (members.first().map(|member| member.id), true),
    };
    GroupPlayback {
        current_member,
        autoplay,
        expanded: image.expanded,
    }
}

fn sequence_image(
    members: &[SequenceMember],
    label: &str,
    playback: GroupPlayback,
) -> Option<ImageItem> {
    let mut image = members.first()?.image.clone();
    image.width = members.iter().map(|member| member.image.width).max()?;
    image.height = members.iter().map(|member| member.image.height).max()?;
    image.info.width = image.width;
    image.info.height = image.height;
    let frames = members
        .iter()
        .map(|member| {
            let (pixels, width, height) = member.image.current_pixels();
            Frame {
                pixels: pixels.to_vec(),
                width,
                height,
                left: 0,
                top: 0,
                delay: Duration::from_millis(100),
            }
        })
        .collect::<Vec<_>>();
    let current = playback
        .current_member
        .and_then(|id| members.iter().position(|member| member.id == id))
        .unwrap_or(0);
    image.frames = FrameSource::Animated {
        current,
        autoplay: playback.autoplay,
        last_advance: None,
        frames,
    };
    image.expanded = playback.expanded;
    image.midata = None;
    image.path = label.to_string();
    Some(image)
}

impl ImageItem {
    pub fn current_pixels(&self) -> (&[Color32], u32, u32) {
        match &self.frames {
            FrameSource::Single {
                pixels,
                width,
                height,
            } => (pixels.as_slice(), *width, *height),
            FrameSource::Animated {
                frames, current, ..
            } => {
                if let Some(frame) = frames.get(*current).or_else(|| frames.first()) {
                    (frame.pixels.as_slice(), frame.width, frame.height)
                } else {
                    (&[], 0, 0)
                }
            }
        }
    }

    pub fn advance_frame(&mut self) -> bool {
        let FrameSource::Animated {
            frames,
            current,
            autoplay,
            last_advance,
        } = &mut self.frames
        else {
            return false;
        };

        if !*autoplay || frames.len() <= 1 {
            return false;
        }

        let now = Instant::now();
        let last = last_advance.get_or_insert(now);
        let mut remaining = now.saturating_duration_since(*last);
        let mut advanced = false;

        while let Some(frame) = frames.get(*current) {
            let delay = frame.delay.max(Duration::from_millis(1));
            if remaining < delay {
                break;
            }
            remaining -= delay;
            *current = (*current + 1) % frames.len();
            advanced = true;
            if frames.len() <= 1 {
                break;
            }
        }

        if advanced {
            *last_advance = Some(now.checked_sub(remaining).unwrap_or(now));
        }

        advanced
    }

    pub fn set_autoplay(&mut self, autoplay: bool) {
        if let FrameSource::Animated {
            autoplay: current_autoplay,
            last_advance,
            ..
        } = &mut self.frames
        {
            if *current_autoplay != autoplay {
                *current_autoplay = autoplay;
                *last_advance = None;
            }
        }
    }

    pub fn autoplay(&self) -> bool {
        match &self.frames {
            FrameSource::Animated { autoplay, .. } => *autoplay,
            FrameSource::Single { .. } => false,
        }
    }

    pub fn frame_count(&self) -> usize {
        match &self.frames {
            FrameSource::Single { .. } => 1,
            FrameSource::Animated { frames, .. } => frames.len().max(1),
        }
    }

    pub fn total_duration(&self) -> Option<Duration> {
        match &self.frames {
            FrameSource::Single { .. } => None,
            FrameSource::Animated { frames, .. } => Some(
                frames
                    .iter()
                    .fold(Duration::ZERO, |acc, frame| acc.saturating_add(frame.delay)),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkspaceId(u64);

#[derive(Clone, PartialEq)]
pub struct WorkspaceItem {
    id: WorkspaceId,
    content_revision: u64,
    content: SidebarItem,
}

impl WorkspaceItem {
    pub fn id(&self) -> WorkspaceId {
        self.id
    }

    pub fn content(&self) -> &SidebarItem {
        &self.content
    }

    pub fn content_revision(&self) -> u64 {
        self.content_revision
    }
}

#[allow(dead_code)]
#[derive(Clone, PartialEq)]
pub enum SidebarItem {
    Image(ImageItem),
    Glyph(OpenedGlyph),
}

impl SidebarItem {
    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        match self {
            SidebarItem::Image(i) => &i.path,
            SidebarItem::Glyph(g) => &g.name,
        }
    }

    #[allow(dead_code)]
    pub fn as_image(&self) -> Option<&ImageItem> {
        match self {
            SidebarItem::Image(i) => Some(i),
            SidebarItem::Glyph(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphNodeRole {
    Endpoint,
    QuadControl,
    CubicControl1,
    CubicControl2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphNodeId {
    pub command_index: usize,
    pub role: GlyphNodeRole,
}

pub const GLYPH_HISTORY_LIMIT: usize = 64;

#[derive(Clone, Default, PartialEq)]
pub struct GlyphEditorState {
    pub selected_node: Option<GlyphNodeId>,
    undo: Vec<Vec<icu_lib::mirx::PathCmd>>,
    redo: Vec<Vec<icu_lib::mirx::PathCmd>>,
    pub drag_before: Option<Vec<icu_lib::mirx::PathCmd>>,
}

impl GlyphEditorState {
    pub fn record(&mut self, before: Vec<icu_lib::mirx::PathCmd>) {
        if self.undo.last() != Some(&before) {
            self.undo.push(before);
            if self.undo.len() > GLYPH_HISTORY_LIMIT {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
    }

    pub fn undo(
        &mut self,
        current: &[icu_lib::mirx::PathCmd],
    ) -> Option<Vec<icu_lib::mirx::PathCmd>> {
        let previous = self.undo.pop()?;
        self.redo.push(current.to_vec());
        Some(previous)
    }

    pub fn redo(
        &mut self,
        current: &[icu_lib::mirx::PathCmd],
    ) -> Option<Vec<icu_lib::mirx::PathCmd>> {
        let next = self.redo.pop()?;
        self.undo.push(current.to_vec());
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

pub fn glyph_nodes(outline: &[icu_lib::mirx::PathCmd]) -> Vec<(GlyphNodeId, icu_lib::mirx::Point)> {
    let mut nodes = Vec::new();
    for (command_index, command) in outline.iter().enumerate() {
        let endpoint = GlyphNodeId {
            command_index,
            role: GlyphNodeRole::Endpoint,
        };
        match command {
            icu_lib::mirx::PathCmd::MoveTo(point) | icu_lib::mirx::PathCmd::LineTo(point) => {
                nodes.push((endpoint, *point));
            }
            icu_lib::mirx::PathCmd::QuadTo { ctrl, end } => {
                nodes.push((
                    GlyphNodeId {
                        command_index,
                        role: GlyphNodeRole::QuadControl,
                    },
                    *ctrl,
                ));
                nodes.push((endpoint, *end));
            }
            icu_lib::mirx::PathCmd::CubicTo { ctrl1, ctrl2, end } => {
                nodes.push((
                    GlyphNodeId {
                        command_index,
                        role: GlyphNodeRole::CubicControl1,
                    },
                    *ctrl1,
                ));
                nodes.push((
                    GlyphNodeId {
                        command_index,
                        role: GlyphNodeRole::CubicControl2,
                    },
                    *ctrl2,
                ));
                nodes.push((endpoint, *end));
            }
            icu_lib::mirx::PathCmd::Close => {}
        }
    }
    nodes
}

pub fn glyph_node_point(
    outline: &[icu_lib::mirx::PathCmd],
    node: GlyphNodeId,
) -> Option<icu_lib::mirx::Point> {
    match outline.get(node.command_index)? {
        icu_lib::mirx::PathCmd::MoveTo(point) | icu_lib::mirx::PathCmd::LineTo(point)
            if node.role == GlyphNodeRole::Endpoint =>
        {
            Some(*point)
        }
        icu_lib::mirx::PathCmd::QuadTo { ctrl, .. } if node.role == GlyphNodeRole::QuadControl => {
            Some(*ctrl)
        }
        icu_lib::mirx::PathCmd::QuadTo { end, .. } if node.role == GlyphNodeRole::Endpoint => {
            Some(*end)
        }
        icu_lib::mirx::PathCmd::CubicTo { ctrl1, .. }
            if node.role == GlyphNodeRole::CubicControl1 =>
        {
            Some(*ctrl1)
        }
        icu_lib::mirx::PathCmd::CubicTo { ctrl2, .. }
            if node.role == GlyphNodeRole::CubicControl2 =>
        {
            Some(*ctrl2)
        }
        icu_lib::mirx::PathCmd::CubicTo { end, .. } if node.role == GlyphNodeRole::Endpoint => {
            Some(*end)
        }
        _ => None,
    }
}

pub fn move_glyph_node(
    outline: &mut [icu_lib::mirx::PathCmd],
    node: GlyphNodeId,
    point: icu_lib::mirx::Point,
) -> bool {
    let Some(command) = outline.get_mut(node.command_index) else {
        return false;
    };
    match (command, node.role) {
        (icu_lib::mirx::PathCmd::MoveTo(target), GlyphNodeRole::Endpoint)
        | (icu_lib::mirx::PathCmd::LineTo(target), GlyphNodeRole::Endpoint) => *target = point,
        (icu_lib::mirx::PathCmd::QuadTo { ctrl, .. }, GlyphNodeRole::QuadControl) => *ctrl = point,
        (icu_lib::mirx::PathCmd::QuadTo { end, .. }, GlyphNodeRole::Endpoint) => *end = point,
        (icu_lib::mirx::PathCmd::CubicTo { ctrl1, .. }, GlyphNodeRole::CubicControl1) => {
            *ctrl1 = point
        }
        (icu_lib::mirx::PathCmd::CubicTo { ctrl2, .. }, GlyphNodeRole::CubicControl2) => {
            *ctrl2 = point
        }
        (icu_lib::mirx::PathCmd::CubicTo { end, .. }, GlyphNodeRole::Endpoint) => *end = point,
        _ => return false,
    }
    true
}

pub fn delete_glyph_node(outline: &mut Vec<icu_lib::mirx::PathCmd>, node: GlyphNodeId) -> bool {
    let Some(command) = outline.get(node.command_index) else {
        return false;
    };
    match (command, node.role) {
        (icu_lib::mirx::PathCmd::LineTo(_), GlyphNodeRole::Endpoint) => {
            outline.remove(node.command_index);
            true
        }
        (icu_lib::mirx::PathCmd::QuadTo { end, .. }, GlyphNodeRole::QuadControl) => {
            let end = *end;
            outline[node.command_index] = icu_lib::mirx::PathCmd::LineTo(end);
            true
        }
        (icu_lib::mirx::PathCmd::CubicTo { end, .. }, GlyphNodeRole::CubicControl1)
        | (icu_lib::mirx::PathCmd::CubicTo { end, .. }, GlyphNodeRole::CubicControl2) => {
            let end = *end;
            outline[node.command_index] = icu_lib::mirx::PathCmd::LineTo(end);
            true
        }
        _ => false,
    }
}

pub fn can_delete_glyph_node(outline: &[icu_lib::mirx::PathCmd], node: GlyphNodeId) -> bool {
    matches!(
        (outline.get(node.command_index), node.role),
        (
            Some(icu_lib::mirx::PathCmd::LineTo(_)),
            GlyphNodeRole::Endpoint
        ) | (
            Some(icu_lib::mirx::PathCmd::QuadTo { .. }),
            GlyphNodeRole::QuadControl
        ) | (
            Some(icu_lib::mirx::PathCmd::CubicTo { .. }),
            GlyphNodeRole::CubicControl1
        ) | (
            Some(icu_lib::mirx::PathCmd::CubicTo { .. }),
            GlyphNodeRole::CubicControl2
        )
    )
}

pub fn can_add_glyph_node(outline: &[icu_lib::mirx::PathCmd], node: GlyphNodeId) -> bool {
    node.role == GlyphNodeRole::Endpoint
        && glyph_node_point(outline, node).is_some()
        && outline
            .get(node.command_index + 1)
            .is_some_and(|command| !matches!(command, icu_lib::mirx::PathCmd::Close))
}

pub fn add_glyph_node(outline: &mut Vec<icu_lib::mirx::PathCmd>, node: GlyphNodeId) -> bool {
    if node.role != GlyphNodeRole::Endpoint {
        return false;
    }
    let Some(current) = glyph_node_point(outline, node) else {
        return false;
    };
    let Some(next) = outline
        .get(node.command_index + 1)
        .and_then(|command| match command {
            icu_lib::mirx::PathCmd::LineTo(point) | icu_lib::mirx::PathCmd::MoveTo(point) => {
                Some(*point)
            }
            icu_lib::mirx::PathCmd::QuadTo { end, .. }
            | icu_lib::mirx::PathCmd::CubicTo { end, .. } => Some(*end),
            icu_lib::mirx::PathCmd::Close => None,
        })
    else {
        return false;
    };
    let midpoint = icu_lib::mirx::Point::new(
        icu_lib::mirx::Fixed::from_raw((current.x.raw() + next.x.raw()) / 2),
        icu_lib::mirx::Fixed::from_raw((current.y.raw() + next.y.raw()) / 2),
    );
    outline.insert(
        node.command_index + 1,
        icu_lib::mirx::PathCmd::LineTo(midpoint),
    );
    true
}

#[allow(dead_code)]
#[derive(Clone, PartialEq)]
pub struct OpenedGlyph {
    pub name: String,
    pub codepoint: u32,
    pub char_repr: String,
    pub advance: u16,
    pub bearing: (i16, i16),
    pub bbox: (i16, i16, i16, i16),
    pub outline: Vec<icu_lib::mirx::PathCmd>,
    pub outline_approximate: bool,
    pub source_font: String,
    pub source_is_sdf: bool,
    pub editor: GlyphEditorState,
}

pub struct GlyphDiffResult {
    pub codepoint: u32,
    pub char_repr: String,
    pub img_a: icu_lib::image::RgbaImage,
    pub img_b: icu_lib::image::RgbaImage,
    pub diff: ImageDiffResult,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RightTab {
    Info,
    Convert,
    Diff,
}

impl Default for RightTab {
    fn default() -> Self {
        RightTab::Info
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum FontMode {
    Atlas,
    Rendered,
    Grid,
    Vector,
}

impl Default for FontMode {
    fn default() -> Self {
        FontMode::Grid
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasViewCommand {
    Fit,
    ActualSize,
}

#[derive(Clone, Copy)]
pub struct GlyphCanvasView {
    pub zoom: f32,
    pub pan: Vec2,
    pub pending: Option<CanvasViewCommand>,
}

pub struct GlyphTextureCache {
    pub map: std::collections::HashMap<usize, TextureHandle>,
    pub key: String,
}

impl Default for GlyphCanvasView {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            pending: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum IndexedViewMode {
    RGBA,
    IndexMap,
}

impl Default for IndexedViewMode {
    fn default() -> Self {
        IndexedViewMode::RGBA
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BakeCharsetTab {
    Text,
    Range,
    File,
}

impl Default for BakeCharsetTab {
    fn default() -> Self {
        BakeCharsetTab::Text
    }
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PathMode {
    Preview,
}

impl Default for PathMode {
    fn default() -> Self {
        PathMode::Preview
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppContext {
    pub show_grid: bool,
    pub anti_alias: bool,
    pub diff_active: bool,
    pub right_tab: RightTab,
    pub background_color: Color32,
    pub diff_blend: f32,
    pub diff_tolerance: f32,
    pub min_diff: f32,
    pub max_diff: f32,

    pub fast_switch: bool,
    pub fast_switch_speed: f32,
    pub fast_switch_phase: f32,
    pub only_show_diff: bool,
    pub language: String,

    pub diff_sorting: DiffSorting,
    pub diff_page_index: usize,
    pub diff_page_size: usize,

    pub convert_params: ConvertParams,
    #[serde(default = "default_mirx_export_kind")]
    pub mirx_export_kind: String,
}

#[derive(Serialize, Deserialize, PartialEq, Copy, Clone, Debug, ValueEnum)]
pub enum DiffSorting {
    Z,
    N,
    ReverseZ,
    ReverseN,
    DiffAsc,
    DiffDesc,
}

#[derive(Serialize, Deserialize, PartialEq, Copy, Clone, Debug, ValueEnum, Default)]
pub enum PngColorMode {
    #[default]
    Rgba,
    Rgb,
    Preserve,
    Indexed1,
    Indexed2,
    Indexed4,
    Indexed8,
}

#[derive(Serialize, Deserialize, PartialEq, Copy, Clone, Debug, ValueEnum, Default)]
pub enum PngCompression {
    Fast,
    #[default]
    Balanced,
    Best,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConvertParams {
    pub output_format: ImageFormat,
    pub lvgl_version: LvglVersion,
    pub color_format: LvglColorFormat,
    pub compression: LvglCompression,
    pub stride_align: u8,
    pub dither: bool,
    pub dither_level: u32,
    #[serde(default)]
    pub png_color_mode: PngColorMode,
    #[serde(default)]
    pub png_compression: PngCompression,
    #[serde(default = "default_jpeg_quality")]
    pub jpeg_quality: u8,
    #[serde(default = "default_jpeg_background")]
    pub jpeg_background: [u8; 3],
    #[serde(default = "default_gif_interval_ms")]
    pub gif_interval_ms: u32,
    #[serde(default)]
    pub gif_repeat: Option<u16>,
}

impl Default for ConvertParams {
    fn default() -> Self {
        Self {
            output_format: ImageFormat::LVGL,
            lvgl_version: LvglVersion::V9,
            color_format: LvglColorFormat::RGB565,
            compression: LvglCompression::None,
            stride_align: 1,
            dither: false,
            dither_level: 10,
            png_color_mode: PngColorMode::default(),
            png_compression: PngCompression::default(),
            jpeg_quality: default_jpeg_quality(),
            jpeg_background: default_jpeg_background(),
            gif_interval_ms: 100,
            gif_repeat: None,
        }
    }
}

fn default_jpeg_quality() -> u8 {
    85
}

fn default_jpeg_background() -> [u8; 3] {
    [255, 255, 255]
}

impl Default for AppContext {
    fn default() -> Self {
        Self {
            show_grid: true,
            anti_alias: true,
            diff_active: false,
            right_tab: RightTab::default(),
            background_color: Default::default(),
            diff_blend: 0.5,
            diff_tolerance: 0.1,
            min_diff: 0.0,
            max_diff: f32::MAX,
            fast_switch: false,
            fast_switch_speed: 1.0,
            fast_switch_phase: 0.0,
            only_show_diff: false,
            language: crate::image_viewer::utils::get_system_locale(),
            diff_sorting: DiffSorting::Z,
            diff_page_index: 0,
            diff_page_size: 100,
            convert_params: ConvertParams::default(),
            mirx_export_kind: "scene".to_string(),
        }
    }
}

fn default_mirx_export_kind() -> String {
    "scene".to_string()
}

fn default_gif_interval_ms() -> u32 {
    100
}

#[allow(dead_code)]
pub struct ViewerState {
    items: Vec<WorkspaceItem>,
    next_workspace_id: u64,
    sequence_groups: HashMap<WorkspaceId, SequenceGroup>,
    pub selected_id: Option<WorkspaceId>,
    pub primary_target: Option<SelectionTarget>,
    pub renaming_group: Option<WorkspaceId>,
    pub rename_buffer: String,
    pub selected_ids: BTreeSet<WorkspaceId>,
    pub list_focus: bool,
    pub focused_id: Option<WorkspaceId>,
    range_anchor: Option<WorkspaceId>,
    pub hovered_id: Option<WorkspaceId>,
    pub dropped_files: Vec<DroppedFile>,
    pub input_format: crate::converter::ImageFormatCategory,
    pub context: AppContext,
    pub diff_image1_id: Option<WorkspaceId>,
    pub diff_image2_id: Option<WorkspaceId>,
    pub diff_result: Option<(ImageItem, ImageDiffResult)>,
    pub selected_diff_pixel: Option<[u32; 2]>,
    pub hovered_diff_pixel: Option<[u32; 2]>,
    pub hovered_diff_pixel_from_plot: Option<[u32; 2]>,
    pub is_converting: bool,
    pub font_preview_text: String,
    pub font_rendered_preview: Option<icu_lib::image::RgbaImage>,
    pub selected_op: Option<usize>,
    pub path_mode: PathMode,
    pub indexed_hover_palette: Option<u8>,
    pub indexed_edit_palette: Option<usize>,
    pub indexed_edit_color: Color32,
    pub indexed_show_quality: bool,
    pub indexed_view_mode: IndexedViewMode,
    pub font_bake_size: u16,
    pub font_bake_format: String,
    pub font_bake_bit_depth: u8,
    pub font_bake_charset_tab: BakeCharsetTab,
    pub font_bake_charset_text: String,
    pub font_bake_charset_ranges: String,
    pub font_bake_charset_file: Option<String>,
    pub indexed_dither: u32,
    pub indexed_dither_cached: u32,
    pub indexed_dither_cached_id: Option<WorkspaceId>,
    pub indexed_dither_cached_revision: u64,
    pub indexed_requantized: Option<icu_lib::midata::IndexedImageData>,
    pub merge_font_paths: Vec<String>,
    pub font_mode: FontMode,
    pub glyph_diff_char: String,
    pub selected_glyph: Option<usize>,
    pub opened_glyphs: Vec<OpenedGlyph>,
    pub glyph_convert_format: String,
    pub path_export_format: String,
    pub font_atlas_cached: Option<(String, String, Vec<Color32>, u32, u32)>,
    pub font_grid_cached: Option<GlyphTextureCache>,
    pub font_grid_big_cached: Option<(String, TextureHandle)>,
    pub font_bundle_index: usize,
    pub glyph_canvas_view: GlyphCanvasView,
    pub render_canvas_view: GlyphCanvasView,
    #[cfg(target_arch = "wasm32")]
    pub pending_dropped: std::rc::Rc<std::cell::RefCell<Vec<DroppedFile>>>,
}

impl ViewerState {
    fn allocate_id(&mut self) -> WorkspaceId {
        let id = WorkspaceId(self.next_workspace_id);
        self.next_workspace_id = self
            .next_workspace_id
            .checked_add(1)
            .expect("workspace id overflow");
        id
    }

    pub fn items(&self) -> &[WorkspaceItem] {
        &self.items
    }

    pub fn items_snapshot(&self) -> Vec<WorkspaceItem> {
        self.items.clone()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn content_at_mut(&mut self, index: usize) -> Option<&mut SidebarItem> {
        self.items.get_mut(index).map(|item| &mut item.content)
    }

    pub fn index_of(&self, id: WorkspaceId) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }

    pub fn item(&self, id: WorkspaceId) -> Option<&SidebarItem> {
        self.items
            .iter()
            .find(|item| item.id == id)
            .map(|item| &item.content)
    }

    pub fn item_mut(&mut self, id: WorkspaceId) -> Option<&mut SidebarItem> {
        self.items
            .iter_mut()
            .find(|item| item.id == id)
            .map(|item| &mut item.content)
    }

    pub fn selected_item(&self) -> Option<&SidebarItem> {
        self.item(self.selected_id?)
    }

    pub fn selected_item_mut(&mut self) -> Option<&mut SidebarItem> {
        self.item_mut(self.selected_id?)
    }

    pub fn current_image(&self) -> Option<&ImageItem> {
        self.selected_item().and_then(SidebarItem::as_image)
    }

    pub fn current_image_mut(&mut self) -> Option<&mut ImageItem> {
        match self.selected_item_mut()? {
            SidebarItem::Image(image) => Some(image),
            SidebarItem::Glyph(_) => None,
        }
    }

    pub fn select(&mut self, id: WorkspaceId) -> bool {
        if self.index_of(id).is_none() {
            return false;
        }
        let changed = self.selected_id != Some(id);
        self.selected_id = Some(id);
        self.primary_target = Some(SelectionTarget::Entry(id));
        self.selected_ids.clear();
        self.selected_ids.insert(id);
        self.focused_id = Some(id);
        self.range_anchor = Some(id);
        if changed {
            self.invalidate_selection_state();
        }
        true
    }

    pub fn focus_list(&mut self, id: WorkspaceId) -> bool {
        if !self.select(id) {
            return false;
        }
        self.list_focus = true;
        true
    }

    pub fn blur_list(&mut self) {
        self.list_focus = false;
    }

    pub fn select_all(&mut self) {
        self.selected_ids = self
            .items
            .iter()
            .filter(|item| matches!(item.content, SidebarItem::Image(_)))
            .map(WorkspaceItem::id)
            .collect();
        if self.selected_id.is_none() || !self.selected_ids.contains(&self.selected_id.unwrap()) {
            self.selected_id = self.selected_ids.iter().next().copied();
        }
        if let Some(id) = self.selected_id {
            self.focused_id = Some(id);
            self.primary_target = Some(SelectionTarget::Entry(id));
        }
    }

    pub fn select_frame(&mut self, collection: WorkspaceId, index: usize) -> bool {
        if let Some(group) = self.sequence_groups.get(&collection)
            && index >= group.members.len()
        {
            return false;
        }
        let Some(SidebarItem::Image(image)) = self.item(collection) else {
            return false;
        };
        if !matches!(image.frames, FrameSource::Animated { .. }) {
            return false;
        }
        if !self.select(collection) {
            return false;
        }
        if let Some(SidebarItem::Image(image)) = self.item_mut(collection)
            && let FrameSource::Animated {
                frames,
                current,
                last_advance,
                ..
            } = &mut image.frames
        {
            if index >= frames.len() {
                return false;
            }
            *current = index;
            *last_advance = None;
        }
        self.primary_target = Some(SelectionTarget::Frame { collection, index });
        true
    }

    pub fn set_animation_interval(&mut self, id: WorkspaceId, interval: Duration) -> bool {
        let Some(SidebarItem::Image(image)) = self.item_mut(id) else {
            return false;
        };

        let FrameSource::Animated {
            frames,
            last_advance,
            ..
        } = &mut image.frames
        else {
            return false;
        };
        for frame in frames {
            frame.delay = interval;
        }
        *last_advance = None;
        true
    }

    pub fn group_label(&self, id: WorkspaceId) -> Option<&str> {
        self.sequence_groups
            .get(&id)
            .map(|group| group.label.as_str())
    }

    pub fn set_group_label(&mut self, id: WorkspaceId, label: String) -> bool {
        let label = label.trim();
        if label.is_empty() {
            return false;
        }
        let Some(group) = self.sequence_groups.get_mut(&id) else {
            return false;
        };
        group.label = label.to_string();
        if let Some(SidebarItem::Image(image)) = self
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .map(|item| &mut item.content)
        {
            image.path = label.to_string();
        }
        true
    }

    pub fn group_members(&self, id: WorkspaceId) -> Option<Vec<(WorkspaceId, String, ImageItem)>> {
        self.sequence_groups.get(&id).map(|group| {
            group
                .members
                .iter()
                .map(|member| (member.id, member.image.path.clone(), member.image.clone()))
                .collect()
        })
    }

    pub fn frame_snapshots(&self, id: WorkspaceId) -> Option<Vec<(String, ImageItem)>> {
        if let Some(members) = self.group_members(id) {
            return Some(
                members
                    .into_iter()
                    .map(|(_, name, image)| (name, image))
                    .collect(),
            );
        }
        let SidebarItem::Image(image) = self.item(id)? else {
            return None;
        };
        let FrameSource::Animated { frames, .. } = &image.frames else {
            return None;
        };
        Some(
            frames
                .iter()
                .enumerate()
                .map(|(index, frame)| {
                    (
                        format!("{}#{}", image.path, index + 1),
                        ImageItem {
                            path: format!("{}#{}", image.path, index + 1),
                            info: image.info.clone(),
                            width: frame.width,
                            height: frame.height,
                            frames: FrameSource::single(
                                frame.pixels.clone(),
                                frame.width,
                                frame.height,
                            ),
                            midata: None,
                            expanded: false,
                        },
                    )
                })
                .collect(),
        )
    }

    pub fn toggle_selection(&mut self, id: WorkspaceId) -> bool {
        if self.index_of(id).is_none() {
            return false;
        }
        self.list_focus = true;
        self.focused_id = Some(id);
        if !self.selected_ids.remove(&id) {
            self.selected_ids.insert(id);
        }
        if self.selected_ids.is_empty() {
            self.select(id);
        } else {
            self.selected_id = Some(id);
            self.primary_target = Some(SelectionTarget::Entry(id));
        }
        true
    }

    pub fn extend_selection(&mut self, id: WorkspaceId) -> bool {
        let Some(anchor) = self.range_anchor.or(self.selected_id) else {
            return self.select(id);
        };
        let Some(a) = self.index_of(anchor) else {
            return self.select(id);
        };
        let Some(b) = self.index_of(id) else {
            return false;
        };
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        self.selected_ids = self.items[start..=end]
            .iter()
            .map(WorkspaceItem::id)
            .collect();
        self.selected_id = Some(id);
        self.primary_target = Some(SelectionTarget::Entry(id));
        self.focused_id = Some(id);
        self.list_focus = true;
        true
    }

    pub fn move_selection(&mut self, delta: isize) -> bool {
        if !self.list_focus || self.items.is_empty() {
            return false;
        }
        let current = self
            .focused_id
            .or(self.selected_id)
            .and_then(|id| self.index_of(id))
            .unwrap_or(0);
        let next = (current as isize + delta).clamp(0, self.items.len() as isize - 1) as usize;
        if next == current {
            return false;
        }
        let id = self.items[next].id;
        self.select(id)
    }

    pub fn edit_indexed_palette_color(&mut self, index: usize, color: Color32) -> bool {
        let Some(selected_id) = self.selected_id else {
            return false;
        };
        let Some(image) = self.current_image_mut() else {
            return false;
        };
        if !matches!(image.frames, FrameSource::Single { .. }) {
            return false;
        }
        let Some(MiData::INDEXED(indexed)) = image.midata.as_mut() else {
            return false;
        };
        if !indexed.set_palette_color(index, color.to_srgba_unmultiplied()) {
            return false;
        }
        let pixels = indexed
            .rgba
            .chunks(4)
            .map(|pixel| Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]))
            .collect::<Vec<_>>();
        image.frames = FrameSource::single(pixels, indexed.width, indexed.height);
        if let Some(item) = self.items.iter_mut().find(|item| item.id == selected_id) {
            item.content_revision = item.content_revision.saturating_add(1);
        }
        self.invalidate_derived_state();
        true
    }

    pub fn remove_selected(&mut self) {
        let ids = self.selected_ids.iter().copied().collect::<Vec<_>>();
        for id in ids {
            self.remove_id(id);
        }
        let valid_ids = self
            .items
            .iter()
            .map(WorkspaceItem::id)
            .collect::<BTreeSet<_>>();
        self.selected_ids.retain(|id| valid_ids.contains(id));
        if let Some(id) = self.selected_id {
            if !valid_ids.contains(&id) {
                self.selected_id = self.items.first().map(WorkspaceItem::id);
            }
        }
    }

    fn insert_items(
        &mut self,
        index: usize,
        items: impl IntoIterator<Item = SidebarItem>,
    ) -> Vec<WorkspaceId> {
        let mut inserted = Vec::new();
        let mut index = index.min(self.items.len());
        for content in items {
            let id = self.allocate_id();
            self.items.insert(
                index,
                WorkspaceItem {
                    id,
                    content_revision: 0,
                    content,
                },
            );
            inserted.push(id);
            index += 1;
        }
        inserted
    }

    fn append_items(&mut self, items: impl IntoIterator<Item = SidebarItem>) -> Vec<WorkspaceId> {
        self.insert_items(self.items.len(), items)
    }

    pub fn insert_glyph_after_selected(&mut self, glyph: OpenedGlyph) -> WorkspaceId {
        let index = self
            .selected_id
            .and_then(|id| self.index_of(id))
            .map_or(self.items.len(), |index| index + 1);
        let id = self.insert_items(index, [SidebarItem::Glyph(glyph)])[0];
        self.select(id);
        id
    }

    pub fn insert_and_select_first(
        &mut self,
        items: impl IntoIterator<Item = SidebarItem>,
    ) -> Vec<WorkspaceId> {
        let playback = self.expand_automatic_sequence_groups();
        let mut seen_paths = self
            .items
            .iter()
            .filter_map(|item| item.content.as_image().map(|image| image.path.clone()))
            .collect::<std::collections::HashSet<_>>();
        let ids = self.append_items(items.into_iter().filter(|item| {
            item.as_image()
                .is_none_or(|image| seen_paths.insert(image.path.clone()))
        }));
        self.auto_group_sequences(&playback);
        if !ids.is_empty() {
            let selected = self
                .items
                .iter()
                .find(|item| ids.contains(&item.id))
                .map(WorkspaceItem::id)
                .or_else(|| {
                    self.sequence_groups.iter().find_map(|(group_id, group)| {
                        group
                            .members
                            .iter()
                            .any(|member| ids.contains(&member.id))
                            .then_some(*group_id)
                    })
                });
            if let Some(selected) = selected {
                self.select(selected);
            }
        }
        ids
    }

    fn expand_automatic_sequence_groups(&mut self) -> HashMap<SequenceKey, GroupPlayback> {
        let group_ids = self
            .sequence_groups
            .iter()
            .filter_map(|(id, group)| group.automatic.then_some(*id))
            .collect::<Vec<_>>();
        let mut playback = HashMap::new();
        for group_id in group_ids {
            let Some(group) = self.sequence_groups.remove(&group_id) else {
                continue;
            };
            let Some(index) = self.index_of(group_id) else {
                continue;
            };
            let current_member = if let SidebarItem::Image(image) = &self.items[index].content {
                let state = playback_state(image, &group.members);
                if let Some(first) = group.members.first()
                    && let Some((key, _)) =
                        sequence_key(&first.image.path, first.image.width, first.image.height)
                {
                    playback.insert(key, state);
                }
                state.current_member
            } else {
                None
            };
            if let Some(member_id) = current_member {
                self.remap_expanded_group_references(group_id, member_id);
            }
            self.items.remove(index);
            for (offset, member) in group.members.into_iter().enumerate() {
                self.items.insert(
                    index + offset,
                    WorkspaceItem {
                        id: member.id,
                        content_revision: 0,
                        content: SidebarItem::Image(member.image),
                    },
                );
            }
        }
        playback
    }

    pub fn is_sequence_group(&self, id: WorkspaceId) -> bool {
        self.sequence_groups.contains_key(&id)
    }

    fn remap_expanded_group_references(&mut self, group_id: WorkspaceId, member_id: WorkspaceId) {
        if self.selected_ids.remove(&group_id) {
            self.selected_ids.insert(member_id);
        }
        for reference in [
            &mut self.selected_id,
            &mut self.focused_id,
            &mut self.range_anchor,
            &mut self.hovered_id,
            &mut self.diff_image1_id,
            &mut self.diff_image2_id,
        ] {
            if *reference == Some(group_id) {
                *reference = Some(member_id);
            }
        }
        if self.primary_target.is_some_and(|target| match target {
            SelectionTarget::Entry(id) => id == group_id,
            SelectionTarget::Frame { collection, .. } => collection == group_id,
        }) {
            self.primary_target = Some(SelectionTarget::Entry(member_id));
        }
    }

    fn remap_group_references(&mut self, member_ids: &[WorkspaceId], group_id: WorkspaceId) {
        let mut was_selected = false;
        for id in member_ids {
            was_selected |= self.selected_ids.remove(id);
        }
        if was_selected {
            self.selected_ids.insert(group_id);
        }
        if self.focused_id.is_some_and(|id| member_ids.contains(&id)) {
            self.focused_id = Some(group_id);
        }
        if self.range_anchor.is_some_and(|id| member_ids.contains(&id)) {
            self.range_anchor = Some(group_id);
        }
        if self.selected_id.is_some_and(|id| member_ids.contains(&id)) {
            self.selected_id = Some(group_id);
        }
        if self.hovered_id.is_some_and(|id| member_ids.contains(&id)) {
            self.hovered_id = Some(group_id);
        }
        if self
            .diff_image1_id
            .is_some_and(|id| member_ids.contains(&id))
        {
            self.diff_image1_id = Some(group_id);
        }
        if self
            .diff_image2_id
            .is_some_and(|id| member_ids.contains(&id))
        {
            self.diff_image2_id = Some(group_id);
        }
    }

    pub fn group_selected(&mut self) -> Option<WorkspaceId> {
        let ids = self.selected_ids.iter().copied().collect::<Vec<_>>();
        self.group_images(&ids)
    }

    pub fn ungroup_selected(&mut self) -> usize {
        let group_ids = self
            .items
            .iter()
            .filter_map(|item| {
                (self.selected_ids.contains(&item.id)
                    && self.sequence_groups.contains_key(&item.id))
                .then_some(item.id)
            })
            .collect::<Vec<_>>();
        let mut restored = Vec::new();
        for group_id in &group_ids {
            if let Some(member_ids) = self.ungroup_members(*group_id) {
                restored.extend(member_ids);
            }
        }
        if let Some(primary) = restored.first().copied() {
            self.selected_ids = restored.iter().copied().collect();
            self.selected_id = Some(primary);
            self.primary_target = Some(SelectionTarget::Entry(primary));
            self.focused_id = Some(primary);
            self.range_anchor = Some(primary);
            self.invalidate_selection_state();
        }
        group_ids.len()
    }

    pub fn group_images(&mut self, ids: &[WorkspaceId]) -> Option<WorkspaceId> {
        let unique = ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        if unique.len() != ids.len() {
            return None;
        }
        let mut members = ids
            .iter()
            .filter_map(|id| {
                let index = self.index_of(*id)?;
                let SidebarItem::Image(image) = &self.items[index].content else {
                    return None;
                };
                (image.frame_count() == 1).then_some((
                    index,
                    SequenceMember {
                        id: *id,
                        image: image.clone(),
                        sequence_number: sequence_digits(&image.path).map(|value| value.0),
                        digit_width: sequence_digits(&image.path).map_or(0, |value| value.1),
                    },
                ))
            })
            .collect::<Vec<_>>();
        members.sort_by_key(|(index, _)| *index);
        if members.len() < 2 || members.len() != ids.len() {
            return None;
        }
        let width = members[0].1.image.width;
        let height = members[0].1.image.height;
        if members
            .iter()
            .any(|(_, member)| member.image.width != width || member.image.height != height)
        {
            return None;
        }
        let first_index = members[0].0;
        let originals = members
            .iter()
            .map(|(_, member)| member.clone())
            .collect::<Vec<_>>();
        let label = sequence_label(&originals);
        let group_image = sequence_image(
            &originals,
            &label,
            GroupPlayback {
                current_member: originals.first().map(|member| member.id),
                autoplay: true,
                expanded: false,
            },
        )?;
        let member_ids = originals.iter().map(|member| member.id).collect::<Vec<_>>();
        let group_id = self.allocate_id();
        self.remap_group_references(&member_ids, group_id);
        self.sequence_groups.insert(
            group_id,
            SequenceGroup {
                label,
                members: originals,
                automatic: false,
            },
        );
        self.items[first_index] = WorkspaceItem {
            id: group_id,
            content_revision: 0,
            content: SidebarItem::Image(group_image),
        };
        for index in members.iter().skip(1).map(|(index, _)| *index).rev() {
            self.items.remove(index);
        }
        self.selected_id = Some(group_id);
        self.primary_target = Some(SelectionTarget::Entry(group_id));
        self.invalidate_selection_state();
        Some(group_id)
    }

    fn auto_group_sequences(&mut self, playback: &HashMap<SequenceKey, GroupPlayback>) {
        let mut candidates: HashMap<SequenceKey, Vec<(usize, WorkspaceId, u32)>> = HashMap::new();
        for (index, item) in self.items.iter().enumerate() {
            let SidebarItem::Image(image) = &item.content else {
                continue;
            };
            if image.frame_count() != 1
                || !matches!(
                    image.midata,
                    None | Some(MiData::RGBA(_)) | Some(MiData::INDEXED(_))
                )
            {
                continue;
            }
            let Some((key, number)) = sequence_key(&image.path, image.width, image.height) else {
                continue;
            };
            candidates
                .entry(key)
                .or_default()
                .push((index, item.id, number));
        }

        let mut groups = candidates
            .into_iter()
            .filter_map(|(key, mut members)| {
                members.sort_by_key(|(_, _, number)| *number);
                (members.len() >= 2 && members.windows(2).all(|pair| pair[1].2 == pair[0].2 + 1))
                    .then_some((key, members))
            })
            .collect::<Vec<_>>();
        groups.sort_by_key(|(_, members)| members[0].0);

        for (key, members) in groups.into_iter().rev() {
            let member_ids = members.iter().map(|(_, id, _)| *id).collect::<Vec<_>>();
            if member_ids
                .iter()
                .any(|id| self.sequence_groups.contains_key(id))
            {
                continue;
            }
            let Some(first_index) = self.index_of(member_ids[0]) else {
                continue;
            };
            let originals = member_ids
                .iter()
                .filter_map(|id| match self.item(*id) {
                    Some(SidebarItem::Image(image)) => Some(SequenceMember {
                        id: *id,
                        image: image.clone(),
                        sequence_number: sequence_digits(&image.path).map(|value| value.0),
                        digit_width: sequence_digits(&image.path).map_or(0, |value| value.1),
                    }),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if originals.len() != member_ids.len() {
                continue;
            }
            let state = playback.get(&key).copied().unwrap_or(GroupPlayback {
                current_member: originals.first().map(|member| member.id),
                autoplay: true,
                expanded: false,
            });
            let label = sequence_label(&originals);
            let Some(group_image) = sequence_image(&originals, &label, state) else {
                continue;
            };
            let member_ids = originals.iter().map(|member| member.id).collect::<Vec<_>>();
            let group_id = self.allocate_id();
            self.remap_group_references(&member_ids, group_id);
            self.sequence_groups.insert(
                group_id,
                SequenceGroup {
                    label,
                    members: originals,
                    automatic: true,
                },
            );
            self.items[first_index] = WorkspaceItem {
                id: group_id,
                content_revision: 0,
                content: SidebarItem::Image(group_image),
            };
            let mut removal_indices = member_ids
                .iter()
                .skip(1)
                .filter_map(|id| self.index_of(*id))
                .collect::<Vec<_>>();
            removal_indices.sort_unstable_by(|left, right| right.cmp(left));
            for index in removal_indices {
                self.items.remove(index);
            }
        }
    }

    fn ungroup_members(&mut self, group_id: WorkspaceId) -> Option<Vec<WorkspaceId>> {
        let members = self.sequence_groups.remove(&group_id)?;
        let index = self.index_of(group_id)?;
        self.items.remove(index);
        let member_ids = members
            .members
            .iter()
            .map(|member| member.id)
            .collect::<Vec<_>>();
        let first_member_id = member_ids.first().copied();
        if let Some(first_member_id) = first_member_id {
            if self.selected_id == Some(group_id) {
                self.selected_id = Some(first_member_id);
                self.primary_target = Some(SelectionTarget::Entry(first_member_id));
            }
            if self.hovered_id == Some(group_id) {
                self.hovered_id = Some(first_member_id);
            }
            if self.diff_image1_id == Some(group_id) {
                self.diff_image1_id = Some(first_member_id);
            }
            if self.diff_image2_id == Some(group_id) {
                self.diff_image2_id = Some(first_member_id);
            }
        }
        for (offset, member) in members.members.into_iter().enumerate() {
            self.items.insert(
                index + offset,
                WorkspaceItem {
                    id: member.id,
                    content_revision: 0,
                    content: SidebarItem::Image(member.image),
                },
            );
        }
        Some(member_ids)
    }

    pub fn ungroup(&mut self, group_id: WorkspaceId) -> bool {
        let Some(member_ids) = self.ungroup_members(group_id) else {
            return false;
        };
        if let Some(first_member_id) = member_ids.first().copied() {
            self.select(first_member_id);
        }
        true
    }

    #[allow(dead_code)]
    pub fn replace_id(&mut self, id: WorkspaceId, content: SidebarItem) -> Option<SidebarItem> {
        let item = self.items.iter_mut().find(|item| item.id == id)?;
        let previous = std::mem::replace(&mut item.content, content);
        self.invalidate_derived_state();
        Some(previous)
    }

    fn invalidate_selection_state(&mut self) {
        self.selected_op = None;
        self.indexed_hover_palette = None;
        self.indexed_edit_palette = None;
        self.indexed_requantized = None;
        self.indexed_dither_cached = u32::MAX;
        self.indexed_dither_cached_id = None;
        self.indexed_dither_cached_revision = 0;
        self.font_rendered_preview = None;
        self.font_atlas_cached = None;
        self.font_grid_cached = None;
        self.font_grid_big_cached = None;
        self.selected_glyph = None;
        self.font_bundle_index = 0;
    }

    fn invalidate_derived_state(&mut self) {
        self.diff_result = None;
        self.selected_diff_pixel = None;
        self.hovered_diff_pixel = None;
        self.hovered_diff_pixel_from_plot = None;
        self.invalidate_selection_state();
    }

    pub fn remove_id(&mut self, id: WorkspaceId) -> Option<SidebarItem> {
        let index = self.index_of(id)?;
        let removed = self.items.remove(index).content;
        self.sequence_groups.remove(&id);
        self.sequence_groups
            .retain(|_, members| members.members.iter().all(|member| member.id != id));

        self.selected_ids.remove(&id);
        if self.focused_id == Some(id) {
            self.focused_id = None;
        }
        if self.range_anchor == Some(id) {
            self.range_anchor = None;
        }
        if self.selected_id == Some(id) {
            self.selected_id = self
                .items
                .get(index)
                .or_else(|| self.items.last())
                .map(|item| item.id);
            if let Some(selected) = self.selected_id {
                self.selected_ids.insert(selected);
                self.focused_id = Some(selected);
            }
        }
        if self.hovered_id == Some(id) {
            self.hovered_id = None;
        }
        if self.diff_image1_id == Some(id) {
            self.diff_image1_id = None;
        }
        if self.diff_image2_id == Some(id) {
            self.diff_image2_id = None;
        }
        self.invalidate_derived_state();
        Some(removed)
    }

    pub fn clear_items(&mut self) {
        self.items.clear();
        self.sequence_groups.clear();
        self.selected_id = None;
        self.selected_ids.clear();
        self.focused_id = None;
        self.range_anchor = None;
        self.hovered_id = None;
        self.diff_image1_id = None;
        self.diff_image2_id = None;
        self.invalidate_derived_state();
    }
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next_workspace_id: 1,
            sequence_groups: HashMap::new(),
            selected_id: None,
            primary_target: None,
            renaming_group: None,
            rename_buffer: String::new(),
            selected_ids: BTreeSet::new(),
            list_focus: false,
            focused_id: None,
            range_anchor: None,
            hovered_id: None,
            dropped_files: Vec::new(),
            input_format: crate::converter::ImageFormatCategory::Auto,
            context: AppContext::default(),
            diff_image1_id: None,
            diff_image2_id: None,
            diff_result: None,
            selected_diff_pixel: None,
            hovered_diff_pixel: None,
            hovered_diff_pixel_from_plot: None,
            is_converting: false,
            font_preview_text: "The quick brown fox".to_string(),
            font_rendered_preview: None,
            selected_op: None,
            path_mode: PathMode::default(),
            indexed_hover_palette: None,
            indexed_edit_palette: None,
            indexed_edit_color: Color32::WHITE,
            indexed_show_quality: false,
            indexed_view_mode: IndexedViewMode::default(),
            font_bake_size: 24,
            font_bake_format: "sdf".to_string(),
            font_bake_bit_depth: 4,
            font_bake_charset_tab: BakeCharsetTab::default(),
            font_bake_charset_text: "ABCabc012 .,;:!?".to_string(),
            font_bake_charset_ranges: "U+0020-U+007F".to_string(),
            font_bake_charset_file: None,
            indexed_dither: 0,
            indexed_dither_cached: u32::MAX,
            indexed_dither_cached_id: None,
            indexed_dither_cached_revision: 0,
            indexed_requantized: None,
            merge_font_paths: Vec::new(),
            font_mode: FontMode::default(),
            glyph_diff_char: "A".to_string(),
            selected_glyph: None,
            opened_glyphs: Vec::new(),
            glyph_convert_format: "LVGL".to_string(),
            path_export_format: "PNG".to_string(),
            font_atlas_cached: None,
            font_grid_cached: None,
            font_grid_big_cached: None,
            font_bundle_index: 0,
            glyph_canvas_view: GlyphCanvasView::default(),
            render_canvas_view: GlyphCanvasView::default(),
            #[cfg(target_arch = "wasm32")]
            pending_dropped: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(name: &str) -> SidebarItem {
        SidebarItem::Glyph(OpenedGlyph {
            name: name.to_string(),
            codepoint: 0,
            char_repr: String::new(),
            advance: 0,
            bearing: (0, 0),
            bbox: (0, 0, 0, 0),
            outline: Vec::new(),
            outline_approximate: false,
            source_font: String::new(),
            source_is_sdf: false,
            editor: GlyphEditorState::default(),
        })
    }

    fn image(path: &str) -> SidebarItem {
        SidebarItem::Image(ImageItem {
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
        })
    }

    #[test]
    fn glyph_editor_history_round_trips_and_clears_redo() {
        let first = vec![icu_lib::mirx::PathCmd::Close];
        let second = vec![icu_lib::mirx::PathCmd::MoveTo(icu_lib::mirx::Point::new(
            icu_lib::mirx::Fixed::from_int(1),
            icu_lib::mirx::Fixed::from_int(2),
        ))];
        let third = vec![icu_lib::mirx::PathCmd::MoveTo(icu_lib::mirx::Point::new(
            icu_lib::mirx::Fixed::from_int(3),
            icu_lib::mirx::Fixed::from_int(4),
        ))];
        let mut editor = GlyphEditorState::default();
        editor.record(first.clone());
        assert_eq!(editor.undo(&second), Some(first));
        assert_eq!(editor.redo(&second), Some(second.clone()));
        editor.record(third);
        assert!(!editor.can_redo());
    }

    #[test]
    fn glyph_node_commands_preserve_path_structure() {
        let p = |x, y| {
            icu_lib::mirx::Point::new(
                icu_lib::mirx::Fixed::from_int(x),
                icu_lib::mirx::Fixed::from_int(y),
            )
        };
        let mut outline = vec![
            icu_lib::mirx::PathCmd::MoveTo(p(0, 0)),
            icu_lib::mirx::PathCmd::LineTo(p(10, 0)),
            icu_lib::mirx::PathCmd::QuadTo {
                ctrl: p(12, 4),
                end: p(10, 10),
            },
            icu_lib::mirx::PathCmd::Close,
        ];
        let first = GlyphNodeId {
            command_index: 0,
            role: GlyphNodeRole::Endpoint,
        };
        assert!(add_glyph_node(&mut outline, first));
        assert!(matches!(outline[1], icu_lib::mirx::PathCmd::LineTo(_)));
        assert!(!delete_glyph_node(&mut outline, first));

        let line = GlyphNodeId {
            command_index: 1,
            role: GlyphNodeRole::Endpoint,
        };
        assert!(delete_glyph_node(&mut outline, line));
        let quad = GlyphNodeId {
            command_index: 2,
            role: GlyphNodeRole::QuadControl,
        };
        assert!(delete_glyph_node(&mut outline, quad));
        assert!(matches!(outline[2], icu_lib::mirx::PathCmd::LineTo(_)));

        let endpoint = GlyphNodeId {
            command_index: 1,
            role: GlyphNodeRole::Endpoint,
        };
        assert!(move_glyph_node(&mut outline, endpoint, p(20, 20)));
        assert_eq!(glyph_node_point(&outline, endpoint), Some(p(20, 20)));
        assert!(matches!(
            outline.first(),
            Some(icu_lib::mirx::PathCmd::MoveTo(_))
        ));
        assert!(matches!(
            outline.last(),
            Some(icu_lib::mirx::PathCmd::Close)
        ));
    }

    #[test]
    fn glyph_editor_history_is_bounded() {
        let mut editor = GlyphEditorState::default();
        for index in 0..=GLYPH_HISTORY_LIMIT {
            editor.record(vec![icu_lib::mirx::PathCmd::MoveTo(
                icu_lib::mirx::Point::new(
                    icu_lib::mirx::Fixed::from_int(index as i32),
                    icu_lib::mirx::Fixed::ZERO,
                ),
            )]);
        }
        let mut count = 0;
        while editor.can_undo() {
            let _ = editor.undo(&[]);
            count += 1;
        }
        assert_eq!(count, GLYPH_HISTORY_LIMIT);
    }

    #[test]
    fn inserting_before_selection_preserves_identity() {
        let mut state = ViewerState::default();
        let ids = state.append_items([glyph("first"), glyph("selected")]);
        assert!(state.select(ids[1]));

        state.insert_items(0, [glyph("inserted")]);

        assert_eq!(state.selected_id, Some(ids[1]));
        assert_eq!(state.index_of(ids[1]), Some(2));
        assert_eq!(
            state.selected_item().map(SidebarItem::name),
            Some("selected")
        );
    }

    #[test]
    fn removing_selection_uses_next_then_previous_and_clears_references() {
        let mut state = ViewerState::default();
        let ids = state.append_items([glyph("first"), glyph("middle"), glyph("last")]);
        state.select(ids[1]);
        state.hovered_id = Some(ids[1]);
        state.diff_image1_id = Some(ids[1]);
        state.diff_image2_id = Some(ids[2]);

        state.remove_id(ids[1]);

        assert_eq!(state.selected_id, Some(ids[2]));
        assert_eq!(state.hovered_id, None);
        assert_eq!(state.diff_image1_id, None);
        assert_eq!(state.diff_image2_id, Some(ids[2]));

        state.remove_id(ids[2]);
        assert_eq!(state.selected_id, Some(ids[0]));
    }

    #[test]
    fn current_image_mut_edits_workspace_source() {
        let mut state = ViewerState::default();
        let id = state.append_items([image("before")])[0];
        state.select(id);

        state.current_image_mut().unwrap().path = "after".to_string();

        assert_eq!(
            state.current_image().map(|image| image.path.as_str()),
            Some("after")
        );
        assert_eq!(state.item(id).map(SidebarItem::name), Some("after"));
    }

    #[test]
    fn sequence_key_uses_rightmost_numeric_token_and_stable_suffix() {
        let (key, number) = sequence_key("/tmp/walk_0001_diffuse.png", 8, 4).unwrap();
        assert_eq!(number, 1);
        assert_eq!(key.1, "walk_");
        assert_eq!(key.2, "_diffuse");
        assert_eq!(key.3, "png");
        assert_eq!(key.4, 8);
        assert_eq!(key.5, 4);
        assert!(sequence_key("/tmp/2024.png", 8, 4).is_some());
        assert!(sequence_key("/tmp/2024", 8, 4).is_none());
    }

    #[test]
    fn numeric_filename_sequence_is_grouped_in_one_batch() {
        let mut state = ViewerState::default();
        state.insert_and_select_first([
            image("/tmp/0.bin"),
            image("/tmp/1.bin"),
            image("/tmp/2.bin"),
        ]);
        assert_eq!(state.len(), 1);
        assert!(state.is_sequence_group(state.items()[0].id()));
        assert_eq!(state.current_image().unwrap().frame_count(), 3);
    }

    #[test]
    fn multiple_sequences_group_in_one_batch_without_index_drift() {
        let mut state = ViewerState::default();
        state.insert_and_select_first([
            image("/tmp/walk_0001.png"),
            image("/tmp/walk_0002.png"),
            image("/tmp/idle_0001.png"),
            image("/tmp/idle_0002.png"),
            image("/tmp/jump_0001.png"),
            image("/tmp/jump_0002.png"),
            image("/tmp/static.png"),
        ]);

        assert_eq!(state.len(), 4);
        assert_eq!(
            state
                .items()
                .iter()
                .filter(|item| state.is_sequence_group(item.id()))
                .count(),
            3
        );
        assert_eq!(
            state
                .items()
                .iter()
                .filter_map(|item| item.content().as_image())
                .map(ImageItem::frame_count)
                .collect::<Vec<_>>(),
            vec![2, 2, 2, 1]
        );
    }

    #[test]
    fn auto_group_is_workspace_wide_and_ungroup_is_lossless() {
        let mut state = ViewerState::default();
        let first_batch = state
            .insert_and_select_first([image("/tmp/walk_0001.png"), image("/tmp/walk_0002.png")]);
        assert_eq!(state.len(), 1);
        let group_id = state.items()[0].id();
        assert!(state.is_sequence_group(group_id));
        assert_eq!(state.current_image().unwrap().frame_count(), 2);

        let second_batch = state.insert_and_select_first([image("/tmp/walk_0003.png")]);
        assert_eq!(state.len(), 1);
        let regrouped_id = state.items()[0].id();
        assert_ne!(regrouped_id, group_id);
        assert!(state.is_sequence_group(regrouped_id));
        assert_eq!(state.current_image().unwrap().frame_count(), 3);
        assert_eq!(state.selected_id, Some(regrouped_id));
        assert!(!second_batch.is_empty());

        assert!(state.ungroup(regrouped_id));
        assert_eq!(state.len(), 3);
        let paths = state
            .items()
            .iter()
            .map(|item| item.content().name().to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            [
                "/tmp/walk_0001.png",
                "/tmp/walk_0002.png",
                "/tmp/walk_0003.png"
            ]
        );
        assert_eq!(state.items()[0].id(), first_batch[0]);
        assert_eq!(state.items()[1].id(), first_batch[1]);
        assert_eq!(state.items()[2].id(), second_batch[0]);
    }

    #[test]
    fn regroup_maps_member_references_to_the_new_group_id() {
        let mut state = ViewerState::default();
        state.insert_and_select_first([image("/tmp/walk_0001.png"), image("/tmp/walk_0002.png")]);
        let old_group_id = state.items()[0].id();
        state.hovered_id = Some(old_group_id);
        state.diff_image1_id = Some(old_group_id);

        let earlier = state.insert_and_select_first([image("/tmp/walk_0000.png")]);
        let new_group_id = state.items()[0].id();

        assert_ne!(new_group_id, earlier[0]);
        assert_ne!(new_group_id, old_group_id);
        assert_eq!(state.selected_id, Some(new_group_id));
        assert_eq!(state.hovered_id, Some(new_group_id));
        assert_eq!(state.diff_image1_id, Some(new_group_id));
        assert!(state.item(old_group_id).is_none());
    }

    #[test]
    fn manual_group_is_reversible_and_rejects_duplicate_ids() {
        let mut state = ViewerState::default();
        let ids = state.append_items([image("/a/a.png"), image("/a/b.png")]);
        assert!(state.group_images(&[ids[0], ids[0]]).is_none());

        let group_id = state.group_images(&ids).unwrap();
        assert_eq!(state.len(), 1);
        assert!(state.is_sequence_group(group_id));
        assert!(state.ungroup(group_id));
        assert_eq!(state.items()[0].id(), ids[0]);
        assert_eq!(state.items()[1].id(), ids[1]);
    }

    #[test]
    fn changing_group_interval_updates_playback_frames() {
        let mut state = ViewerState::default();
        state.insert_and_select_first([image("/tmp/walk_0001.png"), image("/tmp/walk_0002.png")]);
        let group_id = state.items()[0].id();
        assert!(state.set_animation_interval(group_id, Duration::from_millis(240)));
        let image = state.current_image().unwrap();
        let FrameSource::Animated { frames, .. } = &image.frames else {
            panic!("expected grouped animation");
        };
        assert!(
            frames
                .iter()
                .all(|frame| frame.delay == Duration::from_millis(240))
        );
    }

    #[test]
    fn regroup_preserves_playback_state_and_group_frames_loop() {
        let mut state = ViewerState::default();
        state.insert_and_select_first([image("/tmp/walk_0001.png"), image("/tmp/walk_0002.png")]);
        let group_id = state.items()[0].id();
        let group_image = match state.item_mut(group_id).unwrap() {
            SidebarItem::Image(image) => image,
            SidebarItem::Glyph(_) => unreachable!(),
        };
        group_image.expanded = true;
        if let FrameSource::Animated {
            current,
            autoplay,
            last_advance,
            ..
        } = &mut group_image.frames
        {
            *current = 1;
            *autoplay = false;
            *last_advance = Some(Instant::now() - Duration::from_secs(1));
        }

        state.insert_and_select_first([image("/tmp/walk_0000.png")]);
        let image = state.current_image_mut().unwrap();
        assert!(image.expanded);
        assert!(!image.autoplay());
        assert!(!image.advance_frame());
        if let FrameSource::Animated {
            current,
            autoplay,
            last_advance,
            ..
        } = &mut image.frames
        {
            assert_eq!(*current, 2);
            *autoplay = true;
            *last_advance = Some(Instant::now() - Duration::from_millis(101));
        }
        assert!(image.advance_frame());
        assert!(matches!(
            image.frames,
            FrameSource::Animated { current: 0, .. }
        ));
    }

    #[test]
    fn auto_group_rejects_gaps_padding_extension_directory_and_size_mismatch() {
        let cases = [
            ["/a/walk_01.png", "/a/walk_03.png"],
            ["/a/walk_1.png", "/a/walk_02.png"],
            ["/a/walk_01.png", "/a/walk_02.jpg"],
            ["/a/walk_01.png", "/b/walk_02.png"],
        ];
        for (paths, expected_len) in cases.into_iter().zip([2, 1, 2, 2]) {
            let mut state = ViewerState::default();
            state.insert_and_select_first(paths.map(image));
            assert_eq!(state.len(), expected_len, "unexpected group for {paths:?}");
        }

        let mut state = ViewerState::default();
        let mut second = match image("/a/walk_02.png") {
            SidebarItem::Image(image) => image,
            SidebarItem::Glyph(_) => unreachable!(),
        };
        second.width = 2;
        state.insert_and_select_first([image("/a/walk_01.png"), SidebarItem::Image(second)]);
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn duplicate_paths_are_not_reinserted() {
        let mut state = ViewerState::default();
        state.insert_and_select_first([image("/a/frame_01.png")]);
        let ids =
            state.insert_and_select_first([image("/a/frame_01.png"), image("/a/frame_01.png")]);
        assert!(ids.is_empty());
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn multi_selection_supports_toggle_range_focus_and_keyboard_bounds() {
        let mut state = ViewerState::default();
        let ids = state.append_items([
            glyph("first"),
            glyph("second"),
            glyph("third"),
            glyph("fourth"),
        ]);

        assert!(state.focus_list(ids[1]));
        assert_eq!(state.selected_ids, BTreeSet::from([ids[1]]));
        assert!(state.toggle_selection(ids[3]));
        assert_eq!(state.selected_ids, BTreeSet::from([ids[1], ids[3]]));
        assert!(state.extend_selection(ids[2]));
        assert_eq!(state.selected_ids, BTreeSet::from([ids[1], ids[2]]));
        assert!(state.move_selection(-1));
        assert_eq!(state.selected_id, Some(ids[1]));
        assert!(state.move_selection(-1));
        assert_eq!(state.selected_id, Some(ids[0]));
        assert!(!state.move_selection(-1));
        assert!(state.move_selection(1));
        assert_eq!(state.selected_id, Some(ids[1]));
        state.blur_list();
        assert!(!state.move_selection(1));
    }

    #[test]
    fn batch_remove_keeps_selection_consistent() {
        let mut state = ViewerState::default();
        let ids = state.append_items([
            image("/a/one.png"),
            image("/a/two.png"),
            image("/a/three.png"),
        ]);
        state.focus_list(ids[0]);
        state.toggle_selection(ids[1]);
        state.remove_selected();
        assert_eq!(state.len(), 1);
        assert_eq!(state.selected_ids, BTreeSet::from([ids[2]]));
        assert_eq!(state.selected_id, Some(ids[2]));
        assert!(
            state
                .items()
                .iter()
                .all(|item| item.id() != ids[0] && item.id() != ids[1])
        );
    }

    #[test]
    fn grouping_selected_items_updates_selection_to_group() {
        let mut state = ViewerState::default();
        let ids = state.append_items([image("/a/one.png"), image("/a/two.png")]);
        state.focus_list(ids[0]);
        state.toggle_selection(ids[1]);
        let group_id = state.group_selected().unwrap();
        assert_eq!(state.selected_ids, BTreeSet::from([group_id]));
        assert!(state.is_sequence_group(group_id));
        assert_eq!(state.ungroup_selected(), 1);
        assert_eq!(state.selected_ids, ids.into_iter().collect());
    }

    #[test]
    fn ungroup_selected_restores_every_selected_group() {
        let mut state = ViewerState::default();
        let first = state.append_items([image("/a/one.png"), image("/a/two.png")]);
        let second = state.append_items([image("/b/three.png"), image("/b/four.png")]);
        let first_group = state.group_images(&first).unwrap();
        let second_group = state.group_images(&second).unwrap();
        state.focus_list(first_group);
        state.toggle_selection(second_group);

        assert_eq!(state.ungroup_selected(), 2);
        assert_eq!(state.len(), 4);
        assert_eq!(
            state.selected_ids,
            first.iter().chain(&second).copied().collect()
        );
        assert_eq!(state.selected_id, Some(first[0]));
        assert_eq!(state.focused_id, Some(first[0]));
        assert!(state.sequence_groups.is_empty());
    }

    #[test]
    fn workspace_ids_remain_unique_across_insert_remove_and_reinsert() {
        let mut state = ViewerState::default();
        let first = state.append_items([glyph("a"), glyph("b")]);
        let inserted = state.insert_items(1, [glyph("c")]);
        state.remove_id(first[0]);
        let later = state.append_items([glyph("d"), glyph("e")]);

        let ids = state
            .items()
            .iter()
            .map(WorkspaceItem::id)
            .collect::<Vec<_>>();
        let unique = ids
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), unique.len());
        assert!(!ids.contains(&first[0]));
        assert!(ids.contains(&first[1]));
        assert!(ids.contains(&inserted[0]));
        assert!(
            later
                .iter()
                .all(|id| !first.contains(id) && !inserted.contains(id))
        );
    }

    #[test]
    fn changing_selection_invalidates_item_bound_state() {
        let mut state = ViewerState::default();
        let ids = state.append_items([image("first"), image("second")]);
        state.select(ids[0]);
        state.indexed_requantized = Some(icu_lib::midata::IndexedImageData {
            rgba: icu_lib::image::RgbaImage::new(1, 1),
            palette: vec![[0, 0, 0, 255]],
            indexes: vec![0],
            bpp: 1,
            width: 1,
            height: 1,
        });
        state.indexed_dither_cached = 7;
        state.font_rendered_preview = Some(icu_lib::image::RgbaImage::new(1, 1));
        state.selected_glyph = Some(3);
        state.font_bundle_index = 2;

        state.select(ids[1]);

        assert!(state.indexed_requantized.is_none());
        assert_eq!(state.indexed_dither_cached, u32::MAX);
        assert!(state.font_rendered_preview.is_none());
        assert_eq!(state.selected_glyph, None);
        assert_eq!(state.font_bundle_index, 0);
    }

    #[test]
    fn replacing_content_preserves_id_and_invalidates_derived_state() {
        let mut state = ViewerState::default();
        let id = state.append_items([glyph("before")])[0];
        state.select(id);
        state.diff_image1_id = Some(id);
        state.selected_diff_pixel = Some([1, 2]);
        state.indexed_dither_cached = 3;

        let previous = state.replace_id(id, glyph("after"));

        assert_eq!(previous.as_ref().map(SidebarItem::name), Some("before"));
        assert_eq!(state.items()[0].id(), id);
        assert_eq!(state.selected_item().map(SidebarItem::name), Some("after"));
        assert_eq!(state.diff_image1_id, Some(id));
        assert_eq!(state.selected_diff_pixel, None);
        assert_eq!(state.indexed_dither_cached, u32::MAX);
    }
}
