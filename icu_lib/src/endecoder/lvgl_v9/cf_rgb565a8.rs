use crate::endecoder::lvgl_v9::color_converter::{rgba8888_from, rgba8888_to};
use crate::endecoder::lvgl_v9::{
    ColorFormat, ColorFormatRGB565A8, Flags, ImageDescriptor, ImageHeader,
};
use crate::endecoder::EnDecoder;
use crate::midata::MiData;
use image::RgbaImage;
use std::io::{Cursor, Write};

impl EnDecoder for ColorFormatRGB565A8 {
    fn encode(data: &MiData) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut img_data = img.clone();
                let img_data = rgba8888_to(img_data.as_mut(), ColorFormat::RGB565A8);

                let mut buf = Cursor::new(Vec::new());
                buf.write_all(
                    &ImageDescriptor::new(
                        ImageHeader::new(
                            ColorFormat::RGB565A8,
                            Flags::NONE,
                            img.width() as u16,
                            img.height() as u16,
                            img.width() as u16 * 2,
                        ),
                        img_data,
                    )
                    .encode(),
                )
                .unwrap();

                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(data: Vec<u8>) -> MiData {
        let img_desc = ImageDescriptor::decode(data);
        let img_buffer = RgbaImage::from_vec(
            img_desc.header.h as u32,
            img_desc.header.w as u32,
            rgba8888_from(img_desc.data.clone().as_mut(), ColorFormat::RGB565A8),
        )
        .unwrap();

        MiData::RGBA(img_buffer)
    }
}
