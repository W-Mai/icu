use crate::endecoder::lvgl_v9::color_converter::rgba8888_from;
use crate::endecoder::lvgl_v9::{ColorFormatAutoDectect, ImageDescriptor};
use crate::endecoder::EnDecoder;
use crate::midata::MiData;
use crate::EncoderParams;
use image::RgbaImage;

impl EnDecoder for ColorFormatAutoDectect {
    fn encode(&self, _data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        unimplemented!()
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        log::trace!("Decoding image data with ColorFormatAutoDectect");
        let img_desc = ImageDescriptor::decode(data);
        let header = &img_desc.header;

        log::trace!("Decoded image header: {:#?}", img_desc.header);

        log::trace!("Converting image data to RGBA");
        let img_buffer = RgbaImage::from_vec(
            img_desc.header.h as u32,
            img_desc.header.w as u32,
            rgba8888_from(
                img_desc.data.clone().as_mut(),
                img_desc.header.cf,
                header.w as u32,
                header.h as u32,
                img_desc.header.stride as u32,
            ),
        )
        .unwrap();
        
        log::trace!("Image data converted to RGBA");

        log::trace!("Creating MiData object with RGBA image data and returning it");
        MiData::RGBA(img_buffer)
    }
}
