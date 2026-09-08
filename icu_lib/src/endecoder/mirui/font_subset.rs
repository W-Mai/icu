use std::fmt;

use hb_subset::{sys, Blob, FontFace as HbFace, SubsetInput};
use ttf_parser::Face;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontVariation {
    tag: [u8; 4],
    value: f32,
}

impl FontVariation {
    pub const fn new(tag: [u8; 4], value: f32) -> Self {
        Self { tag, value }
    }

    pub const fn tag(self) -> [u8; 4] {
        self.tag
    }

    pub const fn value(self) -> f32 {
        self.value
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FontSubsetOptions {
    face_index: u32,
    variations: Vec<FontVariation>,
}

impl FontSubsetOptions {
    pub const fn new() -> Self {
        Self {
            face_index: 0,
            variations: Vec::new(),
        }
    }

    pub const fn with_face_index(mut self, face_index: u32) -> Self {
        self.face_index = face_index;
        self
    }

    pub fn with_variation(mut self, tag: [u8; 4], value: f32) -> Self {
        self.variations.push(FontVariation::new(tag, value));
        self
    }

    pub const fn face_index(&self) -> u32 {
        self.face_index
    }

    pub fn variations(&self) -> &[FontVariation] {
        &self.variations
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubsetCmapEntry {
    character: char,
    glyph_id: u16,
}

impl SubsetCmapEntry {
    pub const fn character(self) -> char {
        self.character
    }

    pub const fn glyph_id(self) -> u16 {
        self.glyph_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontSubset {
    bytes: Vec<u8>,
    cmap: Vec<SubsetCmapEntry>,
    glyph_count: u16,
}

impl FontSubset {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn cmap(&self) -> &[SubsetCmapEntry] {
        &self.cmap
    }

    pub const fn glyph_count(&self) -> u16 {
        self.glyph_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontSubsetError {
    InvalidFont,
    EmptyCharset,
    InvalidVariationValue,
    UnknownVariationAxis([u8; 4]),
    DuplicateVariationAxis([u8; 4]),
    VariationOutOfRange([u8; 4]),
    SubsetFailed,
    MissingCodepoint(char),
}

impl fmt::Display for FontSubsetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFont => f.write_str("invalid font source"),
            Self::EmptyCharset => f.write_str("font charset is empty"),
            Self::InvalidVariationValue => f.write_str("font variation value must be finite"),
            Self::UnknownVariationAxis(tag) => write!(f, "unknown font variation axis {tag:?}"),
            Self::DuplicateVariationAxis(tag) => {
                write!(f, "duplicate font variation axis {tag:?}")
            }
            Self::VariationOutOfRange(tag) => {
                write!(f, "font variation axis {tag:?} is outside its range")
            }
            Self::SubsetFailed => f.write_str("font subsetting failed"),
            Self::MissingCodepoint(character) => {
                write!(f, "font subset lost codepoint {character:?}")
            }
        }
    }
}

impl std::error::Error for FontSubsetError {}

pub fn subset_font(
    source: &[u8],
    characters: impl IntoIterator<Item = char>,
    options: &FontSubsetOptions,
) -> Result<FontSubset, FontSubsetError> {
    let mut characters = characters.into_iter().collect::<Vec<_>>();
    characters.sort_unstable();
    characters.dedup();
    if characters.is_empty() {
        return Err(FontSubsetError::EmptyCharset);
    }
    validate_variations(options.variations())?;

    let source_ttf =
        Face::parse(source, options.face_index()).map_err(|_| FontSubsetError::InvalidFont)?;
    let blob = Blob::from_bytes(source).map_err(|_| FontSubsetError::InvalidFont)?;
    let source_face = HbFace::new_with_index(blob, options.face_index())
        .map_err(|_| FontSubsetError::InvalidFont)?;
    let mut input = SubsetInput::new().map_err(|_| FontSubsetError::SubsetFailed)?;
    input.flags().remove_hinting().retain_layout_closure();
    {
        let mut unicodes = input.unicode_set();
        for character in &characters {
            unicodes.insert(*character);
        }
    }
    pin_variations(&mut input, &source_face, &source_ttf, options.variations())?;

    let plan = input
        .plan(&source_face)
        .map_err(|_| FontSubsetError::SubsetFailed)?;
    let subset_face = plan.subset().map_err(|_| FontSubsetError::SubsetFailed)?;
    let bytes = subset_face.underlying_blob().to_vec();
    let face = Face::parse(&bytes, 0).map_err(|_| FontSubsetError::SubsetFailed)?;
    let glyph_count = face.number_of_glyphs();
    let cmap = characters
        .into_iter()
        .map(|character| {
            face.glyph_index(character)
                .map(|glyph| SubsetCmapEntry {
                    character,
                    glyph_id: glyph.0,
                })
                .ok_or(FontSubsetError::MissingCodepoint(character))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FontSubset {
        bytes,
        cmap,
        glyph_count,
    })
}

fn validate_variations(variations: &[FontVariation]) -> Result<(), FontSubsetError> {
    for (index, variation) in variations.iter().enumerate() {
        if !variation.value().is_finite() {
            return Err(FontSubsetError::InvalidVariationValue);
        }
        if variations[..index]
            .iter()
            .any(|other| other.tag() == variation.tag())
        {
            return Err(FontSubsetError::DuplicateVariationAxis(variation.tag()));
        }
    }
    Ok(())
}

fn pin_variations(
    input: &mut SubsetInput,
    hb_face: &HbFace<'_>,
    ttf_face: &Face<'_>,
    variations: &[FontVariation],
) -> Result<(), FontSubsetError> {
    for requested in variations {
        if !ttf_face
            .variation_axes()
            .into_iter()
            .any(|axis| axis.tag.0.to_be_bytes() == requested.tag())
        {
            return Err(FontSubsetError::UnknownVariationAxis(requested.tag()));
        }
    }
    for axis in ttf_face.variation_axes() {
        let tag = axis.tag.0.to_be_bytes();
        let requested = variations
            .iter()
            .find(|variation| variation.tag() == tag)
            .map(|variation| variation.value());
        if requested.is_some_and(|value| value < axis.min_value || value > axis.max_value) {
            return Err(FontSubsetError::VariationOutOfRange(tag));
        }
        let value = requested.unwrap_or(axis.def_value);
        // Both handles stay alive for the call, and HarfBuzz only records the axis value.
        let pinned = unsafe {
            sys::hb_subset_input_pin_axis_location(
                input.as_raw(),
                hb_face.as_raw(),
                axis.tag.0,
                value,
            )
        };
        if pinned == 0 {
            return Err(FontSubsetError::SubsetFailed);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARK_PIXEL: &[u8] =
        include_bytes!("../../../../assets/ark-pixel-12px-monospaced-zh_cn.otf");

    #[test]
    fn subset_is_parseable_and_keeps_only_requested_cmap_entries() {
        let subset = subset_font(ARK_PIXEL, ['B', 'A', 'B'], &FontSubsetOptions::new()).unwrap();
        assert!(subset.bytes().len() < ARK_PIXEL.len());
        assert_eq!(
            subset
                .cmap()
                .iter()
                .map(|entry| entry.character())
                .collect::<Vec<_>>(),
            ['A', 'B']
        );
        let face = Face::parse(subset.bytes(), 0).unwrap();
        assert_eq!(subset.glyph_count(), face.number_of_glyphs());
        assert!(subset.glyph_count() >= 3);
    }

    #[test]
    fn subset_rejects_invalid_options_before_native_execution() {
        assert_eq!(
            subset_font(ARK_PIXEL, [], &FontSubsetOptions::new()),
            Err(FontSubsetError::EmptyCharset)
        );
        assert_eq!(
            subset_font(
                ARK_PIXEL,
                ['A'],
                &FontSubsetOptions::new().with_variation(*b"wght", f32::NAN),
            ),
            Err(FontSubsetError::InvalidVariationValue)
        );
        assert!(matches!(
            subset_font(
                ARK_PIXEL,
                ['A'],
                &FontSubsetOptions::new().with_variation(*b"wght", 400.0),
            ),
            Err(FontSubsetError::UnknownVariationAxis(tag)) if tag == *b"wght"
        ));
    }
}
