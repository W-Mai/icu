use image::RgbaImage;
use mirui::render::backends::sw::SwRenderer;
use mirui::render::canvas::Canvas;
use mirui::render::font::Font;
use mirui::render::texture::{ColorFormat, Texture};
use mirui::types::{Fixed, Point, Rect};
use mirx::FontChunkKind;

pub fn render_font_atlas(font: &mirx::Font) -> RgbaImage {
    let atlas = &font.atlas;
    let source = atlas.source_size as u32;
    if source == 0 || font.metrics.is_empty() {
        return RgbaImage::new(0, 0);
    }
    let bytes_per_glyph = atlas.bytes_per_glyph as usize;
    let cols = (font.metrics.len() as f64).sqrt().ceil() as u32;
    let cols = cols.max(1);
    let rows = (font.metrics.len() as u32 + cols - 1) / cols;
    let cell = source;
    let gap = 1u32;
    let grid_w = cols * cell + (cols + 1) * gap;
    let grid_h = rows * cell + (rows + 1) * gap;
    let mut img = RgbaImage::new(grid_w, grid_h);
    for (i, _m) in font.metrics.iter().enumerate() {
        let row = i as u32 / cols;
        let col = i as u32 % cols;
        let x0 = gap + col * (cell + gap);
        let y0 = gap + row * (cell + gap);
        let start = i * bytes_per_glyph;
        let end = (start + bytes_per_glyph).min(font.data.len());
        let glyph_bytes = &font.data[start..end];
        for y in 0..cell {
            for x in 0..cell {
                let alpha = sample_atlas_pixel(glyph_bytes, source, x, y, atlas.bit_depth);
                let px = img.get_pixel_mut(x0 + x, y0 + y);
                px.0 = [alpha, alpha, alpha, 255];
            }
        }
    }
    img
}

fn sample_atlas_pixel(bytes: &[u8], source: u32, x: u32, y: u32, bit_depth: u8) -> u8 {
    let idx = y * source + x;
    match bit_depth {
        1 => {
            let byte = bytes[idx as usize / 8];
            let bit = 7 - (idx % 8) as u8;
            if (byte >> bit) & 1 == 1 { 255 } else { 0 }
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
) -> RgbaImage {
    if width == 0 || height == 0 || text.is_empty() {
        return RgbaImage::new(0, 0);
    }
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
    render_text_with_font(&mirui_font, text, width, height)
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
) -> RgbaImage {
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let w = width.min(u16::MAX as u32) as u16;
    let h = height.min(u16::MAX as u32) as u16;
    let texture = Texture::new(&mut buffer, w, h, ColorFormat::RGBA8888);
    let mut renderer = SwRenderer::new(texture);
    let clip = Rect::new(Fixed::ZERO, Fixed::ZERO, Fixed::from_int(w as i32), Fixed::from_int(h as i32));
    let pos = Point::new(Fixed::ZERO, Fixed::ZERO);
    let color = mirui::types::Color { r: 255, g: 255, b: 255, a: 255 };
    renderer.draw_label(&pos, text, font, &clip, &color, 255);
    renderer.flush();
    RgbaImage::from_raw(width, height, buffer).unwrap_or_else(|| RgbaImage::new(width, height))
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
        let img = render_font_text(&font, "A", 32, 16);
        assert_eq!(img.dimensions(), (32, 16));
    }
}
