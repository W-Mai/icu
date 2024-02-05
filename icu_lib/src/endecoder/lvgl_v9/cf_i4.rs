use crate::endecoder::lvgl_v9::{
    common_decode_function, common_encode_function, ColorFormat, ColorFormatI4,
};
use crate::endecoder::EnDecoder;
use crate::midata::MiData;
use crate::EncoderParams;

impl EnDecoder for ColorFormatI4 {
    fn encode(data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        common_encode_function(data, ColorFormat::I4, encoder_params)
    }

    fn decode(data: Vec<u8>) -> MiData {
        common_decode_function(data, ColorFormat::I4)
    }
}
