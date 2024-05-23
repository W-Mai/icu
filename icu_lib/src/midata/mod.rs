use crate::endecoder::EnDecoder;
use crate::EncoderParams;
use image::{GrayAlphaImage, RgbaImage};

pub enum MiData {
    RGBA(RgbaImage),
    GRAY(GrayAlphaImage),
    PATH(usvg::Tree),
}

impl MiData {
    pub fn decode_from(ed: &dyn EnDecoder, data: Vec<u8>) -> Self {
        ed.decode(data)
    }

    pub fn encode_into(&self, ed: &dyn EnDecoder, encoder_params: EncoderParams) -> Vec<u8> {
        ed.encode(self, encoder_params)
    }
}
