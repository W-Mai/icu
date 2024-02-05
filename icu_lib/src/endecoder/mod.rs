pub mod common;
pub mod lvgl_v9;

use crate::midata::MiData;
use crate::EncoderParams;

pub trait EnDecoder {
    fn encode(data: &MiData, encoder_params: EncoderParams) -> Vec<u8>;
    fn decode(data: Vec<u8>) -> MiData;
}
