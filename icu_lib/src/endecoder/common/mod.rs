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
    fn encode(_data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        unimplemented!()
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::RGBA(image::load_from_memory(&data).unwrap().to_rgba8())
    }
}

impl EnDecoder for PNG {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Png)
                .unwrap()
                .to_rgba8(),
        )
    }
}

impl EnDecoder for JPEG {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Jpeg)
                .unwrap()
                .to_rgba8(),
        )
    }
}

impl EnDecoder for BMP {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Bmp).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Bmp)
                .unwrap()
                .to_rgba8(),
        )
    }
}

impl EnDecoder for GIF {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Gif).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Gif)
                .unwrap()
                .to_rgba8(),
        )
    }
}

impl EnDecoder for TIFF {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Tiff).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Tiff)
                .unwrap()
                .to_rgba8(),
        )
    }
}

impl EnDecoder for WEBP {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::WebP).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::WebP)
                .unwrap()
                .to_rgba8(),
        )
    }
}

impl EnDecoder for ICO {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Ico).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Ico)
                .unwrap()
                .to_rgba8(),
        )
    }
}

impl EnDecoder for PBM {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::GRAY(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::GRAY(
            image::load_from_memory_with_format(&data, image::ImageFormat::Pnm)
                .unwrap()
                .to_luma_alpha8(),
        )
    }
}

impl EnDecoder for PGM {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::GRAY(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::GRAY(
            image::load_from_memory_with_format(&data, image::ImageFormat::Pnm)
                .unwrap()
                .to_luma_alpha8(),
        )
    }
}

impl EnDecoder for PPM {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Pnm)
                .unwrap()
                .to_rgba8(),
        )
    }
}

impl EnDecoder for PAM {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Pnm)
                .unwrap()
                .to_rgba8(),
        )
    }
}

impl EnDecoder for TGA {
    fn encode(data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Tga).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Tga)
                .unwrap()
                .to_rgba8(),
        )
    }
}
