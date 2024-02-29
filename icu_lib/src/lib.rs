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
