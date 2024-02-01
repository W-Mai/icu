use image::{GrayAlphaImage, RgbaImage};
use crate::endecoder::EnDecoder;

pub enum MiData {
    RGBA(RgbaImage),
    GRAY(GrayAlphaImage),
    PATH,
}

impl MiData {
    fn from(endecoder: &dyn EnDecoder, data: Vec<u8>) -> Self {
        endecoder.decode(data)
    }

    fn into(endecoder: &dyn EnDecoder, data: &Self) -> Vec<u8> {
        endecoder.encode(data)
    }
}

