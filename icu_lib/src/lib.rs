use crate::endecoder::lvgl;

pub mod endecoder;
pub mod midata;
type RawImageHeader = lvgl::ImageHeader;

pub struct EncoderParams {
    pub color_format: lvgl::ColorFormat,
    pub stride_align: u32,
    pub dither: Option<u32>,
    pub compress: lvgl::Compress,
    pub lvgl_version: lvgl::LVGLVersion,
    pub raw_image_header: Option<RawImageHeader>,
}

impl Default for EncoderParams {
    fn default() -> Self {
        Self {
            color_format: Default::default(),
            stride_align: 1,
            dither: None,
            compress: Default::default(),
            lvgl_version: lvgl::LVGLVersion::Unknown,
            raw_image_header: Default::default(),
        }
    }
}

impl EncoderParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_color_format(mut self, color_format: lvgl::ColorFormat) -> Self {
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
}
