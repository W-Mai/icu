use crate::endecoder::lvgl::color_converter::{rgba8888_from, rgba8888_to};
use crate::endecoder::lvgl::ColorFormat as LvglColorFormat;
use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::MiData;
use crate::EncoderParams;
use image::RgbaImage;
use mirx::{ColorFormat as MirxColorFormat, FlatImageInput, MirxFile};
use serde_json::json;

pub struct Mirx;

fn lvgl_to_mirx(cf: LvglColorFormat) -> Option<MirxColorFormat> {
    match cf {
        LvglColorFormat::RGB565 => Some(MirxColorFormat::RGB565),
        LvglColorFormat::RGB888 => Some(MirxColorFormat::RGB888),
        LvglColorFormat::ARGB8888 | LvglColorFormat::XRGB8888 => Some(MirxColorFormat::BGRA8888),
        _ => None,
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
                MirxColorFormat::RGBA8888 => {
                    dst[di..di + 4].copy_from_slice(&raw[si..si + 4]);
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
                MirxColorFormat::RGBA8888 => {
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

fn bpp_for(cf: MirxColorFormat) -> usize {
    match cf {
        MirxColorFormat::RGB565 | MirxColorFormat::RGB565Swapped => 2,
        MirxColorFormat::RGB888 => 3,
        MirxColorFormat::RGBA8888 | MirxColorFormat::BGRA8888 | MirxColorFormat::XRGB8888 => 4,
        _ => 0,
    }
}

impl EnDecoder for Mirx {
    fn can_decode(&self, data: &[u8]) -> bool {
        data.len() >= 4 && &data[..4] == b"MIRX"
    }

    fn encode(&self, data: &MiData, params: EncoderParams) -> Vec<u8> {
        let img = match data {
            MiData::RGBA(img) => img,
            _ => return Vec::new(),
        };
        let lvgl_cf = if params.color_format == LvglColorFormat::UNKNOWN {
            LvglColorFormat::ARGB8888
        } else {
            params.color_format
        };
        let mirx_cf = match lvgl_to_mirx(lvgl_cf) {
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
            Ok(MirxFile::Chunk(file)) => ImageInfo {
                width: file.header.primary_width,
                height: file.header.primary_height,
                data_size: data.len() as u32,
                format: format!("{:?}", file.header.primary_color_format),
                other_info: json!({"layout": "chunk", "chunks": file.entries.len()}),
            },
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

    fn roundtrip(cf: LvglColorFormat) {
        let img = sample_rgba(4, 4);
        let ed = Mirx;
        let bytes = ed.encode(
            &MiData::RGBA(img.clone()),
            EncoderParams::default().with_color_format(cf),
        );
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
        roundtrip(LvglColorFormat::RGB565);
    }

    #[test]
    fn roundtrip_rgb888() {
        roundtrip(LvglColorFormat::RGB888);
    }

    #[test]
    fn roundtrip_argb8888() {
        roundtrip(LvglColorFormat::ARGB8888);
    }

    #[test]
    fn info_reports_flat_layout() {
        let img = sample_rgba(2, 2);
        let ed = Mirx;
        let bytes = ed.encode(
            &MiData::RGBA(img),
            EncoderParams::default().with_color_format(LvglColorFormat::ARGB8888),
        );
        let info = ed.info(&bytes);
        assert_eq!(info.width, 2);
        assert_eq!(info.height, 2);
        assert!(info.format.contains("BGRA8888"));
    }
}
