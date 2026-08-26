use crate::endecoder::lvgl;

pub use image;
pub use mirx;

pub mod endecoder;
pub mod midata;
pub mod postprocess;
type RawImageHeader = lvgl::ImageHeader;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PngColorMode {
    #[default]
    Rgba,
    Rgb,
    Indexed(u8),
    Preserve,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PngCompression {
    Fast,
    #[default]
    Balanced,
    Best,
}

pub struct EncoderParams {
    pub color_format: endecoder::ColorFormat,
    pub stride_align: u32,
    pub dither: Option<u32>,
    pub compress: lvgl::Compress,
    pub lvgl_version: lvgl::LVGLVersion,
    pub raw_image_header: Option<RawImageHeader>,
    pub png_color_mode: PngColorMode,
    pub png_compression: PngCompression,
    pub jpeg_quality: u8,
    pub jpeg_background: [u8; 3],
}

impl Default for EncoderParams {
    fn default() -> Self {
        Self {
            color_format: endecoder::ColorFormat::RGB565,
            stride_align: 1,
            dither: None,
            compress: Default::default(),
            lvgl_version: lvgl::LVGLVersion::Unknown,
            raw_image_header: Default::default(),
            png_color_mode: PngColorMode::default(),
            png_compression: PngCompression::default(),
            jpeg_quality: 85,
            jpeg_background: [255, 255, 255],
        }
    }
}

impl EncoderParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_color_format(mut self, color_format: endecoder::ColorFormat) -> Self {
        self.color_format = color_format;
        self
    }

    pub fn with_stride_align(mut self, stride_align: u32) -> Self {
        self.stride_align = stride_align;
        self
    }

    pub fn with_dither(mut self, dither: Option<u32>) -> Self {
        self.dither = dither;
        self
    }

    pub fn with_compress(mut self, compress: lvgl::Compress) -> Self {
        self.compress = compress;
        self
    }

    pub fn with_lvgl_version(mut self, lvgl_version: lvgl::LVGLVersion) -> Self {
        self.lvgl_version = lvgl_version;
        self
    }

    pub fn with_raw_image_header(mut self, raw_image_header: RawImageHeader) -> Self {
        self.raw_image_header = Some(raw_image_header);
        self
    }

    pub fn with_png_color_mode(mut self, png_color_mode: PngColorMode) -> Self {
        self.png_color_mode = png_color_mode;
        self
    }

    pub fn with_png_compression(mut self, png_compression: PngCompression) -> Self {
        self.png_compression = png_compression;
        self
    }

    pub fn with_jpeg_quality(mut self, jpeg_quality: u8) -> Self {
        self.jpeg_quality = jpeg_quality;
        self
    }

    pub fn with_jpeg_background(mut self, jpeg_background: [u8; 3]) -> Self {
        self.jpeg_background = jpeg_background;
        self
    }
}
