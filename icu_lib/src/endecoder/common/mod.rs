use crate::EncoderParams;
use std::io::Cursor;

use crate::endecoder::EnDecoder;
use crate::midata::MiData;

pub struct AutoDectect {}

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

impl EnDecoder for AutoDectect {
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
                img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
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
}
