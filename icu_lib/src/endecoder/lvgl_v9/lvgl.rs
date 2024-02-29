use crate::endecoder::lvgl_v9::color_converter::{rgba8888_from, rgba8888_to};
use crate::endecoder::lvgl_v9::{ColorFormat, Flags, ImageDescriptor, ImageHeader, LVGL};
use crate::endecoder::EnDecoder;
use crate::midata::MiData;
use crate::EncoderParams;
use image::RgbaImage;
use std::io::{Cursor, Write};

impl EnDecoder for LVGL {
    fn can_decode(&self, data: &Vec<u8>) -> bool {
        let header_size = std::mem::size_of::<ImageHeader>();
        if data.len() < header_size {
            return false;
        }

        let header_data = &data[..header_size];

        let header = ImageHeader::decode(Vec::from(header_data));
        if header.magic != 0x19 && header.cf != ColorFormat::UNKNOWN {
            return false;
        }

        true
    }

    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        let color_format = encoder_params.color_format;

        match data {
            MiData::RGBA(img) => {
                let stride = color_format.get_stride_size(img.width(), encoder_params.stride_align);
                let mut img_data = img.clone();
                let img_data = rgba8888_to(
                    img_data.as_mut(),
                    color_format,
                    img.width(),
                    img.height(),
                    stride,
                );

                let mut buf = Cursor::new(Vec::new());
                buf.write_all(
                    &ImageDescriptor::new(
                        ImageHeader::new(
                            color_format,
                            Flags::NONE,
                            img.width() as u16,
                            img.height() as u16,
                            stride as u16,
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

    fn decode(&self, data: Vec<u8>) -> MiData {
        log::trace!("Decoding image with data size: {}", data.len());
        let img_desc = ImageDescriptor::decode(data);

        let header = &img_desc.header;

        log::trace!("Decoding image with color format: {:?}", header.cf);
        log::trace!("Decoded image header: {:#?}", img_desc.header);
        log::trace!("Converting image data to RGBA");

        // Convert image data to RGBA
        let img_buffer = RgbaImage::from_vec(
            img_desc.header.h as u32,
            img_desc.header.w as u32,
            rgba8888_from(
                img_desc.data.clone().as_mut(),
                header.cf,
                header.w as u32,
                header.h as u32,
                header.stride as u32,
            ),
        )
        .unwrap();

        log::trace!("Converted image data to RGBA");
        log::trace!(
            "Decoded image with size: {}x{}",
            img_buffer.width(),
            img_buffer.height()
        );
        log::trace!("Creating MiData object with RGBA image data and returning it");

        MiData::RGBA(img_buffer)
    }
}
