use std::io::{Cursor};

use crate::endecoder::EnDecoder;
use crate::midata::MiData;

pub struct PNG {}

impl EnDecoder for PNG {
    fn encode(&self, data: &MiData) -> Vec<u8> {
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
        MiData::RGBA(image::load_from_memory_with_format(&data, image::ImageFormat::Png).unwrap().to_rgba8())
    }
}
