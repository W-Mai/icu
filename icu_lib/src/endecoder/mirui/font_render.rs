use image::imageops::overlay;
use image::RgbaImage;
use mirui::render::backends::sw::SwRenderer;
use mirui::render::canvas::Canvas;
use mirui::render::font::Font;
use mirui::render::path::{Path, PathCmd};
use mirui::render::raster::FillRule;
use mirui::render::texture::{AlphaMode, ColorFormat, Texture};
use mirui::types::{Fixed, Point, Rect};
use mirx::{FontChunkKind, Paint};

pub fn render_font_atlas(font: &mirx::Font) -> RgbaImage {
    let atlas = &font.atlas;
    let source = atlas.source_size as u32;
    if source == 0 || font.metrics.is_empty() {
        return RgbaImage::new(0, 0);
    }
    let payload: &'static [u8] = leak_font_payload(font);
    let mirui_font = match font.chunk_header.kind {
        FontChunkKind::Sdf => {
            match mirui::render::font::sdf::font_from_mirx_chunk("atlas", payload) {
                Ok(f) => f,
                Err(_) => return RgbaImage::new(0, 0),
            }
        }
        FontChunkKind::Grayscale => {
            match mirui::render::font::gray::font_from_mirx_chunk("atlas", payload) {
                Ok(f) => f,
                Err(_) => return RgbaImage::new(0, 0),
            }
        }
    };

    let cell = source;
    let gap = 1u32;
    let cols = (font.metrics.len() as f64).sqrt().ceil() as u32;
    let cols = cols.max(1);
    let rows = (font.metrics.len() as u32).div_ceil(cols);
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
    let color = mirui::types::Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    for (i, m) in font.metrics.iter().enumerate() {
        let row = i as u32 / cols;
        let col = i as u32 % cols;
        let x0 = gap + col * (cell + gap);
        let y0 = gap + row * (cell + gap);
        let ch = char::from_u32(m.codepoint).unwrap_or('?');
        let pos = Point::new(Fixed::from_int(x0 as i32), Fixed::from_int(y0 as i32));
        renderer.draw_label(&pos, &ch.to_string(), &mirui_font, &clip, &color, 255);
    }
    renderer.flush();
    RgbaImage::from_raw(grid_w, grid_h, buffer).unwrap_or_else(|| RgbaImage::new(grid_w, grid_h))
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
        let raw_x = p.x.raw() as f32 / 256.0;
        let raw_y = p.y.raw() as f32 / 256.0;
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

fn sample_atlas_pixel(bytes: &[u8], source: u32, x: u32, y: u32, bit_depth: u8) -> Option<u8> {
    let idx = y.checked_mul(source)?.checked_add(x)? as usize;
    match bit_depth {
        1 => {
            let byte = *bytes.get(idx / 8)?;
            let bit = 7 - (idx % 8) as u8;
            Some(if (byte >> bit) & 1 == 1 { 255 } else { 0 })
        }
        2 => {
            let byte = *bytes.get(idx / 4)?;
            let shift = 6 - (idx % 4) as u8 * 2;
            Some(((byte >> shift) & 0x3) * 85)
        }
        4 => {
            let byte = *bytes.get(idx / 2)?;
            let shift = 4 - (idx % 2) as u8 * 4;
            Some(((byte >> shift) & 0xF) * 17)
        }
        8 => bytes.get(idx).copied(),
        _ => None,
    }
}

fn render_gray_glyph_cell(
    font: &mirx::Font,
    ch: char,
    raster_size: u32,
    color: mirx::Color,
) -> Option<RgbaImage> {
    let source_size = u32::from(font.atlas.source_size);
    let bit_depth = font.atlas.bit_depth;
    if source_size == 0 || raster_size == 0 || !matches!(bit_depth, 1 | 2 | 4 | 8) {
        return None;
    }
    let glyph_index = font
        .metrics
        .binary_search_by_key(&(ch as u32), |metric| metric.codepoint)
        .ok()?;
    let expected_bytes = source_size
        .checked_mul(source_size)?
        .checked_mul(u32::from(bit_depth))?
        .div_ceil(8);
    if font.atlas.bytes_per_glyph != expected_bytes {
        return None;
    }
    let stride = usize::try_from(expected_bytes).ok()?;
    let start = glyph_index.checked_mul(stride)?;
    let bytes = font.data.get(start..start.checked_add(stride)?)?;
    let mut image = RgbaImage::new(raster_size, raster_size);
    for y in 0..raster_size {
        let source_y = (u64::from(y) * u64::from(source_size) / u64::from(raster_size))
            .min(u64::from(source_size - 1)) as u32;
        for x in 0..raster_size {
            let source_x = (u64::from(x) * u64::from(source_size) / u64::from(raster_size))
                .min(u64::from(source_size - 1)) as u32;
            let coverage = sample_atlas_pixel(bytes, source_size, source_x, source_y, bit_depth)?;
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
    if raster_size == 0 || font.atlas.source_size == 0 {
        return RgbaImage::new(0, 0);
    }

    match font.chunk_header.kind {
        FontChunkKind::Sdf => {
            let payload: &'static [u8] = leak_font_payload(font);
            let mut mirui_font =
                match mirui::render::font::sdf::font_from_mirx_chunk("icu-glyph-cell", payload) {
                    Ok(font) => font,
                    Err(_) => return RgbaImage::new(raster_size, raster_size),
                };
            mirui_font.size = raster_size.min(u16::MAX as u32) as u16;
            render_text_with_font_at(
                &mirui_font,
                &ch.to_string(),
                raster_size,
                raster_size,
                0.0,
                0.0,
                color,
            )
        }
        FontChunkKind::Grayscale => render_gray_glyph_cell(font, ch, raster_size, color)
            .unwrap_or_else(|| RgbaImage::new(raster_size, raster_size)),
    }
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
    let source_size = font.atlas.source_size.max(1) as u32;
    let scale = height as f32 / source_size as f32;
    let total_advance: u32 = text
        .chars()
        .filter_map(|ch| {
            font.metrics
                .iter()
                .find(|m| m.codepoint == ch as u32)
                .map(|m| m.advance as u32)
        })
        .sum::<u32>()
        .max(1);
    let actual_width = (total_advance as f32 * scale).ceil() as u32 + source_size;
    let ascender = font.atlas.ascender as u32;
    let actual_height = height.max(ascender + source_size + font.atlas.descender as u32);
    let payload: &'static [u8] = leak_font_payload(font);
    let mirui_font = match font.chunk_header.kind {
        FontChunkKind::Sdf => {
            match mirui::render::font::sdf::font_from_mirx_chunk("icu-preview", payload) {
                Ok(f) => f,
                Err(_) => return RgbaImage::new(actual_width, actual_height),
            }
        }
        FontChunkKind::Grayscale => {
            match mirui::render::font::gray::font_from_mirx_chunk("icu-preview", payload) {
                Ok(f) => f,
                Err(_) => return RgbaImage::new(actual_width, actual_height),
            }
        }
    };
    render_text_with_font(&mirui_font, text, actual_width, actual_height, color)
}

fn leak_font_payload(font: &mirx::Font) -> &'static [u8] {
    let payload = font.encode();
    Box::leak(payload.into_boxed_slice())
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
    let payload: &'static [u8] = leak_font_payload(font);
    let mirui_font = match font.chunk_header.kind {
        FontChunkKind::Sdf => {
            match mirui::render::font::sdf::font_from_mirx_chunk("icu-preview", payload) {
                Ok(f) => f,
                Err(_) => return RgbaImage::new(width, height),
            }
        }
        FontChunkKind::Grayscale => {
            match mirui::render::font::gray::font_from_mirx_chunk("icu-preview", payload) {
                Ok(f) => f,
                Err(_) => return RgbaImage::new(width, height),
            }
        }
    };
    render_text_with_font_at(
        &mirui_font,
        &ch.to_string(),
        width,
        height,
        x,
        baseline_y,
        color,
    )
}

fn render_text_with_font_at(
    font: &Font,
    text: &str,
    width: u32,
    height: u32,
    x: f32,
    baseline_y: f32,
    color: mirx::Color,
) -> RgbaImage {
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let w = width.min(u16::MAX as u32) as u16;
    let h = height.min(u16::MAX as u32) as u16;
    let texture = Texture::new(&mut buffer, w, h, ColorFormat::RGBA8888);
    let mut renderer = SwRenderer::new(texture).with_alpha_mode(AlphaMode::Blend);
    let clip = Rect::new(
        Fixed::ZERO,
        Fixed::ZERO,
        Fixed::from_int(w as i32),
        Fixed::from_int(h as i32),
    );
    let pos = Point::new(Fixed::from_f32(x), Fixed::from_f32(baseline_y));
    let render_color = mirui::types::Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    };
    renderer.draw_label(&pos, text, font, &clip, &render_color, 255);
    renderer.flush();
    let mut img =
        RgbaImage::from_raw(width, height, buffer).unwrap_or_else(|| RgbaImage::new(width, height));
    for px in img.pixels_mut() {
        px.0[0] = color.r;
        px.0[1] = color.g;
        px.0[2] = color.b;
    }
    img
}

fn render_text_with_font(
    font: &Font,
    text: &str,
    width: u32,
    height: u32,
    color: mirx::Color,
) -> RgbaImage {
    let metrics = font.metrics();
    render_text_with_font_at(
        font,
        text,
        width,
        height,
        0.0,
        metrics.ascender as f32,
        color,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_sdf_payload() -> Vec<u8> {
        mirx::Font {
            chunk_header: mirx::FontChunkHeader {
                kind: mirx::FontChunkKind::Sdf,
                format: 4,
                size: 24,
            },
            atlas: mirx::AtlasHeader {
                version: mirx::SUPPORTED_VERSION,
                bit_depth: 4,
                _pad0: 0,
                source_size: 4,
                spread: 1,
                glyph_count: 1,
                metric_offset: mirx::HEADER_LEN as u32,
                data_offset: (mirx::HEADER_LEN + mirx::METRIC_LEN) as u32,
                bytes_per_glyph: 8,
                ascender: 3,
                descender: 1,
                line_height: 4,
                _pad1: 0,
            },
            metrics: vec![mirx::GlyphMetric {
                codepoint: 'A' as u32,
                advance: 4,
                bearing_x: 0,
                bearing_y: 3,
            }],
            data: vec![0xFFu8; 8],
        }
        .encode()
    }

    #[test]
    fn render_atlas_returns_grid() {
        let payload = build_sdf_payload();
        let font = mirx::Font::decode(&payload).unwrap();
        let img = render_font_atlas(&font);
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }

    #[test]
    fn render_text_returns_image() {
        let payload = build_sdf_payload();
        let font = mirx::Font::decode(&payload).unwrap();
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

    fn asymmetric_font(kind: FontChunkKind, source_size: u16, data: Vec<u8>) -> mirx::Font {
        let bit_depth = match kind {
            FontChunkKind::Sdf => 4,
            FontChunkKind::Grayscale => 8,
        };
        mirx::Font {
            chunk_header: mirx::FontChunkHeader {
                kind,
                format: bit_depth,
                size: source_size,
            },
            atlas: mirx::AtlasHeader {
                version: mirx::SUPPORTED_VERSION,
                bit_depth,
                _pad0: 0,
                source_size,
                spread: 1,
                glyph_count: 1,
                metric_offset: mirx::HEADER_LEN as u32,
                data_offset: (mirx::HEADER_LEN + mirx::METRIC_LEN) as u32,
                bytes_per_glyph: data.len() as u32,
                ascender: source_size,
                descender: 0,
                line_height: source_size,
                _pad1: 0,
            },
            metrics: vec![mirx::GlyphMetric {
                codepoint: 'A' as u32,
                advance: source_size,
                bearing_x: 0,
                bearing_y: source_size.min(i8::MAX as u16) as i8,
            }],
            data,
        }
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
    fn gray_glyph_cell_preserves_native_rows_and_nearest_neighbor_scaling() {
        let font = asymmetric_font(FontChunkKind::Grayscale, 2, vec![255, 0, 0, 0]);

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
    fn gray_glyph_cell_supports_full_u16_source_geometry() {
        let source_size = 256u16;
        let mut data = vec![0; usize::from(source_size) * usize::from(source_size)];
        *data.last_mut().unwrap() = 255;
        let font = asymmetric_font(FontChunkKind::Grayscale, source_size, data);

        let image = render_mirx_glyph_cell(&font, 'A', u32::from(source_size), white());

        assert_eq!(image.get_pixel(255, 255).0[3], 255);
        assert_eq!(image.get_pixel(255, 254).0[3], 0);
    }

    #[test]
    fn gray_glyph_cell_multiplies_coverage_by_color_alpha() {
        let font = asymmetric_font(FontChunkKind::Grayscale, 1, vec![128]);
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
        let font = asymmetric_font(
            FontChunkKind::Sdf,
            4,
            vec![0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        );

        let scaled = render_mirx_glyph_cell(&font, 'A', 8, white());
        let top_alpha: u32 = (0..8).map(|x| u32::from(scaled.get_pixel(x, 0).0[3])).sum();
        let bottom_alpha: u32 = (0..8).map(|x| u32::from(scaled.get_pixel(x, 7).0[3])).sum();
        assert!(top_alpha > bottom_alpha);
    }
}
