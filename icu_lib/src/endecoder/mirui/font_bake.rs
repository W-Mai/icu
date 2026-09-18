use std::fmt;

#[cfg(not(target_arch = "wasm32"))]
use mirui::render::path::Path as MirPath;
#[cfg(not(target_arch = "wasm32"))]
use mirui::render::raster::{flatten_into, scanline_fill, FillRule, LineSeg};
#[cfg(not(target_arch = "wasm32"))]
use mirui::types::{Fixed, Point};
#[cfg(not(target_arch = "wasm32"))]
use mirx::font::{
    CmapEntry, FontAdvanceSource, FontFace, FontRepresentation, GlyphId, GlyphMap,
    GlyphSurfaceAsset, RasterMetrics, RawGlyphs,
};
use mirx::font::{Font, FontAsset, RepresentationAsset};
#[cfg(not(target_arch = "wasm32"))]
use mirx::image::{AtlasMap, Region, SampleLayout};
use mirx::reader::PayloadLimits;
#[cfg(not(target_arch = "wasm32"))]
use ttf_parser::{Face, GlyphId as TtfGlyphId, OutlineBuilder};

#[cfg(not(target_arch = "wasm32"))]
use super::font_subset::{subset_font, FontSubsetError, FontSubsetOptions, FontVariation};

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
    pub face_index: u32,
    #[cfg(not(target_arch = "wasm32"))]
    pub variations: Vec<FontVariation>,
}

impl FontBakeParams {
    pub fn ascii(source_size: u16, kind: FontBakeKind) -> Self {
        let charset = (0x20u32..=0x7e).filter_map(char::from_u32).collect();
        Self {
            kind,
            source_size,
            bit_depth: match kind {
                FontBakeKind::Coverage => 4,
                FontBakeKind::SignedDistance => 8,
            },
            spread: (source_size / 4).max(1),
            min_ppem: None,
            max_ppem: None,
            charset,
            face_index: 0,
            #[cfg(not(target_arch = "wasm32"))]
            variations: Vec::new(),
        }
    }

    pub fn size_range(&self) -> (u16, u16) {
        match self.kind {
            FontBakeKind::Coverage => (self.source_size, self.source_size),
            FontBakeKind::SignedDistance => (
                self.min_ppem
                    .unwrap_or_else(|| (self.source_size / 2).max(1)),
                self.max_ppem
                    .unwrap_or_else(|| self.source_size.saturating_mul(2)),
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
    IncompatibleFaces,
    MissingFont,
    Container,
    Subset,
    UnsupportedPlatform,
    NativeOnly,
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
            Self::IncompatibleFaces => "font representations describe different faces",
            Self::MissingFont => "input contains no MIRX font face",
            Self::Container => "invalid MIRX container",
            Self::Subset => "font subsetting failed",
            Self::UnsupportedPlatform => "font baking is unavailable on this platform",
            Self::NativeOnly => "font baking requires a native target",
        })
    }
}

impl std::error::Error for FontBakeError {}

#[cfg(not(target_arch = "wasm32"))]
struct PathBuilder {
    path: MirPath,
    scale: f32,
    left: f32,
    top: f32,
}

#[cfg(not(target_arch = "wasm32"))]
impl PathBuilder {
    fn new(scale: f32, left: f32, top: f32) -> Self {
        Self {
            path: MirPath::new(),
            scale,
            left,
            top,
        }
    }

    fn finish(self) -> MirPath {
        self.path
    }

    fn map(&self, x: f32, y: f32) -> Point {
        Point {
            x: Fixed::from_f32(x * self.scale - self.left),
            y: Fixed::from_f32(self.top - y * self.scale),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct GlyphBitmap {
    width: u32,
    height: u32,
    samples: Vec<u8>,
    metrics: RasterMetrics,
}

#[cfg(not(target_arch = "wasm32"))]
fn sample_layout(bits: u8) -> Option<SampleLayout> {
    Some(match bits {
        1 => SampleLayout::A1,
        2 => SampleLayout::A2,
        4 => SampleLayout::A4,
        8 => SampleLayout::A8,
        _ => return None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn fixed(value: f32) -> mirx::types::Fixed {
    Fixed::from_f32(value).into()
}

#[cfg(not(target_arch = "wasm32"))]
fn checked_area(width: u32, height: u32) -> Result<usize, FontBakeError> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(FontBakeError::SizeOverflow)
}

#[cfg(not(target_arch = "wasm32"))]
fn rasterize_to_coverage(
    segments: &[LineSeg],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, FontBakeError> {
    let mut samples = vec![0u8; checked_area(width, height)?];
    let mut accumulator = Vec::new();
    let mut crossings = Vec::new();
    scanline_fill(
        segments,
        0,
        0,
        width as i32,
        height as i32,
        FillRule::NonZero,
        &mut accumulator,
        &mut crossings,
        |x, y, coverage| {
            if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
                samples[y as usize * width as usize + x as usize] =
                    (coverage * Fixed::from_int(255)).to_int().clamp(0, 255) as u8;
            }
        },
    );
    Ok(samples)
}

#[cfg(not(target_arch = "wasm32"))]
fn point_segment_distance_squared(x: f32, y: f32, segment: &LineSeg) -> f32 {
    let x1 = segment.p1.x.to_f32();
    let y1 = segment.p1.y.to_f32();
    let dx = segment.p2.x.to_f32() - x1;
    let dy = segment.p2.y.to_f32() - y1;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f32::EPSILON {
        return (x - x1) * (x - x1) + (y - y1) * (y - y1);
    }
    let t = (((x - x1) * dx + (y - y1) * dy) / length_squared).clamp(0.0, 1.0);
    let nearest_x = x1 + t * dx;
    let nearest_y = y1 + t * dy;
    (x - nearest_x) * (x - nearest_x) + (y - nearest_y) * (y - nearest_y)
}

#[cfg(not(target_arch = "wasm32"))]
fn contour_sample(segments: &[LineSeg], x: f32, y: f32) -> (f32, bool) {
    debug_assert!(!segments.is_empty());
    let mut distance_squared = f32::INFINITY;
    let mut winding = 0i32;
    for segment in segments {
        distance_squared = distance_squared.min(point_segment_distance_squared(x, y, segment));
        let x1 = segment.p1.x.to_f32();
        let y1 = segment.p1.y.to_f32();
        let x2 = segment.p2.x.to_f32();
        let y2 = segment.p2.y.to_f32();
        let side = (x2 - x1) * (y - y1) - (x - x1) * (y2 - y1);
        if y1 <= y {
            if y2 > y && side > 0.0 {
                winding += 1;
            }
        } else if y2 <= y && side < 0.0 {
            winding -= 1;
        }
    }
    (distance_squared.sqrt(), winding != 0)
}

#[cfg(not(target_arch = "wasm32"))]
fn signed_distance(
    segments: &[LineSeg],
    width: u32,
    height: u32,
    spread: f32,
) -> Result<Vec<u8>, FontBakeError> {
    let mut output = vec![0; checked_area(width, height)?];
    if segments.is_empty() {
        return Ok(output);
    }
    for y in 0..height {
        let sample_y = y as f32 + 0.5;
        for x in 0..width {
            let sample_x = x as f32 + 0.5;
            let (distance, inside) = contour_sample(segments, sample_x, sample_y);
            let distance = distance.min(spread);
            let signed = if inside { distance } else { -distance };
            output[y as usize * width as usize + x as usize] = ((signed / spread * 127.5) + 127.5)
                .round()
                .clamp(0.0, 255.0)
                as u8;
        }
    }
    Ok(output)
}

#[cfg(not(target_arch = "wasm32"))]
fn quantize_coverage(coverage: &[u8], bits: u8) -> Vec<u8> {
    let max = (1u16 << bits) - 1;
    coverage
        .iter()
        .map(|value| ((u16::from(*value) * max + 127) / 255) as u8)
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn pack_rows(samples: &[u8], width: u32, height: u32, bits: u8) -> Option<Vec<u8>> {
    if !matches!(bits, 1 | 2 | 4 | 8)
        || samples.len() != (width as usize).checked_mul(height as usize)?
    {
        return None;
    }
    let stride = (width as usize).checked_mul(bits as usize)?.div_ceil(8);
    let mut output = vec![0u8; stride.checked_mul(height as usize)?];
    let mask = ((1u16 << bits) - 1) as u8;
    for y in 0..height as usize {
        for x in 0..width as usize {
            let bit = y * stride * 8 + x * bits as usize;
            let shift = 8 - bits - (bit % 8) as u8;
            output[bit / 8] |= (samples[y * width as usize + x] & mask) << shift;
        }
    }
    Some(output)
}

#[cfg(not(target_arch = "wasm32"))]
fn rasterize_glyph(
    face: &Face<'_>,
    glyph_id: TtfGlyphId,
    params: &FontBakeParams,
) -> Result<GlyphBitmap, FontBakeError> {
    let Some(bounds) = face.glyph_bounding_box(glyph_id) else {
        return Ok(GlyphBitmap {
            width: 0,
            height: 0,
            samples: Vec::new(),
            metrics: RasterMetrics::default(),
        });
    };
    let scale = f32::from(params.source_size) / f32::from(face.units_per_em());
    let padding = match params.kind {
        FontBakeKind::Coverage => 1i64,
        FontBakeKind::SignedDistance => i64::from(params.spread) + 1,
    };
    let left = ((f32::from(bounds.x_min) * scale).floor() as i64)
        .checked_sub(padding)
        .ok_or(FontBakeError::SizeOverflow)?;
    let right = ((f32::from(bounds.x_max) * scale).ceil() as i64)
        .checked_add(padding)
        .ok_or(FontBakeError::SizeOverflow)?;
    let top = ((f32::from(bounds.y_max) * scale).ceil() as i64)
        .checked_add(padding)
        .ok_or(FontBakeError::SizeOverflow)?;
    let bottom = ((f32::from(bounds.y_min) * scale).floor() as i64)
        .checked_sub(padding)
        .ok_or(FontBakeError::SizeOverflow)?;
    let width = u32::try_from(right.checked_sub(left).ok_or(FontBakeError::SizeOverflow)?)
        .map_err(|_| FontBakeError::SizeOverflow)?;
    let height = u32::try_from(top.checked_sub(bottom).ok_or(FontBakeError::SizeOverflow)?)
        .map_err(|_| FontBakeError::SizeOverflow)?;
    let min_metric = i64::from(Fixed::MIN.to_int());
    let max_metric = i64::from(Fixed::MAX.to_int());
    if left < min_metric
        || left > max_metric
        || top < min_metric
        || top > max_metric
        || width > max_metric as u32
        || height > max_metric as u32
    {
        return Err(FontBakeError::SizeOverflow);
    }
    let mut builder = PathBuilder::new(scale, left as f32, top as f32);
    let path = if face.outline_glyph(glyph_id, &mut builder).is_some() {
        builder.finish()
    } else {
        MirPath::new()
    };
    let mut segments = Vec::new();
    flatten_into(path.commands(), None, &mut segments);
    let samples = match params.kind {
        FontBakeKind::Coverage => quantize_coverage(
            &rasterize_to_coverage(&segments, width, height)?,
            params.bit_depth,
        ),
        FontBakeKind::SignedDistance => {
            signed_distance(&segments, width, height, f32::from(params.spread))?
        }
    };
    Ok(GlyphBitmap {
        width,
        height,
        samples,
        metrics: RasterMetrics::new(fixed(left as f32), fixed(top as f32)),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn pack_atlas(glyphs: &[GlyphBitmap]) -> Result<(u32, u32, Vec<Region>, Vec<u8>), FontBakeError> {
    let max_width = glyphs.iter().map(|glyph| glyph.width).max().unwrap_or(0);
    let area = glyphs.iter().try_fold(0u64, |area, glyph| {
        let width = glyph
            .width
            .checked_add(1)
            .ok_or(FontBakeError::SizeOverflow)?;
        let height = glyph
            .height
            .checked_add(1)
            .ok_or(FontBakeError::SizeOverflow)?;
        let padded = u64::from(width)
            .checked_mul(u64::from(height))
            .ok_or(FontBakeError::SizeOverflow)?;
        area.checked_add(padded).ok_or(FontBakeError::SizeOverflow)
    })?;
    let estimated = (area as f64).sqrt().ceil() as u32;
    let width = estimated
        .max(max_width)
        .max(1)
        .checked_next_power_of_two()
        .ok_or(FontBakeError::SizeOverflow)?;
    let mut x = 0u32;
    let mut y = 0u32;
    let mut row_height = 0u32;
    let mut regions = Vec::with_capacity(glyphs.len());
    for glyph in glyphs {
        if glyph.width == 0 || glyph.height == 0 {
            regions.push(Region::new(0, 0, 0, 0).map_err(|_| FontBakeError::SizeOverflow)?);
            continue;
        }
        if x != 0 && x.checked_add(glyph.width).is_none_or(|right| right > width) {
            y = y
                .checked_add(row_height)
                .and_then(|value| value.checked_add(1))
                .ok_or(FontBakeError::SizeOverflow)?;
            x = 0;
            row_height = 0;
        }
        regions.push(
            Region::new(x, y, glyph.width, glyph.height)
                .map_err(|_| FontBakeError::SizeOverflow)?,
        );
        x = x
            .checked_add(glyph.width)
            .and_then(|value| value.checked_add(1))
            .ok_or(FontBakeError::SizeOverflow)?;
        row_height = row_height.max(glyph.height);
    }
    let height = y
        .checked_add(row_height)
        .ok_or(FontBakeError::SizeOverflow)?
        .max(1);
    let mut samples = vec![0; checked_area(width, height)?];
    for (glyph, region) in glyphs.iter().zip(&regions) {
        for row in 0..glyph.height as usize {
            let source =
                &glyph.samples[row * glyph.width as usize..(row + 1) * glyph.width as usize];
            let start = (region.y() as usize + row) * width as usize + region.x() as usize;
            samples[start..start + source.len()].copy_from_slice(source);
        }
    }
    Ok((width, height, regions, samples))
}

pub fn bake_font(ttf_bytes: &[u8], params: &FontBakeParams) -> Result<Font, FontBakeError> {
    bake_font_impl(ttf_bytes, params)
}

#[cfg(target_arch = "wasm32")]
fn bake_font_impl(_: &[u8], _: &FontBakeParams) -> Result<Font, FontBakeError> {
    Err(FontBakeError::NativeOnly)
}

#[cfg(not(target_arch = "wasm32"))]
fn bake_font_impl(ttf_bytes: &[u8], params: &FontBakeParams) -> Result<Font, FontBakeError> {
    if params.source_size == 0 {
        return Err(FontBakeError::InvalidSize);
    }
    if params.kind == FontBakeKind::SignedDistance {
        if params.bit_depth != 8 {
            return Err(FontBakeError::InvalidBitDepth);
        }
        if params.spread == 0 {
            return Err(FontBakeError::InvalidSpread);
        }
    }
    let layout = sample_layout(params.bit_depth).ok_or(FontBakeError::InvalidBitDepth)?;
    let mut options = FontSubsetOptions::new().with_face_index(params.face_index);
    for variation in &params.variations {
        options = options.with_variation(variation.tag(), variation.value());
    }
    let subset =
        subset_font(ttf_bytes, params.charset.iter().copied(), &options).map_err(|error| {
            match error {
                FontSubsetError::UnsupportedPlatform => FontBakeError::UnsupportedPlatform,
                _ => FontBakeError::Subset,
            }
        })?;
    let face = Face::parse(subset.bytes(), 0).map_err(|_| FontBakeError::InvalidFont)?;
    let glyph_count = subset.glyph_count();
    if glyph_count == 0 {
        return Err(FontBakeError::EmptyCharset);
    }
    let glyphs = (0..glyph_count)
        .map(|id| rasterize_glyph(&face, TtfGlyphId(id), params))
        .collect::<Result<Vec<_>, _>>()?;
    let (width, height, regions, samples) = pack_atlas(&glyphs)?;
    let data =
        pack_rows(&samples, width, height, params.bit_depth).ok_or(FontBakeError::SizeOverflow)?;
    let atlas =
        AtlasMap::new(width, height, &regions).map_err(|_| FontBakeError::InvalidStorage)?;
    let map = GlyphMap::atlas(atlas);
    let raw = RawGlyphs::builder(map, layout)
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
                8,
                params.spread,
                params.source_size,
                min_ppem,
                max_ppem,
                decoded_bytes,
            )
        }
    }
    .map_err(|_| FontBakeError::InvalidSizeRange)?;
    let face_record = FontFace::new(
        face.units_per_em(),
        GlyphId::NOTDEF,
        glyph_count,
        fixed(f32::from(face.ascender())),
        fixed(f32::from(face.descender())),
        fixed(f32::from(face.line_gap())),
    )
    .map_err(|_| FontBakeError::InvalidFace)?;
    let cmap = subset
        .cmap()
        .iter()
        .map(|entry| CmapEntry::new(entry.character(), GlyphId::new(entry.glyph_id())))
        .collect::<Vec<_>>();
    let raster_metrics = glyphs.iter().map(|glyph| glyph.metrics).collect::<Vec<_>>();
    let representations = [RepresentationAsset::new(representation, 0).with_atlas_map(0)];
    let surfaces = [GlyphSurfaceAsset::raw(raw)];
    Font::from_asset(
        FontAsset::new(
            face_record,
            &cmap,
            FontAdvanceSource::Shaping(subset.bytes()),
        )
        .with_rasters(&representations, &raster_metrics, &surfaces)
        .with_atlas_maps(&[atlas]),
        &PayloadLimits::HOST,
    )
    .map_err(|_| FontBakeError::InvalidFace)
}

pub fn merge_fonts(fonts: &[Font]) -> Result<Font, FontBakeError> {
    let first = fonts.first().ok_or(FontBakeError::MissingFont)?;
    if fonts.iter().skip(1).any(|font| {
        font.face() != first.face()
            || font.cmap() != first.cmap()
            || font.glyph_ids() != first.glyph_ids()
            || font.advance_source() != first.advance_source()
    }) {
        return Err(FontBakeError::IncompatibleFaces);
    }
    let glyph_count = usize::from(first.face().raster_count());
    let mut surfaces = Vec::new();
    let mut representations = Vec::new();
    let mut raster_metrics = Vec::new();
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
            let start = index
                .checked_mul(glyph_count)
                .ok_or(FontBakeError::SizeOverflow)?;
            raster_metrics.extend_from_slice(
                font.raster_metrics()
                    .get(start..start + glyph_count)
                    .ok_or(FontBakeError::InvalidFace)?,
            );
            let mut representation = RepresentationAsset::new(source.metadata(), surface);
            if source.atlas_map_index().is_some() {
                let map = font
                    .surface(usize::from(source.surface_index()))
                    .and_then(|surface| surface.map().atlas_map())
                    .ok_or(FontBakeError::InvalidFace)?;
                let map_index =
                    u32::try_from(maps.len()).map_err(|_| FontBakeError::SizeOverflow)?;
                maps.push(map);
                representation = representation.with_atlas_map(map_index);
            }
            representations.push(representation);
        }
    }
    let mut asset = FontAsset::new(first.face(), first.cmap(), first.advance_source())
        .with_rasters(&representations, &raster_metrics, &surfaces)
        .with_atlas_maps(&maps);
    if let Some(ids) = first.glyph_ids() {
        asset = asset.with_glyph_ids(ids);
    }
    Font::from_asset(asset, &PayloadLimits::HOST).map_err(|_| FontBakeError::InvalidFace)
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
    let mut document = mirx::Document::new_with_limits(PayloadLimits::HOST);
    let id = document
        .push_font_with_flags(&font, mirx::ChunkFlags::CRITICAL)
        .map_err(|_| FontBakeError::Container)?;
    document
        .set_primary(id)
        .map_err(|_| FontBakeError::Container)?;
    document
        .encode(&mirx::document::EncodeOptions::new())
        .map_err(|_| FontBakeError::Container)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    const ARK_PIXEL: &[u8] =
        include_bytes!("../../../../assets/ark-pixel-12px-monospaced-zh_cn.otf");

    fn params(kind: FontBakeKind, source_size: u16, bit_depth: u8) -> FontBakeParams {
        FontBakeParams {
            kind,
            source_size,
            bit_depth,
            spread: 4,
            min_ppem: None,
            max_ppem: None,
            charset: vec!['A', 'B', 'C'],
            face_index: 0,
            #[cfg(not(target_arch = "wasm32"))]
            variations: Vec::new(),
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
    fn atlas_packing_rejects_padded_dimension_overflow() {
        let glyph = GlyphBitmap {
            width: u32::MAX,
            height: 1,
            samples: Vec::new(),
            metrics: RasterMetrics::default(),
        };
        assert!(matches!(
            pack_atlas(&[glyph]),
            Err(FontBakeError::SizeOverflow)
        ));
    }

    fn line(x1: f32, y1: f32, x2: f32, y2: f32) -> LineSeg {
        LineSeg {
            p1: Point::new(Fixed::from_f32(x1), Fixed::from_f32(y1)),
            p2: Point::new(Fixed::from_f32(x2), Fixed::from_f32(y2)),
        }
    }

    #[test]
    fn segment_distance_preserves_subpixel_diagonals() {
        let distance = point_segment_distance_squared(1.0, 2.0, &line(0.0, 0.0, 4.0, 4.0));
        assert!((distance - 0.5).abs() < 0.001);
    }

    #[test]
    fn segment_distance_handles_degenerate_edges() {
        let distance = point_segment_distance_squared(5.0, 7.0, &line(2.0, 3.0, 2.0, 3.0));
        assert!((distance - 25.0).abs() < 0.001);
    }

    #[test]
    fn contour_distance_encodes_inside_and_outside() {
        let segments = [
            line(1.0, 1.0, 5.0, 1.0),
            line(5.0, 1.0, 5.0, 5.0),
            line(5.0, 5.0, 1.0, 5.0),
            line(1.0, 5.0, 1.0, 1.0),
        ];
        let field = signed_distance(&segments, 6, 6, 4.0).unwrap();
        assert!(field[2 * 6 + 2] > 128);
        assert!(field[0] < 128);
        assert_eq!(field[2 * 6], field[2]);
    }

    #[test]
    fn empty_contour_stays_clear() {
        assert_eq!(signed_distance(&[], 3, 2, 2.0).unwrap(), vec![0; 6]);
    }

    #[test]
    fn signed_distance_range_uses_explicit_physical_sizes() {
        let mut params = params(FontBakeKind::SignedDistance, 64, 8);
        params.min_ppem = Some(40);
        params.max_ppem = Some(128);

        assert_eq!(params.size_range(), (40, 128));
    }

    #[test]
    fn bake_sdf_keeps_subset_glyph_closure_in_a8_atlas() {
        let font = bake_font(ARK_PIXEL, &params(FontBakeKind::SignedDistance, 24, 8)).unwrap();
        let representation = font.representation(0).unwrap().metadata();
        assert!(matches!(
            representation.kind(),
            mirx::font::FontRepresentationKind::SignedDistance { bits: 8, spread: 4 }
        ));
        assert_eq!(
            (representation.min_ppem(), representation.max_ppem()),
            (12, 48)
        );
        let subset = subset_font(ARK_PIXEL, ['A', 'B', 'C'], &FontSubsetOptions::new()).unwrap();
        assert_eq!(font.face().raster_count(), subset.glyph_count());
        assert_eq!(
            font.surface(0).unwrap().map().len(),
            usize::from(subset.glyph_count())
        );
        assert_eq!(font.cmap().len(), 3);
        assert!(font.glyph_ids().is_none());
        assert!(matches!(
            font.advance_source(),
            FontAdvanceSource::Shaping(_)
        ));
    }

    #[test]
    fn bake_coverage_uses_variable_atlas_regions() {
        let font = bake_font(ARK_PIXEL, &params(FontBakeKind::Coverage, 13, 4)).unwrap();
        let surface = font.surface(0).unwrap();
        let map = surface.map();
        assert!(map.atlas_map().is_some());
        let nonempty = map
            .iter()
            .filter(|region| region.width() != 0 && region.height() != 0)
            .collect::<Vec<_>>();
        assert!(!nonempty.is_empty());
        assert!(nonempty.iter().all(|region| region.width() < map.width()));
    }

    #[test]
    fn merge_two_representations_produces_one_font_chunk() {
        let sdf = bake_font(ARK_PIXEL, &params(FontBakeKind::SignedDistance, 16, 8)).unwrap();
        let coverage = bake_font(ARK_PIXEL, &params(FontBakeKind::Coverage, 12, 4)).unwrap();
        let inputs = [sdf, coverage]
            .iter()
            .map(|font| {
                let mut document = mirx::Document::new_with_limits(PayloadLimits::HOST);
                let id = document
                    .push_font_with_flags(font, mirx::ChunkFlags::CRITICAL)
                    .unwrap();
                document.set_primary(id).unwrap();
                document
                    .encode(&mirx::document::EncodeOptions::new())
                    .unwrap()
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
