pub mod common;
pub mod lvgl_v9;

use crate::midata::MiData;
use crate::EncoderParams;

pub trait EnDecoder {
    fn can_decode(&self, data: &Vec<u8>) -> bool;
    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8>;
    fn decode(&self, data: Vec<u8>) -> MiData;
}
