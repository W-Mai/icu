use crate::endecoder::lvgl_v9::{
    common_decode_function, common_encode_function, ColorFormat, ColorFormatARGB8888,
};
use crate::endecoder::EnDecoder;
use crate::midata::MiData;

impl EnDecoder for ColorFormatARGB8888 {
    fn encode(data: &MiData) -> Vec<u8> {
        common_encode_function(data, ColorFormat::ARGB8888)
    }

    fn decode(data: Vec<u8>) -> MiData {
        common_decode_function(data, ColorFormat::ARGB8888)
    }
}