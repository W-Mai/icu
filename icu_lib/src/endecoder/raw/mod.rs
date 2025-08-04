use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::MiData;
use crate::EncoderParams;

pub struct RawImage {}

impl EnDecoder for RawImage {
    fn can_decode(&self, _data: &[u8]) -> bool {
        todo!()
    }

    fn encode(&self, _data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        todo!()
    }

    fn decode(&self, _data: Vec<u8>) -> MiData {
        todo!()
    }

    fn info(&self, _data: &[u8]) -> ImageInfo {
        todo!()
    }
}
