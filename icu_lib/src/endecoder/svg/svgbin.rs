use crate::EncoderParams;
use crate::endecoder::{EnDecoder, ImageInfo};
use crate::endecoder::svg::SVGBin;
use crate::midata::MiData;

impl EnDecoder for SVGBin {
    fn can_decode(&self, data: &[u8]) -> bool {
        false
    }

    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        todo!()
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        todo!()
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        todo!()
    }
}
