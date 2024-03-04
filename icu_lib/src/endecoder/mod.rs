pub mod common;
pub mod lvgl_v9;

use crate::midata::MiData;
use crate::EncoderParams;

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
