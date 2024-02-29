use crate::endecoder::lvgl_v9;

pub mod endecoder;
pub mod midata;

pub struct EncoderParams {
    pub color_format: lvgl_v9::ColorFormat,
    pub stride_align: u32,
    pub dither: bool,
}

impl Default for EncoderParams {
    fn default() -> Self {
        Self {
            color_format: Default::default(),
            stride_align: 1,
            dither: false,
        }
    }
}

impl EncoderParams {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_color_format(mut self, color_format: lvgl_v9::ColorFormat) -> Self {
        self.color_format = color_format;
        self
    }

    pub fn with_stride_align(mut self, stride_align: u32) -> Self {
        self.stride_align = stride_align;
        self
    }

    pub fn with_dither(mut self, dither: bool) -> Self {
        self.dither = dither;
        self
    }
}
