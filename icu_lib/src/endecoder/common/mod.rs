use crate::EncoderParams;
use png;
use std::io::Cursor;

use crate::endecoder::{lvgl, EnDecoder, ImageInfo};
use crate::midata::MiData;
use serde_json::json;

pub struct AutoDetect {}

pub struct PNG {}

pub struct JPEG {}

pub struct BMP {}

pub struct GIF {}

pub struct TIFF {}

pub struct WEBP {}

pub struct ICO {}

pub struct PBM {}

pub struct PGM {}

pub struct PPM {}

pub struct PAM {}

pub struct TGA {}

fn decode_raster(
    data: &[u8],
) -> Result<(image::DynamicImage, image::ImageFormat), image::ImageError> {
    let format = image::guess_format(data)?;
    let image = image::load_from_memory_with_format(data, format)?;
    Ok((image, format))
}

fn decode_fixed(data: &[u8], format: image::ImageFormat, label: &str) -> MiData {
    match image::load_from_memory_with_format(data, format) {
        Ok(image) => MiData::RGBA(image.to_rgba8()),
        Err(error) => {
            log::error!("Failed to decode {label}: {error}");
            MiData::RGBA(image::RgbaImage::new(0, 0))
        }
    }
}

fn empty_image_info(data_size: usize) -> ImageInfo {
    ImageInfo {
        width: 0,
        height: 0,
        data_size: data_size as u32,
        format: "unknown".to_string(),
        other_info: serde_json::Value::Null,
    }
}

impl EnDecoder for AutoDetect {
    fn can_decode(&self, data: &[u8]) -> bool {
        image::guess_format(data).is_ok()
    }

    fn encode(&self, _data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        unimplemented!()
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match decode_raster(&data) {
            Ok((image, _)) => MiData::RGBA(image.to_rgba8()),
            Err(error) => {
                log::error!("Failed to decode raster image: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let (img, img_format) = match decode_raster(data) {
            Ok(decoded) => decoded,
            Err(error) => {
                log::error!("Failed to inspect raster image: {error}");
                return empty_image_info(data.len());
            }
        };

        let mut other_info = serde_json::Map::new();

        other_info.insert(
            "Color Type".to_string(),
            json!(format!("{:?}", img.color())),
        );

        // Try to parse EXIF data
        if let Ok(reader) = exif::Reader::new().read_from_container(&mut std::io::Cursor::new(data))
        {
            let mut exif_map = serde_json::Map::new();
            for field in reader.fields() {
                exif_map.insert(
                    field.tag.to_string(),
                    json!(field.display_value().with_unit(&reader).to_string()),
                );
            }
            if !exif_map.is_empty() {
                other_info.insert("Exif".to_string(), serde_json::Value::Object(exif_map));
            }
        }

        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: img_format.to_mime_type().to_owned(),
            other_info: serde_json::Value::Object(other_info),
        }
    }
}

impl EnDecoder for PNG {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Png
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        let color_format: lvgl::ColorFormat = encoder_params.color_format.into();
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());

                let mut encoder = png::Encoder::new(&mut buf, img.width(), img.height());
                encoder.set_compression(png::Compression::Balanced);
                encoder.set_filter(png::Filter::NoFilter);

                match color_format {
                    lvgl::ColorFormat::I1
                    | lvgl::ColorFormat::I2
                    | lvgl::ColorFormat::I4
                    | lvgl::ColorFormat::I8 => {
                        let bpp = color_format.get_bpp();
                        let color_map_size = 1 << bpp;

                        let data = img.to_vec();
                        let nq = color_quant::NeuQuant::new(
                            encoder_params.dither.unwrap_or(1) as i32,
                            color_map_size,
                            &data,
                        );
                        let mut indexes_iter = data.chunks(4).map(|pix| nq.index_of(pix) as u8);
                        let palette = nq.color_map_rgb();
                        let trns = nq
                            .color_map_rgba()
                            .iter()
                            .skip(3)
                            .step_by(4)
                            .copied()
                            .collect::<Vec<_>>();

                        encoder.set_color(png::ColorType::Indexed);
                        encoder.set_depth(png::BitDepth::from_u8(bpp as u8).unwrap());

                        encoder.set_palette(palette);
                        encoder.set_trns(trns);

                        let width = img.width();
                        let stride_bytes = color_format.get_stride_size(width, 1) as usize;
                        let mut indexes = vec![0; stride_bytes * img.height() as usize];
                        indexes.chunks_exact_mut(stride_bytes).for_each(|row| {
                            let mut iter = row.iter_mut();
                            let mut byte = &mut 0u8;

                            for i in 0..width as u16 {
                                let alpha = indexes_iter.next().unwrap();
                                if i % (8 / bpp) == 0 {
                                    if let Some(next_byte) = iter.next() {
                                        byte = next_byte;
                                    } else {
                                        break;
                                    }
                                }
                                *byte |= (alpha) << ((8 / bpp - 1 - i % (8 / bpp)) * bpp);
                            }
                        });

                        let mut writer = encoder.write_header().unwrap();
                        writer.write_image_data(&indexes).unwrap();
                    }
                    lvgl::ColorFormat::RGB888 => {
                        let data = img
                            .to_vec()
                            .chunks_exact(4)
                            .flat_map(|pix| [pix[0], pix[1], pix[2]])
                            .collect::<Vec<_>>();

                        encoder.set_color(png::ColorType::Rgb);
                        encoder.set_depth(png::BitDepth::Eight);

                        let mut writer = encoder.write_header().unwrap();
                        writer.write_image_data(&data).unwrap();
                    }
                    lvgl::ColorFormat::ARGB8888 => {
                        let data = img.to_vec();
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);

                        let mut writer = encoder.write_header().unwrap();
                        writer.write_image_data(&data).unwrap();
                    }
                    _ => {
                        let data = img.to_vec();
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);

                        let mut writer = encoder.write_header().unwrap();
                        writer.write_image_data(&data).unwrap();
                    }
                }
                {}

                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match image::load_from_memory_with_format(&data, image::ImageFormat::Png) {
            Ok(image) => MiData::RGBA(image.to_rgba8()),
            Err(error) => {
                log::error!("Failed to decode PNG: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let mut info = AutoDetect {}.info(data);

        // Add PNG specific info
        if let Ok(decoder) = png::Decoder::new(Cursor::new(data)).read_info() {
            let png_info = decoder.info();
            if let serde_json::Value::Object(ref mut map) = info.other_info {
                map.insert(
                    "PNG Color Type".to_string(),
                    json!(format!("{:?}", png_info.color_type)),
                );
                map.insert(
                    "Bit Depth".to_string(),
                    json!(format!("{:?}", png_info.bit_depth)),
                );
                if png_info.trns.is_some() {
                    map.insert("Transparent".to_string(), json!("Yes"));
                }
                map.insert("Interlaced".to_string(), json!(png_info.interlaced));
            }
        }

        info
    }
}

impl EnDecoder for JPEG {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Jpeg
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
                if let Err(error) = rgb.write_to(&mut buf, image::ImageFormat::Jpeg) {
                    log::error!("Failed to encode JPEG: {error}");
                    return Vec::new();
                }
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match image::load_from_memory_with_format(&data, image::ImageFormat::Jpeg) {
            Ok(image) => MiData::RGBA(image.to_rgba8()),
            Err(error) => {
                log::error!("Failed to decode JPEG: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for BMP {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Bmp
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Bmp).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::Bmp, "BMP")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for GIF {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Gif
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Gif).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match image::load_from_memory_with_format(&data, image::ImageFormat::Gif) {
            Ok(image) => MiData::RGBA(image.to_rgba8()),
            Err(error) => {
                log::error!("Failed to decode GIF: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for TIFF {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Tiff
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Tiff).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::Tiff, "TIFF")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for WEBP {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::WebP
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::WebP).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::WebP, "WEBP")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for ICO {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Ico
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Ico).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::Ico, "ICO")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for PBM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::GRAY(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match image::load_from_memory_with_format(&data, image::ImageFormat::Pnm) {
            Ok(image) => MiData::GRAY(image.to_luma_alpha8()),
            Err(error) => {
                log::error!("Failed to decode PBM: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for PGM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::GRAY(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match image::load_from_memory_with_format(&data, image::ImageFormat::Pnm) {
            Ok(image) => MiData::GRAY(image.to_luma_alpha8()),
            Err(error) => {
                log::error!("Failed to decode PGM: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for PPM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::Pnm, "PNM")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for PAM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::Pnm, "PNM")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

#[cfg(test)]
mod tests {
    use super::{AutoDetect, BMP, JPEG, PNG};
    use crate::endecoder::EnDecoder;
    use crate::midata::MiData;
    use crate::EncoderParams;
    use image::GenericImageView;

    #[test]
    fn jpeg_encode_accepts_rgba_input() {
        let image = image::RgbaImage::from_vec(2, 1, vec![255, 0, 0, 0, 0, 128, 255, 255]).unwrap();

        let encoded = JPEG {}.encode(&MiData::RGBA(image), EncoderParams::default());

        assert!(!encoded.is_empty());
        assert_eq!(
            image::guess_format(&encoded).unwrap(),
            image::ImageFormat::Jpeg
        );
        assert_eq!(
            image::load_from_memory(&encoded).unwrap().dimensions(),
            (2, 1)
        );
    }

    #[test]
    fn format_probes_reject_non_raster_data_without_panicking() {
        let data = vec![0x19, 0x0a, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(!PNG {}.can_decode(&data));
        assert!(!JPEG {}.can_decode(&data));
        assert!(!BMP {}.can_decode(&data));
    }

    #[test]
    fn autodetect_rejects_truncated_raster_without_panicking() {
        let data = b"\x89PNG\r\n\x1a\n".to_vec();
        assert!(AutoDetect {}.can_decode(&data));
        assert!(matches!(
            AutoDetect {}.decode(data.clone()),
            MiData::RGBA(image) if image.width() == 0 && image.height() == 0
        ));
        let info = AutoDetect {}.info(&data);
        assert_eq!((info.width, info.height), (0, 0));
        assert_eq!(info.data_size, data.len() as u32);
    }
}

impl EnDecoder for TGA {
    fn can_decode(&self, _data: &[u8]) -> bool {
        false
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Tga).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Tga)
                .unwrap()
                .to_rgba8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}
