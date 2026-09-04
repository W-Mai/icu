use std::fmt;

use mirui::render::path::Path as MirPath;
use mirui::render::raster::{flatten_into, scanline_fill, FillRule};
use mirui::types::{Fixed, Point};
use mirx::font::{
    FontAsset, GlyphMap, GlyphMetrics, GlyphSurfaceAsset, LineMetrics, RawGlyphs,
    RepresentationAsset,
};
use mirx::image::SampleLayout;
use mirx::{Font, FontRepresentation, PayloadLimits};
use ttf_parser::{Face, OutlineBuilder};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontBakeKind {
    Coverage,
    SignedDistance,
}

pub struct FontBakeParams {
    pub kind: FontBakeKind,
    pub source_size: u16,
    pub bit_depth: u8,
    pub spread: u16,
    pub min_ppem: Option<u16>,
    pub max_ppem: Option<u16>,
    pub charset: Vec<char>,
}

impl FontBakeParams {
    pub fn ascii(source_size: u16, kind: FontBakeKind) -> Self {
        let charset = (0x20u32..=0x7e).filter_map(char::from_u32).collect();
        Self {
            kind,
            source_size,
            bit_depth: 4,
            spread: (source_size / 4).max(1),
            min_ppem: None,
            max_ppem: None,
            charset,
        }
    }

    pub fn size_range(&self) -> (u16, u16) {
        match self.kind {
            FontBakeKind::Coverage => (self.source_size, self.source_size),
            FontBakeKind::SignedDistance => (
                self.min_ppem
                    .unwrap_or_else(|| (self.source_size / 2).max(1)),
                self.max_ppem
                    .unwrap_or_else(|| self.source_size.saturating_mul(4)),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontBakeError {
    InvalidFont,
    EmptyCharset,
    InvalidSize,
    InvalidBitDepth,
    InvalidSpread,
    InvalidSizeRange,
    SizeOverflow,
    InvalidStorage,
    InvalidFace,
    IncompatibleCodepoints,
    MissingFont,
    Container,
}

impl fmt::Display for FontBakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidFont => "invalid font source",
            Self::EmptyCharset => "font charset is empty",
            Self::InvalidSize => "font source size must be nonzero",
            Self::InvalidBitDepth => "unsupported font sample bit depth",
            Self::InvalidSpread => "signed-distance spread must be nonzero",
            Self::InvalidSizeRange => "invalid signed-distance size range",
            Self::SizeOverflow => "font size exceeds MIRX limits",
            Self::InvalidStorage => "invalid glyph surface storage",
            Self::InvalidFace => "invalid MIRX font face",
            Self::IncompatibleCodepoints => "font faces use different codepoint tables",
            Self::MissingFont => "input contains no MIRX font face",
            Self::Container => "invalid MIRX container",
        })
    }
}

impl std::error::Error for FontBakeError {}

struct PathBuilder {
    path: MirPath,
    scale: f32,
    baseline: f32,
}

impl PathBuilder {
    fn new(scale: f32, baseline: f32) -> Self {
        Self {
            path: MirPath::new(),
            scale,
            baseline,
        }
    }

    fn finish(self) -> MirPath {
        self.path
    }

    fn map(&self, x: f32, y: f32) -> Point {
        Point {
            x: Fixed::from_f32(x * self.scale),
            y: Fixed::from_f32(self.baseline - y * self.scale),
        }
    }
}

impl OutlineBuilder for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(self.map(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(self.map(x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path.quad_to(self.map(x1, y1), self.map(x, y));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path
            .cubic_to(self.map(x1, y1), self.map(x2, y2), self.map(x, y));
    }

    fn close(&mut self) {
        self.path.close();
    }
}

fn sample_layout(bits: u8) -> Option<SampleLayout> {
    Some(match bits {
        1 => SampleLayout::A1,
        2 => SampleLayout::A2,
        4 => SampleLayout::A4,
        8 => SampleLayout::A8,
        _ => return None,
    })
}

fn fixed(value: f32) -> mirx::Fixed {
    let raw = (value * 256.0)
        .round()
        .clamp(i32::MIN as f32, i32::MAX as f32) as i32;
    mirx::Fixed::from_raw(raw)
}

fn rasterize_to_coverage(path: &MirPath, size: u16) -> Vec<u8> {
    let mut segments = Vec::new();
    flatten_into(&path.cmds, None, &mut segments);
    let extent = i32::from(size);
    let mut samples = vec![0u8; usize::from(size) * usize::from(size)];
    let mut accumulator = Vec::new();
    let mut crossings = Vec::new();
    scanline_fill(
        &segments,
        0,
        0,
        extent,
        extent,
        FillRule::NonZero,
        &mut accumulator,
        &mut crossings,
        |x, y, coverage| {
            if (0..extent).contains(&x) && (0..extent).contains(&y) {
                samples[(y * extent + x) as usize] =
                    (coverage * Fixed::from_int(255)).to_int().clamp(0, 255) as u8;
            }
        },
    );
    samples
}

fn euclidean_distance_transform(coverage: &[u8], size: u16, spread: u16) -> Vec<f32> {
    let extent = i32::from(size);
    let mut output = vec![0.0; coverage.len()];
    let cap = f32::from(spread);
    let cap_squared = cap * cap;

    for y in 0..extent {
        for x in 0..extent {
            let inside = coverage[(y * extent + x) as usize] >= 128;
            let mut best_squared = cap_squared + 1.0;
            let min_x = (x - cap as i32).max(0);
            let max_x = (x + cap as i32 + 1).min(extent);
            let min_y = (y - cap as i32).max(0);
            let max_y = (y + cap as i32 + 1).min(extent);
            for sample_y in min_y..max_y {
                for sample_x in min_x..max_x {
                    let sample_inside = coverage[(sample_y * extent + sample_x) as usize] >= 128;
                    if sample_inside == inside {
                        continue;
                    }
                    let dx = (sample_x - x) as f32;
                    let dy = (sample_y - y) as f32;
                    best_squared = best_squared.min(dx * dx + dy * dy);
                }
            }
            let distance = best_squared.sqrt().min(cap);
            output[(y * extent + x) as usize] = if inside { distance } else { -distance };
        }
    }
    output
}

fn quantize_distance(distance: &[f32], bits: u8, spread: f32) -> Vec<u8> {
    let max = if bits == 4 { 15.0 } else { 255.0 };
    let midpoint = max / 2.0;
    let scale = midpoint / spread;
    distance
        .iter()
        .map(|value| {
            (value.clamp(-spread, spread) * scale + midpoint)
                .round()
                .clamp(0.0, max) as u8
        })
        .collect()
}

fn quantize_coverage(coverage: &[u8], bits: u8) -> Vec<u8> {
    let max = (1u16 << bits) - 1;
    coverage
        .iter()
        .map(|value| ((u16::from(*value) * max + 127) / 255) as u8)
        .collect()
}

fn pack_rows(samples: &[u8], width: u16, height: u16, bits: u8) -> Option<Vec<u8>> {
    if !matches!(bits, 1 | 2 | 4 | 8)
        || samples.len() != usize::from(width).checked_mul(usize::from(height))?
    {
        return None;
    }
    let stride = usize::from(width)
        .checked_mul(usize::from(bits))?
        .div_ceil(8);
    let mut output = vec![0u8; stride.checked_mul(usize::from(height))?];
    let mask = (1u16 << bits) as u8 - 1;
    for y in 0..usize::from(height) {
        for x in 0..usize::from(width) {
            let bit = y * stride * 8 + x * usize::from(bits);
            let shift = 8 - bits - (bit % 8) as u8;
            output[bit / 8] |= (samples[y * usize::from(width) + x] & mask) << shift;
        }
    }
    Some(output)
}

pub fn bake_font(ttf_bytes: &[u8], params: &FontBakeParams) -> Result<Font, FontBakeError> {
    if params.source_size == 0 {
        return Err(FontBakeError::InvalidSize);
    }
    let layout = sample_layout(params.bit_depth).ok_or(FontBakeError::InvalidBitDepth)?;
    if params.kind == FontBakeKind::SignedDistance && params.spread == 0 {
        return Err(FontBakeError::InvalidSpread);
    }
    let face = Face::parse(ttf_bytes, 0).map_err(|_| FontBakeError::InvalidFont)?;
    let mut requested = params.charset.clone();
    requested.sort_unstable();
    requested.dedup();
    if requested.is_empty() {
        return Err(FontBakeError::EmptyCharset);
    }

    let units_per_em = f32::from(face.units_per_em());
    let scale = f32::from(params.source_size) / units_per_em;
    let baseline = (f32::from(params.source_size) * 0.8).round();
    let line = LineMetrics::new(
        fixed((f32::from(face.ascender()) * scale).max(0.0)),
        fixed((f32::from(face.descender()) * scale).min(0.0)),
        fixed((f32::from(face.height()) * scale).max(1.0)),
    )
    .map_err(|_| FontBakeError::InvalidFace)?;

    let mut codepoints = Vec::with_capacity(requested.len());
    let mut metrics = Vec::with_capacity(requested.len());
    let mut data = Vec::new();
    for character in requested {
        let Some(glyph_id) = face.glyph_index(character) else {
            continue;
        };
        let advance = face
            .glyph_hor_advance(glyph_id)
            .map_or(f32::from(params.source_size) / 2.0, |value| {
                f32::from(value) * scale
            });
        metrics.push(GlyphMetrics::new(
            fixed(advance),
            mirx::Fixed::ZERO,
            fixed(baseline),
        ));
        codepoints.push(character);

        let mut builder = PathBuilder::new(scale, baseline);
        let coverage = if face.outline_glyph(glyph_id, &mut builder).is_some() {
            rasterize_to_coverage(&builder.finish(), params.source_size)
        } else {
            vec![0; usize::from(params.source_size) * usize::from(params.source_size)]
        };
        let quantized = match params.kind {
            FontBakeKind::Coverage => quantize_coverage(&coverage, params.bit_depth),
            FontBakeKind::SignedDistance => quantize_distance(
                &euclidean_distance_transform(&coverage, params.source_size, params.spread),
                params.bit_depth,
                f32::from(params.spread),
            ),
        };
        data.extend(
            pack_rows(
                &quantized,
                params.source_size,
                params.source_size,
                params.bit_depth,
            )
            .ok_or(FontBakeError::SizeOverflow)?,
        );
    }
    if codepoints.is_empty() {
        return Err(FontBakeError::EmptyCharset);
    }

    let map = GlyphMap::glyph_major(
        u32::from(params.source_size),
        u32::from(params.source_size),
        codepoints.len(),
    )
    .map_err(|_| FontBakeError::SizeOverflow)?;
    let glyphs = RawGlyphs::builder(map, layout)
        .build(&data)
        .map_err(|_| FontBakeError::InvalidStorage)?;
    let decoded_bytes = u32::try_from(data.len()).map_err(|_| FontBakeError::SizeOverflow)?;
    let representation = match params.kind {
        FontBakeKind::Coverage => {
            FontRepresentation::coverage(params.bit_depth, params.source_size, decoded_bytes)
        }
        FontBakeKind::SignedDistance => {
            let (min_ppem, max_ppem) = params.size_range();
            FontRepresentation::signed_distance(
                params.bit_depth,
                params.spread,
                params.source_size,
                min_ppem,
                max_ppem,
                decoded_bytes,
            )
        }
    }
    .map_err(|_| FontBakeError::InvalidSizeRange)?;
    let representations = [RepresentationAsset::new(representation, 0, line, &metrics)];
    let surfaces = [GlyphSurfaceAsset::raw(glyphs)];
    Font::from_asset(
        FontAsset::new(&codepoints, &representations, &surfaces),
        &PayloadLimits::HOST,
    )
    .map_err(|_| FontBakeError::InvalidFace)
}

pub fn merge_fonts(fonts: &[Font]) -> Result<Font, FontBakeError> {
    let first = fonts.first().ok_or(FontBakeError::MissingFont)?;
    if fonts
        .iter()
        .skip(1)
        .any(|font| font.codepoints() != first.codepoints())
    {
        return Err(FontBakeError::IncompatibleCodepoints);
    }

    let surface_count = fonts.iter().try_fold(0usize, |count, font| {
        count
            .checked_add(font.surface_count())
            .ok_or(FontBakeError::SizeOverflow)
    })?;
    let representation_count = fonts.iter().try_fold(0usize, |count, font| {
        count
            .checked_add(font.representation_count())
            .ok_or(FontBakeError::SizeOverflow)
    })?;
    let mut surfaces = Vec::with_capacity(surface_count);
    let mut representations = Vec::with_capacity(representation_count);
    let mut maps = Vec::new();

    for font in fonts {
        let surface_base = surfaces.len();
        for index in 0..font.surface_count() {
            surfaces.push(font.surface(index).ok_or(FontBakeError::InvalidFace)?);
        }
        for index in 0..font.representation_count() {
            let source = font
                .representation(index)
                .ok_or(FontBakeError::InvalidFace)?;
            let surface = surface_base
                .checked_add(usize::from(source.surface_index()))
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(FontBakeError::SizeOverflow)?;
            let mut representation = RepresentationAsset::new(
                source.metadata(),
                surface,
                source.line_metrics(),
                source.metrics(),
            );
            if source.map_index().is_some() {
                let map = font
                    .surface(usize::from(source.surface_index()))
                    .ok_or(FontBakeError::InvalidFace)?
                    .map();
                let map_index =
                    u32::try_from(maps.len()).map_err(|_| FontBakeError::SizeOverflow)?;
                maps.push(map);
                representation = representation.with_map(map_index);
            }
            representations.push(representation);
        }
    }

    Font::from_asset(
        FontAsset::new(first.codepoints(), &representations, &surfaces).with_maps(&maps),
        &PayloadLimits::HOST,
    )
    .map_err(|_| FontBakeError::InvalidFace)
}

pub fn merge_font_chunks(inputs: &[Vec<u8>]) -> Result<Vec<u8>, FontBakeError> {
    let mut fonts = Vec::new();
    for input in inputs {
        let reader = mirx::Reader::open(input).map_err(|_| FontBakeError::Container)?;
        for chunk in reader
            .chunks()
            .filter(|chunk| chunk.chunk_type() == mirx::ChunkType::FONT)
        {
            fonts.push(Font::decode(chunk.payload()).map_err(|_| FontBakeError::InvalidFace)?);
        }
    }
    let font = merge_fonts(&fonts)?;
    let payload = font.encode().map_err(|_| FontBakeError::InvalidFace)?;
    Ok(mirx::encode_chunk_generic(
        mirx::chunk_type::FONT,
        mirx::ChunkEntry::FLAG_CRITICAL,
        &payload,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_test_ttf() -> Option<Vec<u8>> {
        for path in [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ] {
            if let Ok(data) = std::fs::read(path) {
                if data.len() >= 4
                    && matches!(&data[..4], [0x00, 0x01, 0x00, 0x00] | b"OTTO" | b"ttcf")
                {
                    return Some(data);
                }
            }
        }
        None
    }

    fn params(kind: FontBakeKind, source_size: u16, bit_depth: u8) -> FontBakeParams {
        FontBakeParams {
            kind,
            source_size,
            bit_depth,
            spread: 4,
            min_ppem: None,
            max_ppem: None,
            charset: vec!['A', 'B', 'C'],
        }
    }

    #[test]
    fn row_packing_does_not_carry_bits_across_rows() {
        assert_eq!(
            pack_rows(&[1, 0, 1, 0, 1, 0], 3, 2, 1),
            Some(vec![0b1010_0000, 0b0100_0000])
        );
        assert_eq!(
            pack_rows(&[1, 2, 3, 4, 5, 6], 3, 2, 4),
            Some(vec![0x12, 0x30, 0x45, 0x60])
        );
    }

    #[test]
    fn bake_sdf_writes_one_sectioned_face() {
        let Some(data) = load_test_ttf() else {
            return;
        };
        let font = bake_font(&data, &params(FontBakeKind::SignedDistance, 16, 4)).unwrap();
        assert_eq!(font.codepoints(), ['A', 'B', 'C']);
        assert_eq!(font.representation_count(), 1);
        assert_eq!(font.surface_count(), 1);
        let representation = font.representation(0).unwrap().metadata();
        assert!(matches!(
            representation.kind(),
            mirx::FontRepresentationKind::SignedDistance { bits: 4, spread: 4 }
        ));
        assert_eq!(
            (representation.min_ppem(), representation.max_ppem()),
            (8, 64)
        );
        let payload = font.encode().unwrap();
        let decoded = Font::decode(&payload).unwrap();
        assert_eq!(decoded, font);
    }

    #[test]
    fn bake_coverage_uses_a_fixed_size_representation() {
        let Some(data) = load_test_ttf() else {
            return;
        };
        let font = bake_font(&data, &params(FontBakeKind::Coverage, 13, 4)).unwrap();
        let representation = font.representation(0).unwrap().metadata();
        assert!(matches!(
            representation.kind(),
            mirx::FontRepresentationKind::Coverage { bits: 4 }
        ));
        assert_eq!(
            (representation.min_ppem(), representation.max_ppem()),
            (13, 13)
        );
        assert_eq!(font.surface(0).unwrap().data().len(), 7 * 13 * 3);
    }

    #[test]
    fn merge_two_representations_produces_one_font_chunk() {
        let Some(data) = load_test_ttf() else {
            return;
        };
        let sdf = bake_font(&data, &params(FontBakeKind::SignedDistance, 16, 4)).unwrap();
        let coverage = bake_font(&data, &params(FontBakeKind::Coverage, 12, 4)).unwrap();
        let inputs = [sdf, coverage]
            .iter()
            .map(|font| {
                mirx::encode_chunk_generic(
                    mirx::chunk_type::FONT,
                    mirx::ChunkEntry::FLAG_CRITICAL,
                    &font.encode().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let merged = merge_font_chunks(&inputs).unwrap();
        let reader = mirx::Reader::open(&merged).unwrap();
        let faces = reader
            .chunks()
            .filter(|chunk| chunk.chunk_type() == mirx::ChunkType::FONT)
            .collect::<Vec<_>>();
        assert_eq!(faces.len(), 1);
        let font = Font::decode(faces[0].payload()).unwrap();
        assert_eq!(font.representation_count(), 2);
        assert_eq!(font.surface_count(), 2);
    }
}
