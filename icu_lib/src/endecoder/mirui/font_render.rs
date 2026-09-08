use image::imageops::overlay;
use image::RgbaImage;
use mirui::render::backends::sw::SwRenderer;
use mirui::render::canvas::Canvas;
use mirui::render::path::{Path, PathCmd};
use mirui::render::raster::FillRule;
use mirui::render::texture::{AlphaMode, ColorFormat, Texture};
use mirui::types::{Fixed, Rect};
use mirx::{
    FontRepresentationFallback, FontRepresentationKind, FontRepresentationRequest,
    FontRepresentations, Paint,
};

pub fn render_font_atlas(font: &mirx::Font) -> RgbaImage {
    let Some(representation) = font.representation(0) else {
        return RgbaImage::new(0, 0);
    };
    let cell = u32::from(representation.metadata().design_ppem());
    let gap = 1u32;
    let cols = (font.codepoints().len() as f64).sqrt().ceil() as u32;
    let cols = cols.max(1);
    let rows = (font.codepoints().len() as u32).div_ceil(cols);
    let grid_w = cols * cell + (cols + 1) * gap;
    let grid_h = rows * cell + (rows + 1) * gap;
    let mut image = RgbaImage::new(grid_w, grid_h);
    let color = mirx::Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    for (index, character) in font.codepoints().iter().copied().enumerate() {
        let glyph = render_glyph(font, 0, index, cell, cell, color)
            .unwrap_or_else(|| RgbaImage::new(cell, cell));
        let row = index as u32 / cols;
        let col = index as u32 % cols;
        let x0 = gap + col * (cell + gap);
        let y0 = gap + row * (cell + gap);
        let _ = character;
        overlay(&mut image, &glyph, i64::from(x0), i64::from(y0));
    }
    image
}

pub fn render_freetype_glyph_at(
    font: &crate::midata::FreeTypeFontData,
    ch: char,
    width: u32,
    _height: u32,
    color: mirx::Color,
) -> Option<RgbaImage> {
    let glyph = font.glyphs.iter().find(|g| g.codepoint == ch as u32)?;
    if glyph.outline.is_empty() {
        return None;
    }
    let units = font.units_per_em.max(1) as f32;
    let scale = width as f32 * 0.7 / units;

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for cmd in &glyph.outline {
        let mirui_cmd: PathCmd = cmd.clone().into();
        let pts: Vec<(f32, f32)> = match &mirui_cmd {
            PathCmd::MoveTo(p) | PathCmd::LineTo(p) => vec![(p.x.to_f32(), p.y.to_f32())],
            PathCmd::QuadTo { ctrl, end } => vec![
                (ctrl.x.to_f32(), ctrl.y.to_f32()),
                (end.x.to_f32(), end.y.to_f32()),
            ],
            PathCmd::CubicTo { ctrl1, ctrl2, end } => vec![
                (ctrl1.x.to_f32(), ctrl1.y.to_f32()),
                (ctrl2.x.to_f32(), ctrl2.y.to_f32()),
                (end.x.to_f32(), end.y.to_f32()),
            ],
            PathCmd::Close => vec![],
        };
        for (x, y) in pts {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    let gw = (max_x - min_x).max(1.0) * scale;
    let gh = (max_y - min_y).max(1.0) * scale;
    let pad = 4.0;
    let offset_x = pad - min_x * scale;
    let offset_y = pad + max_y * scale;
    let actual_w = (gw + pad * 2.0).ceil() as u32;
    let actual_h = (gh + pad * 2.0).ceil() as u32;

    let mut buffer = vec![0u8; (actual_w * actual_h * 4) as usize];
    let w = actual_w.min(u16::MAX as u32) as u16;
    let h = actual_h.min(u16::MAX as u32) as u16;
    let texture = Texture::new(&mut buffer, w, h, ColorFormat::RGBA8888);
    let mut renderer = SwRenderer::new(texture).with_alpha_mode(AlphaMode::Blend);
    let clip = Rect::new(
        Fixed::ZERO,
        Fixed::ZERO,
        Fixed::from_int(w as i32),
        Fixed::from_int(h as i32),
    );
    let mut path = Path::new();
    for cmd in &glyph.outline {
        let mirui_cmd: PathCmd = cmd.clone().into();
        let mapped = map_freetype_cmd(&mirui_cmd, offset_x, offset_y, scale, 0.0);
        match mapped {
            PathCmd::MoveTo(p) => {
                path.move_to(p);
            }
            PathCmd::LineTo(p) => {
                path.line_to(p);
            }
            PathCmd::QuadTo { ctrl, end } => {
                path.quad_to(ctrl, end);
            }
            PathCmd::CubicTo { ctrl1, ctrl2, end } => {
                path.cubic_to(ctrl1, ctrl2, end);
            }
            PathCmd::Close => {
                path.close();
            }
        }
    }
    let paint = mirui::render::canvas::Paint::Color(
        mirui::types::Color {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
        .into(),
    );
    renderer.fill_path(
        &path,
        &clip,
        &paint,
        255,
        mirui::render::raster::FillRule::NonZero,
    );
    renderer.flush();
    let mut img = RgbaImage::from_raw(actual_w, actual_h, buffer)?;
    for px in img.pixels_mut() {
        px.0[0] = color.r;
        px.0[1] = color.g;
        px.0[2] = color.b;
    }
    Some(img)
}

#[allow(clippy::too_many_arguments)]
pub fn render_freetype_glyph_on_canvas(
    font: &crate::midata::FreeTypeFontData,
    ch: char,
    canvas_w: u32,
    canvas_h: u32,
    scale: f32,
    offset_x: f32,
    baseline_y: f32,
    color: mirx::Color,
) -> Option<RgbaImage> {
    let glyph = font.glyphs.iter().find(|g| g.codepoint == ch as u32)?;
    if glyph.outline.is_empty() {
        return Some(RgbaImage::new(canvas_w, canvas_h));
    }

    let mut buffer = vec![0u8; (canvas_w * canvas_h * 4) as usize];
    let w = canvas_w.min(u16::MAX as u32) as u16;
    let h = canvas_h.min(u16::MAX as u32) as u16;
    let texture = Texture::new(&mut buffer, w, h, ColorFormat::RGBA8888);
    let mut renderer = SwRenderer::new(texture).with_alpha_mode(AlphaMode::Blend);
    let clip = Rect::new(
        Fixed::ZERO,
        Fixed::ZERO,
        Fixed::from_int(w as i32),
        Fixed::from_int(h as i32),
    );
    let mut path = Path::new();
    for cmd in &glyph.outline {
        let mirui_cmd: PathCmd = cmd.clone().into();
        let mapped = map_freetype_cmd(&mirui_cmd, offset_x, 0.0, scale, baseline_y);
        match mapped {
            PathCmd::MoveTo(p) => path.move_to(p),
            PathCmd::LineTo(p) => path.line_to(p),
            PathCmd::QuadTo { ctrl, end } => path.quad_to(ctrl, end),
            PathCmd::CubicTo { ctrl1, ctrl2, end } => path.cubic_to(ctrl1, ctrl2, end),
            PathCmd::Close => path.close(),
        };
    }
    let paint = Paint::Color(color);
    renderer.fill_path(&path, &clip, &paint, 255, FillRule::NonZero);
    renderer.flush();
    let mut img = RgbaImage::from_raw(canvas_w, canvas_h, buffer)
        .unwrap_or_else(|| RgbaImage::new(canvas_w, canvas_h));
    for px in img.pixels_mut() {
        px.0[0] = color.r;
        px.0[1] = color.g;
        px.0[2] = color.b;
    }
    Some(img)
}

pub fn render_freetype_glyphs(
    font: &crate::midata::FreeTypeFontData,
    color: mirx::Color,
) -> RgbaImage {
    if font.glyphs.is_empty() {
        return RgbaImage::new(0, 0);
    }
    let cell: u32 = 48;
    let gap: u32 = 2;
    let cols = (font.glyphs.len() as f64).sqrt().ceil() as u32;
    let cols = cols.max(1);
    let rows = (font.glyphs.len() as u32).div_ceil(cols);
    let grid_w = cols * cell + (cols + 1) * gap;
    let grid_h = rows * cell + (rows + 1) * gap;
    let mut buffer = vec![0u8; (grid_w * grid_h * 4) as usize];
    let w = grid_w.min(u16::MAX as u32) as u16;
    let h = grid_h.min(u16::MAX as u32) as u16;
    let texture = Texture::new(&mut buffer, w, h, ColorFormat::RGBA8888);
    let mut renderer = SwRenderer::new(texture).with_alpha_mode(AlphaMode::Blend);
    let clip = Rect::new(
        Fixed::ZERO,
        Fixed::ZERO,
        Fixed::from_int(w as i32),
        Fixed::from_int(h as i32),
    );
    let units = font.units_per_em.max(1) as f32;
    let scale = cell as f32 * 0.7 / units;
    let baseline = cell as f32 * 0.8;
    for (i, glyph) in font.glyphs.iter().enumerate() {
        let row = i as u32 / cols;
        let col = i as u32 % cols;
        let x0 = gap + col * (cell + gap);
        let y0 = gap + row * (cell + gap);
        if glyph.outline.is_empty() {
            continue;
        }
        let mut path = Path::new();
        for cmd in &glyph.outline {
            let mirui_cmd: PathCmd = cmd.clone().into();
            let mapped = map_freetype_cmd(&mirui_cmd, x0 as f32, y0 as f32, scale, baseline);
            match mapped {
                PathCmd::MoveTo(p) => {
                    path.move_to(p);
                }
                PathCmd::LineTo(p) => {
                    path.line_to(p);
                }
                PathCmd::QuadTo { ctrl, end } => {
                    path.quad_to(ctrl, end);
                }
                PathCmd::CubicTo { ctrl1, ctrl2, end } => {
                    path.cubic_to(ctrl1, ctrl2, end);
                }
                PathCmd::Close => {
                    path.close();
                }
            }
        }
        let paint = Paint::Color(color);
        renderer.fill_path(&path, &clip, &paint, 255, FillRule::NonZero);
    }
    renderer.flush();
    let mut img = RgbaImage::from_raw(grid_w, grid_h, buffer)
        .unwrap_or_else(|| RgbaImage::new(grid_w, grid_h));
    for px in img.pixels_mut() {
        px.0[0] = color.r;
        px.0[1] = color.g;
        px.0[2] = color.b;
    }
    img
}

pub fn render_freetype_text(
    font: &crate::midata::FreeTypeFontData,
    text: &str,
    width: u32,
    height: u32,
    color: mirx::Color,
) -> RgbaImage {
    if width == 0 || height == 0 || text.is_empty() {
        return RgbaImage::new(0, 0);
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return RgbaImage::new(0, 0);
    }
    let cell_width = (width / chars.len() as u32).max(1);
    let mut img = RgbaImage::new(width, height);
    for (idx, ch) in chars.into_iter().enumerate() {
        if let Some(glyph) = render_freetype_glyph_at(font, ch, cell_width, height, color) {
            overlay(&mut img, &glyph, i64::from(idx as u32 * cell_width), 0);
        }
    }
    img
}

fn map_freetype_cmd(cmd: &PathCmd, x0: f32, y0: f32, scale: f32, baseline: f32) -> PathCmd {
    let map_pt = |p: mirui::types::Point| -> mirui::types::Point {
        let raw_x = p.x.to_f32();
        let raw_y = p.y.to_f32();
        mirui::types::Point::new(
            Fixed::from_f32(x0 + raw_x * scale),
            Fixed::from_f32(y0 + baseline - raw_y * scale),
        )
    };
    match cmd {
        PathCmd::MoveTo(p) => PathCmd::MoveTo(map_pt(*p)),
        PathCmd::LineTo(p) => PathCmd::LineTo(map_pt(*p)),
        PathCmd::QuadTo { ctrl, end } => PathCmd::QuadTo {
            ctrl: map_pt(*ctrl),
            end: map_pt(*end),
        },
        PathCmd::CubicTo { ctrl1, ctrl2, end } => PathCmd::CubicTo {
            ctrl1: map_pt(*ctrl1),
            ctrl2: map_pt(*ctrl2),
            end: map_pt(*end),
        },
        PathCmd::Close => PathCmd::Close,
    }
}

fn select_representation(font: &mirx::Font, requested_size: u16) -> Option<usize> {
    let representations = (0..font.representation_count())
        .filter_map(|index| font.representation(index).map(|value| value.metadata()))
        .collect::<Vec<_>>();
    FontRepresentations::new(&representations)
        .ok()?
        .select(
            FontRepresentationRequest::new(requested_size)
                .with_fallback(FontRepresentationFallback::Nearest),
        )
        .ok()
        .map(|matched| matched.index())
}

fn render_glyph(
    font: &mirx::Font,
    representation_index: usize,
    glyph_index: usize,
    target_width: u32,
    target_height: u32,
    color: mirx::Color,
) -> Option<RgbaImage> {
    if target_width == 0 || target_height == 0 {
        return None;
    }
    let representation = font.representation(representation_index)?.metadata();
    let glyph = super::font_contour::unpack_glyph(font, representation_index, glyph_index)?;
    let source_width = glyph.width();
    let source_height = glyph.height();
    if source_width == 0 || source_height == 0 {
        return Some(RgbaImage::new(target_width, target_height));
    }
    let mut image = RgbaImage::new(target_width, target_height);
    let sdf_spread = match representation.kind() {
        FontRepresentationKind::SignedDistance { spread, .. } => Some(f32::from(spread)),
        _ => None,
    };
    let source_scale = (source_width as f32 / target_width as f32)
        .max(source_height as f32 / target_height as f32);
    for y in 0..target_height {
        let source_y = (u64::from(y) * u64::from(source_height) / u64::from(target_height))
            .min(u64::from(source_height - 1)) as usize;
        for x in 0..target_width {
            let source_x = (u64::from(x) * u64::from(source_width) / u64::from(target_width))
                .min(u64::from(source_width - 1)) as usize;
            let sample = glyph.samples()[source_y * source_width as usize + source_x];
            let coverage = if let Some(spread) = sdf_spread {
                let half_edge = (source_scale * 127.5 / spread / 2.0).max(0.5);
                (((f32::from(sample) - (127.5 - half_edge)) / (2.0 * half_edge)) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8
            } else {
                sample
            };
            let alpha = (u16::from(coverage) * u16::from(color.a) / 255) as u8;
            image.put_pixel(x, y, image::Rgba([color.r, color.g, color.b, alpha]));
        }
    }
    Some(image)
}

pub fn render_mirx_glyph_cell(
    font: &mirx::Font,
    ch: char,
    raster_size: u32,
    color: mirx::Color,
) -> RgbaImage {
    let requested_size = match u16::try_from(raster_size) {
        Ok(size) if size != 0 => size,
        _ => return RgbaImage::new(0, 0),
    };
    let Some(representation_index) = select_representation(font, requested_size) else {
        return RgbaImage::new(0, 0);
    };
    let Some(glyph_index) = font.codepoints().binary_search(&ch).ok() else {
        return RgbaImage::new(raster_size, raster_size);
    };
    render_glyph(
        font,
        representation_index,
        glyph_index,
        raster_size,
        raster_size,
        color,
    )
    .unwrap_or_else(|| RgbaImage::new(raster_size, raster_size))
}

pub fn render_font_text(
    font: &mirx::Font,
    text: &str,
    width: u32,
    height: u32,
    color: mirx::Color,
) -> RgbaImage {
    if width == 0 || height == 0 || text.is_empty() {
        return RgbaImage::new(0, 0);
    }
    let requested_size = height.min(u32::from(u16::MAX)) as u16;
    let Some(representation_index) = select_representation(font, requested_size) else {
        return RgbaImage::new(width, height);
    };
    let representation = font.representation(representation_index).unwrap();
    let design = f32::from(representation.metadata().design_ppem());
    let scale = f32::from(requested_size) / design;
    let baseline = representation.line_metrics().ascent().to_f32() * scale;
    let mut output = RgbaImage::new(width, height);
    let mut pen_x = 0.0f32;
    for character in text.chars() {
        let Ok(glyph_index) = font.codepoints().binary_search(&character) else {
            continue;
        };
        let Some(metric) = representation.metrics().get(glyph_index).copied() else {
            continue;
        };
        let Some(unpacked) =
            super::font_contour::unpack_glyph(font, representation_index, glyph_index)
        else {
            continue;
        };
        let target_width = (unpacked.width() as f32 * scale).round().max(1.0) as u32;
        let target_height = (unpacked.height() as f32 * scale).round().max(1.0) as u32;
        if let Some(glyph) = render_glyph(
            font,
            representation_index,
            glyph_index,
            target_width,
            target_height,
            color,
        ) {
            let x = pen_x + metric.bearing_x().to_f32() * scale;
            let y = baseline - metric.bearing_y().to_f32() * scale;
            overlay(&mut output, &glyph, x.round() as i64, y.round() as i64);
        }
        pen_x += metric.advance().to_f32() * scale;
    }
    output
}

pub fn render_font_glyph_on_canvas(
    font: &mirx::Font,
    ch: char,
    width: u32,
    height: u32,
    x: f32,
    baseline_y: f32,
    color: mirx::Color,
) -> RgbaImage {
    if width == 0 || height == 0 {
        return RgbaImage::new(0, 0);
    }
    let requested_size = height.min(u32::from(u16::MAX)) as u16;
    let Some(representation_index) = select_representation(font, requested_size) else {
        return RgbaImage::new(width, height);
    };
    let Some(glyph_index) = font.codepoints().binary_search(&ch).ok() else {
        return RgbaImage::new(width, height);
    };
    let representation = font.representation(representation_index).unwrap();
    let metric = representation.metrics()[glyph_index];
    let scale = f32::from(requested_size) / f32::from(representation.metadata().design_ppem());
    let Some(unpacked) = super::font_contour::unpack_glyph(font, representation_index, glyph_index)
    else {
        return RgbaImage::new(width, height);
    };
    let target_width = (unpacked.width() as f32 * scale).round().max(1.0) as u32;
    let target_height = (unpacked.height() as f32 * scale).round().max(1.0) as u32;
    let mut output = RgbaImage::new(width, height);
    if let Some(glyph) = render_glyph(
        font,
        representation_index,
        glyph_index,
        target_width,
        target_height,
        color,
    ) {
        let glyph_x = x + metric.bearing_x().to_f32() * scale;
        let glyph_y = baseline_y - metric.bearing_y().to_f32() * scale;
        overlay(
            &mut output,
            &glyph,
            glyph_x.round() as i64,
            glyph_y.round() as i64,
        );
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use mirx::font::{
        FontAsset, GlyphMap, GlyphMetrics, GlyphSurfaceAsset, LineMetrics, RawGlyphs,
        RepresentationAsset,
    };
    use mirx::image::SampleLayout;

    fn font(kind: mirx::FontRepresentation, size: u16, data: Vec<u8>) -> mirx::Font {
        let bits = match kind.kind() {
            FontRepresentationKind::Coverage { bits }
            | FontRepresentationKind::SignedDistance { bits, .. } => bits,
            _ => unreachable!(),
        };
        let layout = match bits {
            1 => SampleLayout::A1,
            2 => SampleLayout::A2,
            4 => SampleLayout::A4,
            8 => SampleLayout::A8,
            _ => unreachable!(),
        };
        let codepoints = ['A'];
        let map = GlyphMap::glyph_major(u32::from(size), u32::from(size), 1).unwrap();
        let glyphs = RawGlyphs::builder(map, layout).build(&data).unwrap();
        let metrics = [GlyphMetrics::new(
            mirx::Fixed::from_int(i32::from(size)),
            mirx::Fixed::ZERO,
            mirx::Fixed::from_int(i32::from(size)),
        )];
        let line = LineMetrics::new(
            mirx::Fixed::from_int(i32::from(size)),
            mirx::Fixed::ZERO,
            mirx::Fixed::from_int(i32::from(size)),
        )
        .unwrap();
        mirx::Font::from_asset(
            FontAsset::new(
                &codepoints,
                &[RepresentationAsset::new(kind, 0, line, &metrics)],
                &[GlyphSurfaceAsset::raw(glyphs)],
            ),
            &mirx::PayloadLimits::HOST,
        )
        .unwrap()
    }

    #[test]
    fn render_atlas_returns_grid() {
        let font = font(
            mirx::FontRepresentation::signed_distance(4, 1, 4, 2, 16, 8).unwrap(),
            4,
            vec![0xff; 8],
        );
        let img = render_font_atlas(&font);
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }

    #[test]
    fn render_text_returns_image() {
        let font = font(
            mirx::FontRepresentation::signed_distance(4, 1, 4, 2, 16, 8).unwrap(),
            4,
            vec![0xff; 8],
        );
        let img = render_font_text(
            &font,
            "A",
            32,
            16,
            mirx::Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        );
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }

    fn white() -> mirx::Color {
        mirx::Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }

    #[test]
    fn freetype_glyph_raster_keeps_positive_y_above_negative_y() {
        let point = |x, y| mirx::Point::new(mirx::Fixed::from_int(x), mirx::Fixed::from_int(y));
        let font = crate::midata::FreeTypeFontData {
            family: "test".to_owned(),
            style: "regular".to_owned(),
            units_per_em: 100,
            ascender: 100,
            descender: -20,
            line_height: 120,
            glyph_count: 1,
            glyphs: vec![crate::midata::FreeTypeGlyph {
                codepoint: 'A' as u32,
                advance: 50,
                bearing_x: 0,
                bearing_y: 100,
                bbox: (0, -100, 40, 100),
                outline: vec![
                    mirx::PathCmd::MoveTo(point(0, 60)),
                    mirx::PathCmd::LineTo(point(40, 60)),
                    mirx::PathCmd::LineTo(point(40, 100)),
                    mirx::PathCmd::LineTo(point(0, 100)),
                    mirx::PathCmd::Close,
                    mirx::PathCmd::MoveTo(point(0, -100)),
                    mirx::PathCmd::LineTo(point(20, -100)),
                    mirx::PathCmd::LineTo(point(20, -80)),
                    mirx::PathCmd::LineTo(point(0, -80)),
                    mirx::PathCmd::Close,
                ],
            }],
        };

        let image = render_freetype_glyph_at(&font, 'A', 40, 40, white()).unwrap();
        let split = image.height() / 2;
        let top_alpha: u32 = image
            .rows()
            .take(split as usize)
            .flatten()
            .map(|pixel| u32::from(pixel.0[3]))
            .sum();
        let bottom_alpha: u32 = image
            .rows()
            .skip(split as usize)
            .flatten()
            .map(|pixel| u32::from(pixel.0[3]))
            .sum();
        assert!(top_alpha > bottom_alpha);
    }

    #[test]
    fn coverage_glyph_cell_preserves_native_rows_and_nearest_neighbor_scaling() {
        let font = font(
            mirx::FontRepresentation::coverage(8, 2, 4).unwrap(),
            2,
            vec![255, 0, 0, 0],
        );

        let native = render_mirx_glyph_cell(&font, 'A', 2, white());
        assert_eq!(native.get_pixel(0, 0).0[3], 255);
        assert_eq!(native.get_pixel(0, 1).0[3], 0);

        let scaled = render_mirx_glyph_cell(&font, 'A', 4, white());
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(scaled.get_pixel(x, y).0[3], 255);
            }
        }
        for y in 2..4 {
            assert_eq!(scaled.get_pixel(0, y).0[3], 0);
        }
    }

    #[test]
    fn coverage_glyph_cell_supports_full_u16_source_geometry() {
        let source_size = 256u16;
        let mut data = vec![0; usize::from(source_size) * usize::from(source_size)];
        *data.last_mut().unwrap() = 255;
        let font = font(
            mirx::FontRepresentation::coverage(8, source_size, 65_536).unwrap(),
            source_size,
            data,
        );

        let image = render_mirx_glyph_cell(&font, 'A', u32::from(source_size), white());

        assert_eq!(image.get_pixel(255, 255).0[3], 255);
        assert_eq!(image.get_pixel(255, 254).0[3], 0);
    }

    #[test]
    fn coverage_glyph_cell_multiplies_coverage_by_color_alpha() {
        let font = font(
            mirx::FontRepresentation::coverage(8, 1, 1).unwrap(),
            1,
            vec![128],
        );
        let color = mirx::Color {
            r: 12,
            g: 34,
            b: 56,
            a: 128,
        };

        let image = render_mirx_glyph_cell(&font, 'A', 1, color);

        assert_eq!(image.get_pixel(0, 0).0, [12, 34, 56, 64]);
    }

    #[test]
    fn sdf_glyph_cell_scales_without_reversing_rows() {
        let font = font(
            mirx::FontRepresentation::signed_distance(4, 1, 4, 2, 16, 8).unwrap(),
            4,
            vec![0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        );

        let scaled = render_mirx_glyph_cell(&font, 'A', 8, white());
        let top_alpha: u32 = (0..8).map(|x| u32::from(scaled.get_pixel(x, 0).0[3])).sum();
        let bottom_alpha: u32 = (0..8).map(|x| u32::from(scaled.get_pixel(x, 7).0[3])).sum();
        assert!(top_alpha > bottom_alpha);
    }
}
