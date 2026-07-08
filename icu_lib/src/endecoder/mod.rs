pub mod color_format;
pub mod common;
pub mod font;
pub mod lvgl;
pub mod mirui;
pub mod raw;
pub mod utils;

pub use color_format::ColorFormat;

use crate::midata::MiData;
use crate::EncoderParams;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub data_size: u32,
    pub format: String,

    pub other_info: serde_json::Value,
}

pub trait EnDecoder {
    fn can_decode(&self, data: &[u8]) -> bool;
    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8>;
    fn decode(&self, data: Vec<u8>) -> MiData;

    fn info(&self, data: &[u8]) -> ImageInfo;
}

pub fn find_endecoder(data: &[u8]) -> Option<&'static dyn EnDecoder> {
    let eds = vec![
        &common::PNG {} as &dyn EnDecoder,
        &common::AutoDetect {} as &dyn EnDecoder,
        &lvgl::LVGL {} as &dyn EnDecoder,
        &mirui::Mirx {} as &dyn EnDecoder,
        &font::FreeType {} as &dyn EnDecoder,
    ];

    for ed in eds {
        let can_decode = ed.can_decode(data);
        if can_decode {
            return Some(ed);
        }
    }

    None
}
