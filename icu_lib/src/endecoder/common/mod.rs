use crate::EncoderParams;
use image::{codecs, ImageError};
use png;
use std::io::Cursor;

use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::MiData;

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

impl EnDecoder for AutoDetect {
    fn can_decode(&self, data: &[u8]) -> bool {
        image::guess_format(data).is_ok()
    }

    fn encode(&self, _data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        unimplemented!()
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        log::trace!("AutoDectect::decoding");
        let img = image::load_from_memory(&data).unwrap();
        log::trace!("AutoDectect::decoded");
        MiData::RGBA(img.to_rgba8())
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        log::trace!("AutoDectect::decoding");
        let img = image::load_from_memory(&data).unwrap();
        let img_format = image::guess_format(data).unwrap();
        log::trace!("AutoDectect::decoded");

        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: img_format.to_mime_type().to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for PNG {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Png
        } else {
            log::error!("It's not a PNG file");
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());

                {
                    let data = img.to_vec();
                    let mut encoder = png::Encoder::new(&mut buf, img.width(), img.height());
                    encoder.set_color(png::ColorType::Indexed);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder.set_compression(png::Compression::Default);
                    encoder.set_filter(png::FilterType::NoFilter);
                    encoder.set_adaptive_filter(png::AdaptiveFilterType::NonAdaptive);

                    let nq = color_quant::NeuQuant::new(30, 256, &data);
                    let indexes_iter = data.chunks(4).map(|pix| nq.index_of(pix) as u8);
                    let trns = nq
                        .color_map_rgba()
                        .iter()
                        .skip(3)
                        .step_by(4)
                        .copied()
                        .collect::<Vec<_>>();

                    encoder.set_palette(nq.color_map_rgb());
                    encoder.set_trns(trns);

                    let img_data = indexes_iter.collect::<Vec<u8>>();

                    let mut writer = encoder.write_header().unwrap();
                    writer.write_image_data(&img_data).unwrap();
                }

                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Png)
                .unwrap()
                .to_rgba8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Png).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/png".to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for JPEG {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Jpeg
        } else {
            log::error!("It's not a JPEG file");
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Jpeg)
                .unwrap()
                .to_rgba8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/jpeg".to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for BMP {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Bmp
        } else {
            log::error!("It's not a BMP file");
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
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Bmp)
                .unwrap()
                .to_rgba8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Bmp).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/bmp".to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for GIF {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Gif
        } else {
            log::error!("It's not a GIF file");
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
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Gif)
                .unwrap()
                .to_rgba8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Gif).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/gif".to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for TIFF {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Tiff
        } else {
            log::error!("It's not a TIFF file");
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
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Tiff)
                .unwrap()
                .to_rgba8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Tiff).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/tiff".to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for WEBP {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::WebP
        } else {
            log::error!("It's not a WEBP file");
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
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::WebP)
                .unwrap()
                .to_rgba8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::WebP).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/webp".to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for ICO {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Ico
        } else {
            log::error!("It's not a ICO file");
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
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Ico)
                .unwrap()
                .to_rgba8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Ico).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/x-icon".to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for PBM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            log::error!("It's not a PBM file");
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
        MiData::GRAY(
            image::load_from_memory_with_format(&data, image::ImageFormat::Pnm)
                .unwrap()
                .to_luma_alpha8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Pnm).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/x-portable-bitmap".to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for PGM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            log::error!("It's not a PGM file");
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
        MiData::GRAY(
            image::load_from_memory_with_format(&data, image::ImageFormat::Pnm)
                .unwrap()
                .to_luma_alpha8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Pnm).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/x-portable-graymap".to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for PPM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            log::error!("It's not a PPM file");
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
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Pnm)
                .unwrap()
                .to_rgba8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Pnm).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/x-portable-pixmap".to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for PAM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            log::error!("It's not a PAM file");
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
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Pnm)
                .unwrap()
                .to_rgba8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Pnm).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/x-portable-arbitrarymap".to_owned(),
            other_info: Default::default(),
        }
    }
}

impl EnDecoder for TGA {
    fn can_decode(&self, _data: &[u8]) -> bool {
        log::error!("TGA is not supported yet");
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
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Tga).unwrap();
        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: "image/x-targa".to_owned(),
            other_info: Default::default(),
        }
    }
}
