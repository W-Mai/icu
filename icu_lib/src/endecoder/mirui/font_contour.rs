use mirx::font::GlyphSurfaceAsset;
use mirx::{Font, FontRepresentationKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnpackedGlyph {
    width: u32,
    height: u32,
    samples: Vec<u8>,
}

impl UnpackedGlyph {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn samples(&self) -> &[u8] {
        &self.samples
    }
}

/// Unpacks one raw glyph region into row-major eight-bit scalar samples.
pub fn unpack_glyph(
    font: &Font,
    representation_index: usize,
    glyph_index: usize,
) -> Option<UnpackedGlyph> {
    let representation = font.representation(representation_index)?;
    let bits = match representation.metadata().kind() {
        FontRepresentationKind::Coverage { bits }
        | FontRepresentationKind::SignedDistance { bits, .. } => bits,
        _ => return None,
    };
    let GlyphSurfaceAsset::Raw { glyphs, .. } =
        font.surface(usize::from(representation.surface_index()))?
    else {
        return None;
    };
    let raster = glyphs.get(glyph_index)?;
    let region = raster.region();
    let plane = raster.storage().plane(0)?;
    let stride = u64::from(plane.memory().stride());
    let bytes = plane.bytes();
    let count = usize::try_from(u64::from(region.width()) * u64::from(region.height())).ok()?;
    let mut samples = Vec::with_capacity(count);
    let max = (1u16 << bits) - 1;
    for y in 0..region.height() {
        for x in 0..region.width() {
            let bit = (u64::from(region.y()) + u64::from(y)) * stride * 8
                + (u64::from(region.x()) + u64::from(x)) * u64::from(bits);
            let byte = *bytes.get(usize::try_from(bit / 8).ok()?)?;
            let shift = 8 - bits - (bit % 8) as u8;
            let value = u16::from((byte >> shift) & max as u8);
            samples.push((u32::from(value) * 255 / u32::from(max)) as u8);
        }
    }
    Some(UnpackedGlyph {
        width: region.width(),
        height: region.height(),
        samples,
    })
}

/// Extracts deterministic approximate closed contours from one glyph region.
pub fn approximate_glyph_contour(
    font: &Font,
    representation_index: usize,
    glyph_index: usize,
) -> Option<Vec<mirx::PathCmd>> {
    let glyph = unpack_glyph(font, representation_index, glyph_index)?;
    let width = usize::try_from(glyph.width).ok()?;
    let height = usize::try_from(glyph.height).ok()?;
    type Point = (i32, i32);
    let mut segments = std::collections::BTreeSet::<(Point, Point)>::new();

    let sample = |x: isize, y: isize| -> u8 {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
            0
        } else {
            glyph.samples[y as usize * width + x as usize]
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

    for y in 0..=height {
        for x in 0..=width {
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
                7 | 8 => &[(2, 3)],
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
        let to_point = |(x, y): Point| {
            mirx::Point::new(
                mirx::Fixed::from_raw(x),
                mirx::Fixed::from_raw(height as i32 * 256 - y),
            )
        };
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
    use mirx::font::{
        FontAsset, GlyphMap, GlyphMetrics, LineMetrics, RawGlyphs, RepresentationAsset,
    };
    use mirx::image::SampleLayout;
    use mirx::{FontRepresentation, PayloadLimits};

    fn font(kind: FontRepresentation, layout: SampleLayout, size: u32, data: &[u8]) -> Font {
        let codepoints = ['A'];
        let map = GlyphMap::glyph_major(size, size, 1).unwrap();
        let glyphs = RawGlyphs::builder(map, layout).build(data).unwrap();
        let line = LineMetrics::new(
            mirx::Fixed::from_int(size as i32),
            mirx::Fixed::ZERO,
            mirx::Fixed::from_int(size as i32),
        )
        .unwrap();
        let metrics = [GlyphMetrics::new(
            mirx::Fixed::from_int(size as i32),
            mirx::Fixed::ZERO,
            mirx::Fixed::from_int(size as i32),
        )];
        Font::from_asset(
            FontAsset::new(
                &codepoints,
                &[RepresentationAsset::new(kind, 0, line, &metrics)],
                &[GlyphSurfaceAsset::raw(glyphs)],
            ),
            &PayloadLimits::HOST,
        )
        .unwrap()
    }

    #[test]
    fn unpacks_sub_byte_samples_msb_first() {
        let four = font(
            FontRepresentation::coverage(4, 2, 2).unwrap(),
            SampleLayout::A4,
            2,
            &[0x01, 0x2f],
        );
        assert_eq!(
            unpack_glyph(&four, 0, 0).unwrap().samples(),
            [0, 17, 34, 255]
        );
        let one = font(
            FontRepresentation::coverage(1, 2, 2).unwrap(),
            SampleLayout::A1,
            2,
            &[0b1000_0000, 0b1000_0000],
        );
        assert_eq!(
            unpack_glyph(&one, 0, 0).unwrap().samples(),
            [255, 0, 255, 0]
        );
    }

    #[test]
    fn sdf_uses_the_same_msb_first_layout() {
        let font = font(
            FontRepresentation::signed_distance(4, 1, 2, 1, 8, 2).unwrap(),
            SampleLayout::A4,
            2,
            &[0x12, 0x3f],
        );
        assert_eq!(
            unpack_glyph(&font, 0, 0).unwrap().samples(),
            [17, 34, 51, 255]
        );
    }

    #[test]
    fn rejects_invalid_representation_and_glyph_indices() {
        let font = font(
            FontRepresentation::coverage(8, 2, 4).unwrap(),
            SampleLayout::A8,
            2,
            &[0, 1, 2, 3],
        );
        assert!(unpack_glyph(&font, 1, 0).is_none());
        assert!(unpack_glyph(&font, 0, 1).is_none());
    }

    #[test]
    fn maps_top_image_rows_to_high_glyph_coordinates() {
        let mut data = vec![0; 25];
        data[1] = 255;
        let font = font(
            FontRepresentation::coverage(8, 5, 25).unwrap(),
            SampleLayout::A8,
            5,
            &data,
        );
        let contour = approximate_glyph_contour(&font, 0, 0).unwrap();
        let y_values = contour.iter().filter_map(|command| match command {
            mirx::PathCmd::MoveTo(point) | mirx::PathCmd::LineTo(point) => Some(point.y.raw()),
            _ => None,
        });
        assert!(y_values.into_iter().all(|y| y > 2 * 256));
    }

    #[test]
    fn extracts_disconnected_regions_deterministically() {
        let data = [
            0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0, 255, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0,
        ];
        let font = font(
            FontRepresentation::coverage(8, 5, 25).unwrap(),
            SampleLayout::A8,
            5,
            &data,
        );
        let first = approximate_glyph_contour(&font, 0, 0).unwrap();
        let second = approximate_glyph_contour(&font, 0, 0).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first
                .iter()
                .filter(|command| matches!(command, mirx::PathCmd::MoveTo(_)))
                .count(),
            2
        );
    }
}
