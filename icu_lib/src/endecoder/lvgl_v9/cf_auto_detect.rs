use crate::endecoder::lvgl_v9::color_converter::rgba8888_from;
use crate::endecoder::lvgl_v9::{ColorFormatAutoDectect, ImageDescriptor};
use crate::endecoder::EnDecoder;
use crate::midata::MiData;
use image::RgbaImage;

impl EnDecoder for ColorFormatAutoDectect {
    fn encode(_data: &MiData) -> Vec<u8> {
        unimplemented!()
    }

    fn decode(data: Vec<u8>) -> MiData {
        let img_desc = ImageDescriptor::decode(data);
        let img_buffer = RgbaImage::from_vec(
            img_desc.header.h as u32,
            img_desc.header.w as u32,
            rgba8888_from(img_desc.data.clone().as_mut(), img_desc.header.cf),
        )
        .unwrap();

        MiData::RGBA(img_buffer)
    }
}
