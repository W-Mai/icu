use image::{GrayAlphaImage, RgbaImage};
use crate::endecoder::EnDecoder;

pub enum MiData {
    RGBA(RgbaImage),
    GRAY(GrayAlphaImage),
    PATH,
}

impl MiData {
    pub fn decode_from(endecoder: &dyn EnDecoder, data: Vec<u8>) -> Self {
        endecoder.decode(data)
    }

    pub fn encode_into(&self, endecoder: &dyn EnDecoder) -> Vec<u8> {
        endecoder.encode(self)
    }
}

