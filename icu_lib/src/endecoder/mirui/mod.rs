use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::{FontData, MiData, SceneData};
use crate::EncoderParams;
use image::RgbaImage;
use mirx::{ColorFormat as MirxColorFormat, FlatImageInput, MirxFile};
use serde_json::json;

pub mod font_render;
pub mod scene_render;

pub struct Mirx;

fn bpp_for(cf: MirxColorFormat) -> usize {
    match cf {
        MirxColorFormat::RGB565 | MirxColorFormat::RGB565Swapped => 2,
        MirxColorFormat::RGB888 => 3,
        MirxColorFormat::RGBA8888 | MirxColorFormat::BGRA8888 | MirxColorFormat::XRGB8888 => 4,
        _ => 0,
    }
}

fn rgba_to_mirx_pixels(img: &RgbaImage, cf: MirxColorFormat, stride: u32) -> Option<Vec<u8>> {
    let (w, h) = img.dimensions();
    let raw = img.as_raw();
    let bpp = match cf {
        MirxColorFormat::RGB565 | MirxColorFormat::RGB565Swapped => 2,
        MirxColorFormat::RGB888 => 3,
        MirxColorFormat::RGBA8888 | MirxColorFormat::BGRA8888 | MirxColorFormat::XRGB8888 => 4,
        _ => return None,
    };
    let row_bytes = w as usize * bpp;
    let stride = stride as usize;
    let mut out = vec![0u8; stride * h as usize];
    for y in 0..h as usize {
        let dst = &mut out[y * stride..y * stride + row_bytes];
        for x in 0..w as usize {
            let si = (y * w as usize + x) * 4;
            let di = x * bpp;
            match cf {
                MirxColorFormat::RGBA8888 | MirxColorFormat::XRGB8888 => {
                    dst[di..di + 4].copy_from_slice(&raw[si..si + 4]);
                    if matches!(cf, MirxColorFormat::XRGB8888) {
                        dst[di + 3] = 0xFF;
                    }
                }
                MirxColorFormat::BGRA8888 => {
                    dst[di] = raw[si + 2];
                    dst[di + 1] = raw[si + 1];
                    dst[di + 2] = raw[si];
                    dst[di + 3] = raw[si + 3];
                }
                MirxColorFormat::RGB888 => {
                    dst[di] = raw[si];
                    dst[di + 1] = raw[si + 1];
                    dst[di + 2] = raw[si + 2];
                }
                MirxColorFormat::RGB565 => {
                    let r = (raw[si] >> 3) as u16;
                    let g = (raw[si + 1] >> 2) as u16;
                    let b = (raw[si + 2] >> 3) as u16;
                    let px = (r << 11) | (g << 5) | b;
                    dst[di] = (px & 0xFF) as u8;
                    dst[di + 1] = (px >> 8) as u8;
                }
                MirxColorFormat::RGB565Swapped => {
                    let r = (raw[si] >> 3) as u16;
                    let g = (raw[si + 1] >> 2) as u16;
                    let b = (raw[si + 2] >> 3) as u16;
                    let px = (r << 11) | (g << 5) | b;
                    dst[di] = (px >> 8) as u8;
                    dst[di + 1] = (px & 0xFF) as u8;
                }
                _ => return None,
            }
        }
    }
    Some(out)
}

fn mirx_pixels_to_rgba(
    main: &[u8],
    width: u32,
    height: u32,
    stride: u32,
    cf: MirxColorFormat,
) -> Option<RgbaImage> {
    let w = width as usize;
    let h = height as usize;
    let stride = stride as usize;
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let si = y * stride + x * bpp_for(cf);
            let di = (y * w + x) * 4;
            match cf {
                MirxColorFormat::RGBA8888 | MirxColorFormat::XRGB8888 => {
                    out[di..di + 4].copy_from_slice(&main[si..si + 4]);
                }
                MirxColorFormat::BGRA8888 => {
                    out[di] = main[si + 2];
                    out[di + 1] = main[si + 1];
                    out[di + 2] = main[si];
                    out[di + 3] = main[si + 3];
                }
                MirxColorFormat::RGB888 => {
                    out[di] = main[si];
                    out[di + 1] = main[si + 1];
                    out[di + 2] = main[si + 2];
                    out[di + 3] = 255;
                }
                MirxColorFormat::RGB565 => {
                    let px = u16::from_le_bytes([main[si], main[si + 1]]);
                    out[di] = (((px >> 11) & 0x1F) as u8) << 3;
                    out[di + 1] = (((px >> 5) & 0x3F) as u8) << 2;
                    out[di + 2] = ((px & 0x1F) as u8) << 3;
                    out[di + 3] = 255;
                }
                MirxColorFormat::RGB565Swapped => {
                    let px = u16::from_be_bytes([main[si], main[si + 1]]);
                    out[di] = (((px >> 11) & 0x1F) as u8) << 3;
                    out[di + 1] = (((px >> 5) & 0x3F) as u8) << 2;
                    out[di + 2] = ((px & 0x1F) as u8) << 3;
                    out[di + 3] = 255;
                }
                _ => return None,
            }
        }
    }
    RgbaImage::from_vec(width, height, out)
}

impl EnDecoder for Mirx {
    fn can_decode(&self, data: &[u8]) -> bool {
        data.len() >= 4 && &data[..4] == b"MIRX"
    }

    fn encode(&self, data: &MiData, params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mirx_cf = match params.color_format.to_mirx() {
                    Some(cf) => cf,
                    None => return Vec::new(),
                };
                let (w, h) = img.dimensions();
                let stride = (w as usize * bpp_for(mirx_cf))
                    .next_multiple_of(params.stride_align.max(1) as usize) as u32;
                let main = match rgba_to_mirx_pixels(img, mirx_cf, stride) {
                    Some(v) => v,
                    None => return Vec::new(),
                };
                let input = FlatImageInput {
                    width: w,
                    height: h,
                    stride,
                    format: mirx_cf,
                    main: &main,
                    extra: None,
                };
                mirx::encode_flat(&input)
            }
            MiData::PATH(scene_data) => {
                let payload = match scene_data.scene.encode() {
                    Ok(p) => p,
                    Err(_) => return Vec::new(),
                };
                mirx::encode_chunk_generic(
                    mirx::chunk_type::VECTOR,
                    mirx::ChunkEntry::FLAG_CRITICAL,
                    &payload,
                )
            }
            MiData::FONT(font_data) => {
                let font = match font_data {
                    FontData::Mirx(f) => f,
                    FontData::FreeType(_) => return Vec::new(),
                };
                let payload = font.encode();
                mirx::encode_chunk_generic(
                    mirx::chunk_type::FONT,
                    mirx::ChunkEntry::FLAG_CRITICAL,
                    &payload,
                )
            }
            MiData::GRAY(_) => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        let parsed = match mirx::parse(&data) {
            Ok(f) => f,
            Err(_) => return MiData::RGBA(RgbaImage::new(0, 0)),
        };
        match parsed {
            MirxFile::Flat(img) => MiData::RGBA(
                mirx_pixels_to_rgba(img.main, img.width, img.height, img.stride, img.format)
                    .unwrap_or_else(|| RgbaImage::new(0, 0)),
            ),
            MirxFile::Chunk(file) => {
                if let Some(payload) = file.chunk_payload(&data, mirx::chunk_type::VECTOR) {
                    if let Ok(scene) = mirx::Scene::decode(payload) {
                        return MiData::PATH(SceneData { scene });
                    }
                }
                if let Some(payload) = file.chunk_payload(&data, mirx::chunk_type::FONT) {
                    if let Ok(font) = mirx::Font::decode(payload) {
                        return MiData::FONT(FontData::Mirx(font));
                    }
                }
                if let Some(primary) = file.primary_image {
                    return MiData::RGBA(
                        mirx_pixels_to_rgba(
                            primary.data,
                            primary.width,
                            primary.height,
                            primary.stride,
                            primary.format,
                        )
                        .unwrap_or_else(|| RgbaImage::new(0, 0)),
                    );
                }
                MiData::RGBA(RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        match mirx::parse(data) {
            Ok(MirxFile::Flat(img)) => ImageInfo {
                width: img.width,
                height: img.height,
                data_size: img.main.len() as u32,
                format: format!("{:?}", img.format),
                other_info: json!({"layout": "flat"}),
            },
            Ok(MirxFile::Chunk(file)) => {
                let mut chunks_info = serde_json::Map::new();
                for entry in &file.entries {
                    let payload = match data.get(entry.chunk_offset as usize..) {
                        Some(p) => p,
                        None => continue,
                    };
                    let payload_len = entry.chunk_size as usize;
                    let payload = &payload[..payload_len.min(payload.len())];
                    match entry.chunk_type {
                        mirx::chunk_type::VECTOR => {
                            if let Ok(scene) = mirx::Scene::decode(payload) {
                                chunks_info.insert("vector".into(), json!({"op_count": scene.ops.len()}));
                            }
                        }
                        mirx::chunk_type::FONT => {
                            if let Ok(font) = mirx::Font::decode(payload) {
                                chunks_info.insert(
                                    "font".into(),
                                    json!({
                                        "kind": format!("{:?}", font.chunk_header.kind),
                                        "glyph_count": font.atlas.glyph_count,
                                        "source_size": font.atlas.source_size,
                                        "bit_depth": font.atlas.bit_depth,
                                    }),
                                );
                            }
                        }
                        mirx::chunk_type::IMAGE => {
                            if let Some(primary) = &file.primary_image {
                                chunks_info.insert(
                                    "image".into(),
                                    json!({
                                        "width": primary.width,
                                        "height": primary.height,
                                        "format": format!("{:?}", primary.format),
                                    }),
                                );
                            }
                        }
                        _ => {}
                    }
                }
                ImageInfo {
                    width: file.header.primary_width,
                    height: file.header.primary_height,
                    data_size: data.len() as u32,
                    format: format!("{:?}", file.header.primary_color_format),
                    other_info: json!({"layout": "chunk", "chunks": chunks_info}),
                }
            }
            Err(_) => ImageInfo {
                width: 0,
                height: 0,
                data_size: 0,
                format: "unknown".to_string(),
                other_info: json!({}),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endecoder::ColorFormat;
    use image::Rgba;

    fn sample_rgba(w: u32, h: u32) -> RgbaImage {
        let mut img = RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                img.put_pixel(x, y, Rgba([x as u8, y as u8, 128, 255]));
            }
        }
        img
    }

    fn roundtrip(cf: ColorFormat) {
        let img = sample_rgba(4, 4);
        let ed = Mirx;
        let params = EncoderParams::default().with_color_format(cf);
        let bytes = ed.encode(&MiData::RGBA(img.clone()), params);
        assert!(ed.can_decode(&bytes), "can_decode for {:?}", cf);
        match ed.decode(bytes) {
            MiData::RGBA(back) => {
                assert_eq!(back.dimensions(), img.dimensions(), "dims for {:?}", cf);
            }
            _ => panic!("expected RGBA for {:?}", cf),
        }
    }

    #[test]
    fn roundtrip_rgb565() {
        roundtrip(ColorFormat::RGB565);
    }

    #[test]
    fn roundtrip_rgb565_swapped() {
        roundtrip(ColorFormat::RGB565Swapped);
    }

    #[test]
    fn roundtrip_rgb888() {
        roundtrip(ColorFormat::RGB888);
    }

    #[test]
    fn roundtrip_rgba8888() {
        roundtrip(ColorFormat::RGBA8888);
    }

    #[test]
    fn roundtrip_bgra8888() {
        roundtrip(ColorFormat::BGRA8888);
    }

    #[test]
    fn roundtrip_xrgb8888() {
        roundtrip(ColorFormat::XRGB8888);
    }

    #[test]
    fn info_reports_flat_layout() {
        let img = sample_rgba(2, 2);
        let ed = Mirx;
        let params = EncoderParams::default().with_color_format(ColorFormat::BGRA8888);
        let bytes = ed.encode(&MiData::RGBA(img), params);
        let info = ed.info(&bytes);
        assert_eq!(info.width, 2);
        assert_eq!(info.height, 2);
        assert!(info.format.contains("BGRA8888"));
    }

    #[test]
    fn roundtrip_vector_chunk_preserves_ops() {
        let scene = mirx::Scene {
            ops: vec![mirx::SceneOp::FillPath {
                path: mirx::Path {
                    cmds: vec![
                        mirx::PathCmd::MoveTo(mirx::Point::new(mirx::Fixed::from_int(0), mirx::Fixed::from_int(0))),
                        mirx::PathCmd::LineTo(mirx::Point::new(mirx::Fixed::from_int(10), mirx::Fixed::from_int(0))),
                        mirx::PathCmd::LineTo(mirx::Point::new(mirx::Fixed::from_int(10), mirx::Fixed::from_int(10))),
                        mirx::PathCmd::Close,
                    ],
                },
                transform: mirx::Transform::IDENTITY,
                color: mirx::Color { r: 255, g: 128, b: 0, a: 255 },
                opa: 200,
                fill_rule: mirx::FillRule::EvenOdd,
            }],
        };
        let ed = Mirx;
        let bytes = ed.encode(&MiData::PATH(SceneData { scene: scene.clone() }), EncoderParams::default());
        assert!(ed.can_decode(&bytes));
        match ed.decode(bytes) {
            MiData::PATH(back) => assert_eq!(back.scene.ops.len(), 1),
            other => panic!("expected PATH, got {}", other.variant_name()),
        }
    }

    #[test]
    fn roundtrip_font_chunk_preserves_atlas() {
        let font = mirx::Font {
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
                glyph_count: 2,
                metric_offset: mirx::HEADER_LEN as u32,
                data_offset: (mirx::HEADER_LEN + 2 * mirx::METRIC_LEN) as u32,
                bytes_per_glyph: 8,
                ascender: 3,
                descender: 1,
                line_height: 4,
                _pad1: 0,
            },
            metrics: vec![
                mirx::GlyphMetric { codepoint: 'A' as u32, advance: 4, bearing_x: 0, bearing_y: 3 },
                mirx::GlyphMetric { codepoint: 'B' as u32, advance: 4, bearing_x: 0, bearing_y: 3 },
            ],
            data: vec![0u8; 16],
        };
        let ed = Mirx;
        let bytes = ed.encode(&MiData::FONT(FontData::Mirx(font.clone())), EncoderParams::default());
        assert!(ed.can_decode(&bytes));
        match ed.decode(bytes) {
            MiData::FONT(FontData::Mirx(back)) => {
                assert_eq!(back.atlas.glyph_count, 2);
                assert_eq!(back.metrics.len(), 2);
                assert_eq!(back.metrics[0].codepoint, 'A' as u32);
            }
            other => panic!("expected FONT Mirx, got {}", other.variant_name()),
        }
    }

    #[test]
    fn info_reports_vector_chunk_op_count() {
        let scene = mirx::Scene {
            ops: vec![mirx::SceneOp::FillPath {
                path: mirx::Path { cmds: vec![mirx::PathCmd::Close] },
                transform: mirx::Transform::IDENTITY,
                color: mirx::Color { r: 255, g: 255, b: 255, a: 255 },
                opa: 255,
                fill_rule: mirx::FillRule::EvenOdd,
            }],
        };
        let ed = Mirx;
        let bytes = ed.encode(&MiData::PATH(SceneData { scene }), EncoderParams::default());
        let info = ed.info(&bytes);
        let chunks = info.other_info.get("chunks").and_then(|c| c.as_object()).unwrap();
        let vector = chunks.get("vector").unwrap();
        assert_eq!(vector.get("op_count").and_then(|v| v.as_u64()), Some(1));
    }
}
