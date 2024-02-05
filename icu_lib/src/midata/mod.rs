use crate::endecoder::EnDecoder;
use crate::EncoderParams;
use image::{GrayAlphaImage, RgbaImage};

pub enum MiData {
    RGBA(RgbaImage),
    GRAY(GrayAlphaImage),
    PATH,
}

impl MiData {
    pub fn decode_from<ED: EnDecoder>(data: Vec<u8>) -> Self {
        ED::decode(data)
    }

    pub fn encode_into<ED: EnDecoder>(&self, encoder_params: EncoderParams) -> Vec<u8> {
        ED::encode(self, encoder_params)
    }
}
