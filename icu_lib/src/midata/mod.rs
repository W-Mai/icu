use crate::endecoder::EnDecoder;
use crate::EncoderParams;
use image::{GrayAlphaImage, RgbaImage};

#[derive(Clone, PartialEq)]
pub enum MiData {
    RGBA(RgbaImage),
    GRAY(GrayAlphaImage),
    PATH(SceneData),
    FONT(FontData),
    INDEXED(IndexedImageData),
}

impl MiData {
    pub fn variant_name(&self) -> &'static str {
        match self {
            MiData::RGBA(_) => "RGBA",
            MiData::GRAY(_) => "GRAY",
            MiData::PATH(_) => "PATH",
            MiData::FONT(_) => "FONT",
            MiData::INDEXED(_) => "INDEXED",
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct SceneData {
    pub scene: mirx::Scene,
}

#[derive(Clone, PartialEq)]
pub enum FontData {
    Mirx(mirx::Font),
    FreeType(FreeTypeFontData),
}

#[derive(Clone, PartialEq)]
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

#[derive(Clone, PartialEq)]
pub struct FreeTypeGlyph {
    pub codepoint: u32,
    pub advance: u16,
    pub bearing_x: i16,
    pub bearing_y: i16,
    pub bbox: (i16, i16, i16, i16),
    pub outline: Vec<mirx::PathCmd>,
}

#[derive(Clone, PartialEq)]
pub struct IndexedImageData {
    pub rgba: RgbaImage,
    pub palette: Vec<[u8; 4]>,
    pub indexes: Vec<u8>,
    pub bpp: u8,
    pub width: u32,
    pub height: u32,
}

pub fn requantize_indexed(
    indexed: &IndexedImageData,
    dither_level: u32,
) -> Option<IndexedImageData> {
    use image::imageops;
    let color_map_size = 1usize << indexed.bpp;
    let nq = color_quant::NeuQuant::new(dither_level as i32, color_map_size, indexed.rgba.as_raw());
    let mut img = indexed.rgba.clone();
    if dither_level > 0 {
        imageops::dither(&mut img, &nq);
    }
    let palette: Vec<[u8; 4]> = nq
        .color_map_rgba()
        .chunks(4)
        .map(|c| [c[0], c[1], c[2], c[3]])
        .collect();
    let indexes: Vec<u8> = img
        .pixels()
        .map(|p| nq.index_of(&p.0) as u8)
        .collect();
    Some(IndexedImageData {
        rgba: img,
        palette,
        indexes,
        bpp: indexed.bpp,
        width: indexed.width,
        height: indexed.height,
    })
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
