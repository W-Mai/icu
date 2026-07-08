use mirui::render::path::Path as MirPath;
use mirui::render::raster::{FillRule, flatten_into, scanline_fill};
use mirui::types::{Fixed, Point};
use mirx::{AtlasHeader, Font, FontChunkHeader, FontChunkKind, GlyphMetric};
use ttf_parser::{Face, OutlineBuilder};

pub struct FontBakeParams {
    pub kind: FontChunkKind,
    pub source_size: u16,
    pub bit_depth: u8,
    pub spread: u16,
    pub charset: Vec<char>,
}

impl FontBakeParams {
    pub fn ascii(source_size: u16, kind: FontChunkKind) -> Self {
        let charset: Vec<char> = (0x20u32..=0x7E).filter_map(char::from_u32).collect();
        let bit_depth = match kind {
            FontChunkKind::Sdf => 4,
            FontChunkKind::Grayscale => 4,
        };
        let spread = (source_size / 4).max(1);
        Self {
            kind,
            source_size,
            bit_depth,
            spread,
            charset,
        }
    }
}

struct PathBuilder {
    path: MirPath,
    scale: f32,
    cell_size: f32,
}

impl PathBuilder {
    fn new(scale: f32, cell_size: f32) -> Self {
        Self {
            path: MirPath::new(),
            scale,
            cell_size,
        }
    }

    fn finish(self) -> MirPath {
        self.path
    }

    fn map(&self, x: f32, y: f32) -> Point {
        let baseline = (self.cell_size * 0.8).round();
        Point {
            x: Fixed::from_f32(x * self.scale),
            y: Fixed::from_f32(baseline - y * self.scale),
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

fn bytes_per_glyph(size: u16, bit_depth: u8) -> usize {
    let pixels = size as usize * size as usize;
    (pixels * bit_depth as usize).div_ceil(8)
}

fn rasterize_to_coverage(path: &MirPath, size: u16) -> Vec<u8> {
    let mut segs = Vec::new();
    flatten_into(&path.cmds[..], None, &mut segs);
    let n = size as i32;
    let mut buf = vec![0u8; (n * n) as usize];
    let mut acc = Vec::new();
    let mut crossings = Vec::new();
    scanline_fill(
        &segs,
        0,
        0,
        n,
        n,
        FillRule::NonZero,
        &mut acc,
        &mut crossings,
        |x, y, cov| {
            if (0..n).contains(&x) && (0..n).contains(&y) {
                let v = (cov * Fixed::from_int(255)).to_int().clamp(0, 255) as u8;
                buf[(y * n + x) as usize] = v;
            }
        },
    );
    buf
}

fn euclidean_distance_transform(cov: &[u8], size: u16, spread: u16) -> Vec<f32> {
    let n = size as i32;
    let mut out = vec![0f32; cov.len()];
    let cap = spread as f32;
    let cap2 = cap * cap;

    for y in 0..n {
        for x in 0..n {
            let inside = cov[(y * n + x) as usize] >= 128;
            let mut best2 = cap2 + 1.0;
            let lo_x = (x - cap as i32).max(0);
            let hi_x = (x + cap as i32 + 1).min(n);
            let lo_y = (y - cap as i32).max(0);
            let hi_y = (y + cap as i32 + 1).min(n);
            for sy in lo_y..hi_y {
                for sx in lo_x..hi_x {
                    let other_inside = cov[(sy * n + sx) as usize] >= 128;
                    if other_inside == inside {
                        continue;
                    }
                    let dx = (sx - x) as f32;
                    let dy = (sy - y) as f32;
                    let d2 = dx * dx + dy * dy;
                    if d2 < best2 {
                        best2 = d2;
                    }
                }
            }
            let d = best2.sqrt().min(cap);
            out[(y * n + x) as usize] = if inside { d } else { -d };
        }
    }
    out
}

fn quantize(signed: &[f32], bit_depth: u8, spread: f32) -> Vec<u8> {
    let max_q = if bit_depth == 4 { 15.0 } else { 255.0 };
    let zero = max_q / 2.0;
    let scale = zero / spread;
    let bytes_n = if bit_depth == 4 {
        signed.len().div_ceil(2)
    } else {
        signed.len()
    };
    let mut out = vec![0u8; bytes_n];
    for (i, &d) in signed.iter().enumerate() {
        let q = (d.clamp(-spread, spread) * scale + zero)
            .round()
            .clamp(0.0, max_q) as u8;
        if bit_depth == 4 {
            let byte_idx = i >> 1;
            if i & 1 == 0 {
                out[byte_idx] = (out[byte_idx] & 0xF0) | (q & 0x0F);
            } else {
                out[byte_idx] = (out[byte_idx] & 0x0F) | ((q & 0x0F) << 4);
            }
        } else {
            out[i] = q;
        }
    }
    out
}

fn pack_coverage(coverage: &[u8], bpp: u8) -> Vec<u8> {
    let max_q = (1u16 << bpp) - 1;
    let total_bits = coverage.len() * bpp as usize;
    let mut out = vec![0u8; total_bits.div_ceil(8)];
    let mut bit_pos = 0usize;
    for &cov in coverage {
        let q = ((cov as u16 * max_q + 127) / 255) & max_q;
        let byte_idx = bit_pos / 8;
        let bit_off = bit_pos % 8;
        let shift = 16 - bit_off - bpp as usize;
        let placed = (q as u32) << shift;
        out[byte_idx] |= (placed >> 8) as u8;
        if byte_idx + 1 < out.len() {
            out[byte_idx + 1] |= placed as u8;
        }
        bit_pos += bpp as usize;
    }
    out
}

pub fn bake_font(ttf_bytes: &[u8], params: &FontBakeParams) -> Option<Font> {
    let face = Face::parse(ttf_bytes, 0).ok()?;
    let mut chars = params.charset.clone();
    chars.sort();
    chars.dedup();
    if chars.is_empty() {
        return None;
    }

    let units_per_em = face.units_per_em() as f32;
    let scale = params.source_size as f32 / units_per_em;
    let ascender = (face.ascender() as f32 * scale).round() as i32;
    let descender = (face.descender() as f32 * scale).round() as i32;
    let line_height = (face.height() as f32 * scale).round() as i32;
    let bpg = bytes_per_glyph(params.source_size, params.bit_depth);

    let mut metrics: Vec<GlyphMetric> = Vec::new();
    let mut data: Vec<u8> = Vec::new();

    for ch in &chars {
        let gid = match face.glyph_index(*ch) {
            Some(g) => g,
            None => continue,
        };
        let mut builder = PathBuilder::new(scale, params.source_size as f32);
        let bbox = match face.outline_glyph(gid, &mut builder) {
            Some(b) => b,
            None => {
                metrics.push(GlyphMetric {
                    codepoint: *ch as u32,
                    advance: face
                        .glyph_hor_advance(gid)
                        .map(|a| (a as f32 * scale).round() as u16)
                        .unwrap_or(params.source_size / 2),
                    bearing_x: 0,
                    bearing_y: 0,
                });
                data.extend(std::iter::repeat_n(0u8, bpg));
                continue;
            }
        };
        let path = builder.finish();
        let coverage = rasterize_to_coverage(&path, params.source_size);
        let packed = match params.kind {
            FontChunkKind::Sdf => {
                let signed = euclidean_distance_transform(&coverage, params.source_size, params.spread);
                quantize(&signed, params.bit_depth, params.spread as f32)
            }
            FontChunkKind::Grayscale => pack_coverage(&coverage, params.bit_depth),
        };
        debug_assert_eq!(packed.len(), bpg);
        data.extend(packed);

        let advance = face
            .glyph_hor_advance(gid)
            .map(|a| (a as f32 * scale).round() as u16)
            .unwrap_or(params.source_size);
        let bearing_x = (bbox.x_min as f32 * scale).round() as i32;
        let bearing_y = (bbox.y_max as f32 * scale).round() as i32;
        metrics.push(GlyphMetric {
            codepoint: *ch as u32,
            advance,
            bearing_x: bearing_x.clamp(-128, 127) as i8,
            bearing_y: bearing_y.clamp(-128, 127) as i8,
        });
    }

    metrics.sort_by_key(|m| m.codepoint);

    let body_spread = match params.kind {
        FontChunkKind::Sdf => params.spread,
        FontChunkKind::Grayscale => 0,
    };

    Some(Font {
        chunk_header: FontChunkHeader {
            kind: params.kind,
            format: params.bit_depth,
            size: params.source_size,
        },
        atlas: AtlasHeader {
            version: mirx::SUPPORTED_VERSION,
            bit_depth: params.bit_depth,
            _pad0: 0,
            source_size: params.source_size,
            spread: body_spread,
            glyph_count: metrics.len() as u32,
            metric_offset: mirx::HEADER_LEN as u32,
            data_offset: (mirx::HEADER_LEN + metrics.len() * mirx::METRIC_LEN) as u32,
            bytes_per_glyph: bpg as u32,
            ascender: ascender.max(0).min(u16::MAX as i32) as u16,
            descender: descender.unsigned_abs().min(u16::MAX as u32) as u16,
            line_height: line_height.max(0).min(u16::MAX as i32) as u16,
            _pad1: 0,
        },
        metrics,
        data,
    })
}

pub fn merge_font_chunks(inputs: &[Vec<u8>]) -> Vec<u8> {
    let mut chunks: Vec<(u16, u16, &[u8])> = Vec::new();
    for input in inputs {
        let parsed = match mirx::parse(input) {
            Ok(mirx::MirxFile::Chunk(file)) => file,
            _ => continue,
        };
        for entry in &parsed.entries {
            if entry.chunk_type != mirx::chunk_type::FONT {
                continue;
            }
            let start = entry.chunk_offset as usize;
            let end = match start.checked_add(entry.chunk_size as usize) {
                Some(e) => e,
                None => continue,
            };
            let payload = match input.get(start..end) {
                Some(p) => p,
                None => continue,
            };
            chunks.push((
                mirx::chunk_type::FONT,
                mirx::ChunkEntry::FLAG_CRITICAL,
                payload,
            ));
        }
    }
    mirx::encode_chunks(&chunks.iter().map(|(t, f, p)| (*t, *f, *p)).collect::<Vec<_>>())
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
                    && matches!(
                        &data[..4],
                        [0x00, 0x01, 0x00, 0x00] | b"OTTO" | b"ttcf"
                    )
                {
                    return Some(data);
                }
            }
        }
        None
    }

    #[test]
    fn bake_sdf_returns_font() {
        let data = match load_test_ttf() {
            Some(d) => d,
            None => {
                eprintln!("skip: no test TTF");
                return;
            }
        };
        let params = FontBakeParams {
            kind: FontChunkKind::Sdf,
            source_size: 16,
            bit_depth: 4,
            spread: 4,
            charset: vec!['A', 'B', 'C'],
        };
        let font = bake_font(&data, &params).expect("bake should succeed");
        assert_eq!(font.chunk_header.kind, FontChunkKind::Sdf);
        assert_eq!(font.atlas.source_size, 16);
        assert_eq!(font.metrics.len(), 3);
        assert!(!font.data.is_empty());
        let payload = font.encode();
        let back = mirx::Font::decode(&payload).expect("round-trip");
        assert_eq!(back.metrics.len(), 3);
    }

    #[test]
    fn bake_gray_returns_font() {
        let data = match load_test_ttf() {
            Some(d) => d,
            None => {
                eprintln!("skip: no test TTF");
                return;
            }
        };
        let params = FontBakeParams {
            kind: FontChunkKind::Grayscale,
            source_size: 12,
            bit_depth: 4,
            spread: 0,
            charset: vec!['A', 'B'],
        };
        let font = bake_font(&data, &params).expect("bake should succeed");
        assert_eq!(font.chunk_header.kind, FontChunkKind::Grayscale);
        assert_eq!(font.metrics.len(), 2);
    }

    #[test]
    fn merge_two_font_chunks_roundtrips() {
        let data = match load_test_ttf() {
            Some(d) => d,
            None => {
                eprintln!("skip: no test TTF");
                return;
            }
        };
        let p1 = FontBakeParams {
            kind: FontChunkKind::Sdf,
            source_size: 16,
            bit_depth: 4,
            spread: 4,
            charset: vec!['A'],
        };
        let p2 = FontBakeParams {
            kind: FontChunkKind::Grayscale,
            source_size: 12,
            bit_depth: 4,
            spread: 0,
            charset: vec!['A'],
        };
        let f1 = bake_font(&data, &p1).unwrap();
        let f2 = bake_font(&data, &p2).unwrap();
        let bytes1 = mirx::encode_chunk_generic(
            mirx::chunk_type::FONT,
            mirx::ChunkEntry::FLAG_CRITICAL,
            &f1.encode(),
        );
        let bytes2 = mirx::encode_chunk_generic(
            mirx::chunk_type::FONT,
            mirx::ChunkEntry::FLAG_CRITICAL,
            &f2.encode(),
        );
        let merged = merge_font_chunks(&[bytes1, bytes2]);
        let parsed = mirx::parse(&merged).expect("merged should parse");
        match parsed {
            mirx::MirxFile::Chunk(file) => {
                let font_payloads: Vec<_> = file
                    .entries
                    .iter()
                    .filter(|e| e.chunk_type == mirx::chunk_type::FONT)
                    .collect();
                assert_eq!(font_payloads.len(), 2);
            }
            _ => panic!("expected chunk file"),
        }
    }
}
