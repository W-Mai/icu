use crate::endecoder;
use crate::endecoder::EnDecoder;
use crate::EncoderParams;
use image::{GrayAlphaImage, RgbaImage};

pub enum MiData {
    RGBA(RgbaImage),
    GRAY(GrayAlphaImage),
    PATH,
}

impl MiData {
    pub fn decode_from(ed: &dyn EnDecoder, data: Vec<u8>) -> Self {
        ed.decode(data)
    }

    pub fn encode_into(&self, ed: &dyn EnDecoder, encoder_params: EncoderParams) -> Vec<u8> {
        ed.encode(self, encoder_params)
    }
}

pub fn decode_from(data: Vec<u8>) -> MiData {
    let eds = vec![
        &endecoder::common::AutoDectect {} as &dyn EnDecoder,
        &endecoder::lvgl_v9::LVGL {} as &dyn EnDecoder,
    ];


    for ed in eds {
        let can_decode = ed.can_decode(&data);
        if can_decode {
            return ed.decode(data);
        }
    }

    unimplemented!()
}
