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

#[derive(Clone, PartialEq)]
pub struct ImageItem {
    pub path: String,
    pub info: ImageInfo,
    pub width: u32,
    pub height: u32,
    pub image_data: Vec<Color32>,
    pub midata: Option<MiData>,
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
        FontMode::Atlas
    }
}

#[derive(Clone, Copy)]
pub struct GlyphCanvasView {
    pub zoom: f32,
    pub pan: Vec2,
}

impl Default for GlyphCanvasView {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct ConvertParams {
    pub output_format: ImageFormat,
    pub lvgl_version: LvglVersion,
    pub color_format: LvglColorFormat,
    pub compression: LvglCompression,
    pub stride_align: u8,
    pub dither: bool,
    pub dither_level: u32,
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
        }
    }
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
        }
    }
}

#[allow(dead_code)]
pub struct ViewerState {
    pub current_image: Option<ImageItem>,
    pub items: Vec<SidebarItem>,
    pub selected_index: Option<usize>,
    pub hovered_index: Option<usize>,
    pub dropped_files: Vec<DroppedFile>,
    pub context: AppContext,
    pub diff_image1_index: Option<usize>,
    pub diff_image2_index: Option<usize>,
    pub diff_result: Option<(ImageItem, ImageDiffResult)>,
    pub selected_diff_pixel: Option<[u32; 2]>,
    pub hovered_diff_pixel: Option<[u32; 2]>,
    pub hovered_diff_pixel_from_plot: Option<[u32; 2]>,
    pub is_converting: bool,
    pub font_preview_text: String,
    pub font_rendered_preview: Option<icu_lib::image::RgbaImage>,
    pub selected_op: Option<usize>,
    pub selected_node: Option<usize>,
    pub path_mode: PathMode,
    pub indexed_hover_palette: Option<u8>,
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
    pub indexed_requantized: Option<icu_lib::midata::IndexedImageData>,
    pub merge_font_paths: Vec<String>,
    pub font_mode: FontMode,
    pub font_diff_path: Option<String>,
    pub selected_glyph: Option<usize>,
    pub opened_glyphs: Vec<OpenedGlyph>,
    pub glyph_convert_format: String,
    pub font_atlas_cached: Option<(String, String, Vec<Color32>, u32, u32)>,
    pub font_grid_cached: Option<(String, Vec<TextureHandle>, usize)>,
    pub font_grid_big_cached: Option<(String, TextureHandle)>,
    pub font_bundle_index: usize,
    pub glyph_canvas_view: GlyphCanvasView,
    #[cfg(target_arch = "wasm32")]
    pub pending_dropped: std::rc::Rc<std::cell::RefCell<Vec<DroppedFile>>>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            current_image: None,
            items: Vec::new(),
            selected_index: None,
            hovered_index: None,
            dropped_files: Vec::new(),
            context: AppContext::default(),
            diff_image1_index: None,
            diff_image2_index: None,
            diff_result: None,
            selected_diff_pixel: None,
            hovered_diff_pixel: None,
            hovered_diff_pixel_from_plot: None,
            is_converting: false,
            font_preview_text: "The quick brown fox".to_string(),
            font_rendered_preview: None,
            selected_op: None,
            selected_node: None,
            path_mode: PathMode::default(),
            indexed_hover_palette: None,
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
            indexed_requantized: None,
            merge_font_paths: Vec::new(),
            font_mode: FontMode::default(),
            font_diff_path: None,
            selected_glyph: None,
            opened_glyphs: Vec::new(),
            glyph_convert_format: "LVGL".to_string(),
            font_atlas_cached: None,
            font_grid_cached: None,
            font_grid_big_cached: None,
            font_bundle_index: 0,
            glyph_canvas_view: GlyphCanvasView::default(),
            #[cfg(target_arch = "wasm32")]
            pending_dropped: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        }
    }
}
