pub use crate::converter::{
    ImageFormats as ImageFormat, LVGL_Version as LvglVersion,
    OutputColorFormats as LvglColorFormat, OutputCompressedMethod as LvglCompression,
};
use clap::ValueEnum;
use eframe::egui::{Color32, DroppedFile};
use icu_lib::endecoder::ImageInfo;
use icu_lib::endecoder::utils::diff::ImageDiffResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq)]
pub struct ImageItem {
    pub path: String,
    pub info: ImageInfo,
    pub width: u32,
    pub height: u32,
    pub image_data: Vec<Color32>,
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

#[derive(Default)]
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
}
