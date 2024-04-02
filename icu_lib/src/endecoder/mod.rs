pub mod common;
pub mod lvgl;

use crate::midata::MiData;
use crate::{endecoder, EncoderParams};

#[derive(Debug)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub data_size: u32,
    pub format: String,

    pub other_info: std::collections::HashMap<String, String>,
}

pub trait EnDecoder {
    fn can_decode(&self, data: &[u8]) -> bool;
    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8>;
    fn decode(&self, data: Vec<u8>) -> MiData;

    fn info(&self, data: &[u8]) -> ImageInfo;
}

pub fn find_endecoder(data: &[u8]) -> Option<&'static dyn EnDecoder> {
    let eds = vec![
        &endecoder::common::AutoDetect {} as &dyn EnDecoder,
        &endecoder::lvgl::LVGL {} as &dyn EnDecoder,
    ];

    for ed in eds {
        let can_decode = ed.can_decode(data);
        if can_decode {
            return Some(ed);
        }
    }

    None
}
