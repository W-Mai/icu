pub use crate::converter::{
    ImageFormats as ImageFormat, LVGL_Version as LvglVersion,
    OutputColorFormats as LvglColorFormat, OutputCompressedMethod as LvglCompression,
};
use clap::ValueEnum;
use eframe::egui::{Color32, DroppedFile};
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

#[derive(Serialize, Deserialize, Clone)]
pub struct AppContext {
    pub show_grid: bool,
    pub anti_alias: bool,
    pub image_diff: bool,
    pub background_color: Color32,
    pub diff_blend: f32,     // Controls the alpha blending for diff mode
    pub diff_tolerance: f32, // Tolerance for diff
    pub min_diff: f32,       // Minimum diff to show
    pub max_diff: f32,       // Maximum diff to show

    pub fast_switch: bool,      // Whether fast switch is enabled
    pub fast_switch_speed: f32, // Speed of fast switch (Hz)
    pub fast_switch_phase: f32, // Internal phase for fast switch
    pub only_show_diff: bool,   // Only show diff area
    pub language: String,

    pub diff_sorting: DiffSorting,
    pub diff_page_index: usize,
    pub diff_page_size: usize,

    pub show_convert_panel: bool,
    pub convert_params: ConvertParams,
}

#[derive(Serialize, Deserialize, PartialEq, Copy, Clone, Debug, ValueEnum)]
pub enum DiffSorting {
    Z,        // Z-order (default, row by row)
    N,        // N-order (column by column)
    ReverseZ, // Reverse Z-order
    ReverseN, // Reverse N-order
    DiffAsc,  // Diff value ascending
    DiffDesc, // Diff value descending
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConvertParams {
    pub output_format: ImageFormat,
    pub lvgl_version: LvglVersion,
    pub color_format: LvglColorFormat,
    pub compression: LvglCompression,
    pub stride_align: u8,
    pub dither: bool,
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
        }
    }
}

impl Default for AppContext {
    fn default() -> Self {
        Self {
            show_grid: true,
            anti_alias: true,
            image_diff: false,
            background_color: Default::default(),
            diff_blend: 0.5,     // Default alpha for diff blending
            diff_tolerance: 0.1, // Default tolerance for diff
            min_diff: 0.0,       // Default minimum diff to show
            max_diff: f32::MAX,  // Default maximum diff to show
            fast_switch: false,
            fast_switch_speed: 1.0,
            fast_switch_phase: 0.0,
            only_show_diff: false,
            language: crate::image_viewer::utils::get_system_locale(),
            diff_sorting: DiffSorting::Z,
            diff_page_index: 0,
            diff_page_size: 100,
            show_convert_panel: false,
            convert_params: ConvertParams::default(),
        }
    }
}

pub struct ViewerState {
    pub current_image: Option<ImageItem>,
    pub image_items: Vec<ImageItem>,
    pub selected_image_item_index: Option<usize>,
    pub hovered_image_item_index: Option<usize>,
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
    pub path_selected_op: Option<usize>,
    pub indexed_hover_palette: Option<u8>,
    pub indexed_show_quality: bool,
    pub font_bake_size: u16,
    pub font_bake_format: String,
    pub indexed_dither: u32,
    pub indexed_dither_cached: u32,
    pub indexed_requantized: Option<icu_lib::midata::IndexedImageData>,
    pub merge_font_paths: Vec<String>,
    pub font_view_mode: String,
    pub font_diff_path: Option<String>,
    pub font_selected_glyph: Option<usize>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            current_image: None,
            image_items: Vec::new(),
            selected_image_item_index: None,
            hovered_image_item_index: None,
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
            path_selected_op: None,
            indexed_hover_palette: None,
            indexed_show_quality: false,
            font_bake_size: 24,
            font_bake_format: "sdf".to_string(),
            indexed_dither: 0,
            indexed_dither_cached: u32::MAX,
            indexed_requantized: None,
            merge_font_paths: Vec::new(),
            font_view_mode: "atlas".to_string(),
            font_diff_path: None,
            font_selected_glyph: None,
        }
    }
}
