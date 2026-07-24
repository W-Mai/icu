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
    let rows = (font.metrics.len() as u32 + cols - 1) / cols;
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
            PathCmd::QuadTo { ctrl, end } => vec![(ctrl.x.to_f32(), ctrl.y.to_f32()), (end.x.to_f32(), end.y.to_f32())],
            PathCmd::CubicTo { ctrl1, ctrl2, end } => vec![(ctrl1.x.to_f32(), ctrl1.y.to_f32()), (ctrl2.x.to_f32(), ctrl2.y.to_f32()), (end.x.to_f32(), end.y.to_f32())],
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
        let mapped = map_freetype_cmd(&mirui_cmd, offset_x as u32, offset_y as u32, scale, 0.0);
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
    let rows = (font.glyphs.len() as u32 + cols - 1) / cols;
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
            let mapped = map_freetype_cmd(&mirui_cmd, x0, y0, scale, baseline);
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

fn map_freetype_cmd(cmd: &PathCmd, x0: u32, y0: u32, scale: f32, baseline: f32) -> PathCmd {
    let map_pt = |p: mirui::types::Point| -> mirui::types::Point {
        let raw_x = p.x.raw() as f32 / 256.0;
        let raw_y = p.y.raw() as f32 / 256.0;
        mirui::types::Point::new(
            Fixed::from_f32(x0 as f32 + raw_x * scale),
            Fixed::from_f32(y0 as f32 + baseline - raw_y * scale),
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

#[allow(dead_code)]
fn sample_atlas_pixel(bytes: &[u8], source: u32, x: u32, y: u32, bit_depth: u8) -> u8 {
    let idx = y * source + x;
    match bit_depth {
        1 => {
            let byte = bytes[idx as usize / 8];
            let bit = 7 - (idx % 8) as u8;
            if (byte >> bit) & 1 == 1 {
                255
            } else {
                0
            }
        }
        2 => {
            let byte = bytes[idx as usize / 4];
            let shift = 6 - (idx % 4) as u8 * 2;
            let v = (byte >> shift) & 0x3;
            v * 85
        }
        4 => {
            let byte = bytes[idx as usize / 2];
            let shift = 4 - (idx % 2) as u8 * 4;
            let v = (byte >> shift) & 0xF;
            v * 17
        }
        8 => bytes[idx as usize],
        _ => 0,
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

fn render_text_with_font(
    font: &Font,
    text: &str,
    width: u32,
    height: u32,
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
    let metrics = font.metrics();
    let pos = Point::new(Fixed::ZERO, Fixed::from_int(metrics.ascender as i32));
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
        assert_eq!(img.dimensions(), (32, 16));
    }
}
