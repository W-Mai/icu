use mirx::{Font, FontChunkKind};

/// Unpacks one glyph atlas cell into MSB-first quantized scalar samples.
/// Returns `None` for invalid geometry, glyph index, or truncated data.
pub fn unpack_glyph_cell(font: &Font, glyph_index: usize) -> Option<Vec<u8>> {
    let size = usize::from(font.atlas.source_size);
    let bit_depth = usize::from(font.atlas.bit_depth);
    if size == 0 || bit_depth == 0 || bit_depth > 8 {
        return None;
    }
    if matches!(font.chunk_header.kind, FontChunkKind::Sdf) && !matches!(bit_depth, 4 | 8) {
        return None;
    }
    if matches!(font.chunk_header.kind, FontChunkKind::Grayscale)
        && !matches!(bit_depth, 1 | 2 | 4 | 8)
    {
        return None;
    }
    let pixels = size.checked_mul(size)?;
    let bits = pixels.checked_mul(bit_depth)?;
    let bytes = bits.div_ceil(8);
    if font.atlas.bytes_per_glyph as usize != bytes {
        return None;
    }
    if glyph_index >= usize::try_from(font.atlas.glyph_count).ok()? {
        return None;
    }
    let start = glyph_index.checked_mul(bytes)?;
    let end = start.checked_add(bytes)?;
    let packed = font.data.get(start..end)?;
    let max = (1u16 << bit_depth) - 1;
    let mut samples = Vec::with_capacity(pixels);
    for pixel in 0..pixels {
        let value = if matches!(font.chunk_header.kind, FontChunkKind::Sdf) && bit_depth == 4 {
            let byte = packed[pixel / 2];
            if pixel % 2 == 0 {
                u16::from(byte & 0x0f)
            } else {
                u16::from(byte >> 4)
            }
        } else if bit_depth == 8 {
            u16::from(packed[pixel])
        } else {
            let bit = pixel * bit_depth;
            let mut value = 0u16;
            for offset in 0..bit_depth {
                let bit_index = bit + offset;
                let byte = packed[bit_index / 8];
                value = (value << 1) | u16::from((byte >> (7 - bit_index % 8)) & 1);
            }
            value
        };
        samples.push((u32::from(value) * 255 / u32::from(max)) as u8);
    }
    Some(samples)
}

/// Extracts deterministic, approximate closed contours from one atlas glyph cell.
/// Atlas values are thresholded at the scalar midpoint and emitted as pixel-space paths.
pub fn approximate_glyph_contour(font: &Font, glyph_index: usize) -> Option<Vec<mirx::PathCmd>> {
    let size = usize::from(font.atlas.source_size);
    let samples = unpack_glyph_cell(font, glyph_index)?;
    type Point = (i32, i32);
    let mut segments = std::collections::BTreeSet::<(Point, Point)>::new();

    let sample = |x: isize, y: isize| -> u8 {
        if x < 0 || y < 0 || x as usize >= size || y as usize >= size {
            0
        } else {
            samples[y as usize * size + x as usize]
        }
    };
    let intersection = |x: i32, y: i32, edge: usize, a: u8, b: u8| -> Point {
        let t = if a == b {
            0.5
        } else {
            ((128.0 - f32::from(a)) / (f32::from(b) - f32::from(a))).clamp(0.0, 1.0)
        };
        let offset = (t * 256.0).round() as i32;
        match edge {
            0 => (x * 256 + offset, y * 256),
            1 => ((x + 1) * 256, y * 256 + offset),
            2 => (x * 256 + offset, (y + 1) * 256),
            _ => (x * 256, y * 256 + offset),
        }
    };
    let add_segment =
        |segments: &mut std::collections::BTreeSet<(Point, Point)>, a: Point, b: Point| {
            segments.insert(if a <= b { (a, b) } else { (b, a) });
        };

    for y in 0..=size {
        for x in 0..=size {
            let x = x as isize;
            let y = y as isize;
            let values = [
                sample(x - 1, y - 1),
                sample(x, y - 1),
                sample(x, y),
                sample(x - 1, y),
            ];
            let case_id = values.iter().enumerate().fold(0u8, |mask, (index, value)| {
                mask | (u8::from(*value >= 128) << index)
            });
            let x = x as i32;
            let y = y as i32;
            let edge_point = |edge: usize| {
                let (a, b) = match edge {
                    0 => (values[0], values[1]),
                    1 => (values[1], values[2]),
                    2 => (values[3], values[2]),
                    _ => (values[0], values[3]),
                };
                intersection(x, y, edge, a, b)
            };
            let center = values.iter().map(|value| u16::from(*value)).sum::<u16>() / 4;
            let pairs: &[(usize, usize)] = match case_id {
                0 | 15 => &[],
                1 => &[(0, 3)],
                2 => &[(0, 1)],
                3 => &[(1, 3)],
                4 => &[(1, 2)],
                5 if center >= 128 => &[(0, 3), (1, 2)],
                5 => &[(0, 1), (2, 3)],
                6 => &[(0, 2)],
                7 => &[(2, 3)],
                8 => &[(2, 3)],
                9 => &[(0, 2)],
                10 if center >= 128 => &[(0, 1), (2, 3)],
                10 => &[(0, 3), (1, 2)],
                11 => &[(1, 2)],
                12 => &[(1, 3)],
                13 => &[(0, 1)],
                14 => &[(0, 3)],
                _ => &[],
            };
            for &(a, b) in pairs {
                add_segment(&mut segments, edge_point(a), edge_point(b));
            }
        }
    }
    if segments.is_empty() {
        return Some(Vec::new());
    }

    let mut paths = Vec::new();
    while let Some(&(start, _)) = segments.iter().next() {
        let mut points = vec![start];
        let mut current = start;
        let mut closed = false;
        while let Some(&(a, b)) = segments
            .iter()
            .find(|(a, b)| *a == current || *b == current)
        {
            segments.remove(&(a, b));
            current = if a == current { b } else { a };
            if current == start {
                closed = true;
                break;
            }
            points.push(current);
        }
        if !closed || points.len() < 3 {
            continue;
        }
        let to_point =
            |(x, y): Point| mirx::Point::new(mirx::Fixed::from_raw(x), mirx::Fixed::from_raw(y));
        paths.push(mirx::PathCmd::MoveTo(to_point(points[0])));
        for point in points.into_iter().skip(1) {
            paths.push(mirx::PathCmd::LineTo(to_point(point)));
        }
        paths.push(mirx::PathCmd::Close);
    }
    Some(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font(kind: FontChunkKind, bit_depth: u8, data: Vec<u8>) -> Font {
        font_with_size(kind, bit_depth, 2, data)
    }

    fn font_with_size(kind: FontChunkKind, bit_depth: u8, size: u16, data: Vec<u8>) -> Font {
        Font {
            chunk_header: mirx::FontChunkHeader {
                kind,
                format: bit_depth,
                size,
            },
            atlas: mirx::AtlasHeader {
                version: mirx::SUPPORTED_VERSION,
                bit_depth,
                _pad0: 0,
                source_size: size,
                spread: 1,
                glyph_count: 1,
                metric_offset: mirx::HEADER_LEN as u32,
                data_offset: (mirx::HEADER_LEN + mirx::METRIC_LEN) as u32,
                bytes_per_glyph: data.len() as u32,
                ascender: 0,
                descender: 0,
                line_height: 0,
                _pad1: 0,
            },
            metrics: vec![mirx::GlyphMetric {
                codepoint: 65,
                advance: 2,
                bearing_x: 0,
                bearing_y: 0,
            }],
            data,
        }
    }

    #[test]
    fn unpacks_four_bit_samples_msb_first() {
        let f = font(FontChunkKind::Grayscale, 4, vec![0x01, 0x2f]);
        assert_eq!(unpack_glyph_cell(&f, 0), Some(vec![0, 17, 34, 255]));
    }

    #[test]
    fn unpacks_one_and_two_bit_samples_msb_first() {
        let one = font(FontChunkKind::Grayscale, 1, vec![0b1010_0000]);
        assert_eq!(unpack_glyph_cell(&one, 0), Some(vec![255, 0, 255, 0]));
        let two = font(FontChunkKind::Grayscale, 2, vec![0b00_01_10_11]);
        assert_eq!(unpack_glyph_cell(&two, 0), Some(vec![0, 85, 170, 255]));
    }

    #[test]
    fn unpacks_sdf_four_bit_samples_low_nibble_first() {
        let f = font(FontChunkKind::Sdf, 4, vec![0x21, 0xf3]);
        assert_eq!(unpack_glyph_cell(&f, 0), Some(vec![17, 34, 51, 255]));
    }

    #[test]
    fn rejects_invalid_index_and_short_data() {
        let f = font(FontChunkKind::Grayscale, 8, vec![0, 1, 2, 3]);
        assert_eq!(unpack_glyph_cell(&f, 1), None);
        let mut short = f;
        short.data.pop();
        assert_eq!(unpack_glyph_cell(&short, 0), None);
    }

    #[test]
    fn extracts_rectangle_and_disconnected_regions_deterministically() {
        let f = font_with_size(
            FontChunkKind::Grayscale,
            8,
            5,
            vec![
                0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0, 255, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0,
            ],
        );
        let first = approximate_glyph_contour(&f, 0).unwrap();
        let second = approximate_glyph_contour(&f, 0).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first
                .iter()
                .filter(|cmd| matches!(cmd, mirx::PathCmd::MoveTo(_)))
                .count(),
            2
        );
        assert!(first.iter().any(|cmd| matches!(cmd, mirx::PathCmd::Close)));
    }
}
