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
        LvglColorFormat::ARGB8888 | LvglColorFormat::XRGB8888 => Some(MirxColorFormat::BGRA8888),
        _ => None,
    }
}

fn mirx_to_lvgl(cf: MirxColorFormat) -> Option<LvglColorFormat> {
    match cf {
        MirxColorFormat::RGB565 | MirxColorFormat::RGB565Swapped => Some(LvglColorFormat::RGB565),
        MirxColorFormat::BGRA8888 => Some(LvglColorFormat::ARGB8888),
        _ => None,
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
        let stride = lvgl_cf.get_stride_size(w, params.stride_align.max(1));
        let main = rgba8888_to(img.as_raw(), lvgl_cf, w, h, stride, params.dither);
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
            MirxFile::Flat(img) => flat_to_midata(&img),
            MirxFile::Chunk(file) => {
                if let Some(primary) = file.primary_image {
                    let lvgl_cf = match mirx_to_lvgl(primary.format) {
                        Some(cf) => cf,
                        None => return MiData::RGBA(RgbaImage::new(0, 0)),
                    };
                    let rgba = rgba8888_from(
                        primary.data,
                        lvgl_cf,
                        primary.width,
                        primary.height,
                        primary.stride,
                    );
                    return MiData::RGBA(
                        RgbaImage::from_vec(primary.width, primary.height, rgba)
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

fn flat_to_midata(img: &mirx::FlatImage<'_>) -> MiData {
    let lvgl_cf = match mirx_to_lvgl(img.format) {
        Some(cf) => cf,
        None => return MiData::RGBA(RgbaImage::new(0, 0)),
    };
    let rgba = rgba8888_from(img.main, lvgl_cf, img.width, img.height, img.stride);
    MiData::RGBA(
        RgbaImage::from_vec(img.width, img.height, rgba)
            .unwrap_or_else(|| RgbaImage::new(0, 0)),
    )
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

    #[test]
    fn roundtrip_bgra8888() {
        let img = sample_rgba(4, 4);
        let ed = Mirx;
        let bytes = ed.encode(
            &MiData::RGBA(img.clone()),
            EncoderParams::default().with_color_format(LvglColorFormat::ARGB8888),
        );
        assert!(ed.can_decode(&bytes));
        let back = ed.decode(bytes);
        match back {
            MiData::RGBA(back_img) => {
                assert_eq!(back_img.dimensions(), img.dimensions());
                for y in 0..4 {
                    for x in 0..4 {
                        assert_eq!(img.get_pixel(x, y), back_img.get_pixel(x, y));
                    }
                }
            }
            _ => panic!("expected RGBA"),
        }
    }

    #[test]
    fn roundtrip_rgb565() {
        let img = sample_rgba(4, 4);
        let ed = Mirx;
        let bytes = ed.encode(
            &MiData::RGBA(img.clone()),
            EncoderParams::default().with_color_format(LvglColorFormat::RGB565),
        );
        assert!(ed.can_decode(&bytes));
        match ed.decode(bytes) {
            MiData::RGBA(back) => assert_eq!(back.dimensions(), img.dimensions()),
            _ => panic!("expected RGBA"),
        }
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
