use crate::endecoder::lvgl_v9::{
    common_decode_function, common_encode_function, ColorFormat, ColorFormatRGB888,
};
use crate::endecoder::EnDecoder;
use crate::midata::MiData;
use crate::EncoderParams;

impl EnDecoder for ColorFormatRGB888 {
    fn encode(data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        common_encode_function(data, ColorFormat::RGB888, encoder_params)
    }

    fn decode(data: Vec<u8>) -> MiData {
        common_decode_function(data, ColorFormat::RGB888)
    }
}
