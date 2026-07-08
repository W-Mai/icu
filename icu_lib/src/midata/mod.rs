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

pub enum FontData {
    Mirx(mirx::Font),
    FreeType(FreeTypeFontData),
}

pub struct FreeTypeFontData {
    pub family: String,
    pub style: String,
    pub units_per_em: u16,
    pub ascender: i16,
    pub descender: i16,
    pub line_height: i16,
    pub glyph_count: u32,
    pub glyphs: Vec<FreeTypeGlyph>,
}

pub struct FreeTypeGlyph {
    pub codepoint: u32,
    pub advance: u16,
    pub bearing_x: i16,
    pub bearing_y: i16,
    pub bbox: (i16, i16, i16, i16),
    pub outline: Vec<mirx::PathCmd>,
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
