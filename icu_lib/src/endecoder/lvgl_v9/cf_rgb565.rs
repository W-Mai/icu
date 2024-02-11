use crate::endecoder::lvgl_v9::{
    common_decode_function, common_encode_function, ColorFormat, ColorFormatRGB565,
};
use crate::endecoder::EnDecoder;
use crate::midata::MiData;
use crate::EncoderParams;

impl EnDecoder for ColorFormatRGB565 {
    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        common_encode_function(data, ColorFormat::RGB565, encoder_params)
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        common_decode_function(data, ColorFormat::RGB565)
    }
}
