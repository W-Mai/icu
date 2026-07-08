use crate::endecoder::EnDecoder;
use crate::EncoderParams;
use image::{GrayAlphaImage, RgbaImage};

pub enum MiData {
    RGBA(RgbaImage),
    GRAY(GrayAlphaImage),
    PATH(SceneData),
    FONT(FontData),
}

impl MiData {
    pub fn variant_name(&self) -> &'static str {
        match self {
            MiData::RGBA(_) => "RGBA",
            MiData::GRAY(_) => "GRAY",
            MiData::PATH(_) => "PATH",
            MiData::FONT(_) => "FONT",
        }
    }
}

pub struct SceneData {
    pub scene: mirx::Scene,
}

pub struct FontData {
    pub font: mirx::Font,
}

impl MiData {
    pub fn decode_from(ed: &dyn EnDecoder, data: Vec<u8>) -> Self {
        ed.decode(data)
    }

    pub fn encode_into(&self, ed: &dyn EnDecoder, encoder_params: EncoderParams) -> Vec<u8> {
        ed.encode(self, encoder_params)
    }
}

impl MiData {
    pub fn from_rgba(w: u32, h: u32, data: Vec<u8>) -> Option<Self> {
        Some(MiData::RGBA(RgbaImage::from_vec(w, h, data)?))
    }
}
