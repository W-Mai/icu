use crate::endecoder::lvgl_v9::{
    common_decode_function, common_encode_function, ColorFormat, ColorFormatL8,
};
use crate::endecoder::EnDecoder;
use crate::midata::MiData;
use crate::EncoderParams;

impl EnDecoder for ColorFormatL8 {
    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        common_encode_function(data, ColorFormat::L8, encoder_params)
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        common_decode_function(data, ColorFormat::L8)
    }
}
