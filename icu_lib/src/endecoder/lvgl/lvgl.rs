use crate::endecoder::lvgl::color_converter::{rgba8888_from, rgba8888_to};
use crate::endecoder::lvgl::{Flags, ImageDescriptor, ImageHeader, LVGLVersion, LVGL};
use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::MiData;
use crate::EncoderParams;
use image::imageops;
use image::RgbaImage;
use std::io::{Cursor, Write};

impl EnDecoder for LVGL {
    fn can_decode(&self, data: &[u8]) -> bool {
        let header_size = std::mem::size_of::<ImageHeader>();
        if data.len() < header_size {
            return false;
        }

        let header_data = &data[..header_size];

        let header = ImageHeader::decode(Vec::from(header_data));
        header.version() != LVGLVersion::Unknown
    }

    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        let color_format = encoder_params.color_format;

        match data {
            MiData::RGBA(img) => {
                let stride = color_format.get_stride_size(img.width(), encoder_params.stride_align);
                let mut img_data = img.clone();

                if let Some(dither) = encoder_params.dither {
                    let cmap = color_quant::NeuQuant::new(dither as i32, 256, img_data.as_mut());
                    imageops::dither(&mut img_data, &cmap);
                }

                let img_data = rgba8888_to(
                    img_data.as_mut(),
                    color_format,
                    img.width(),
                    img.height(),
                    stride,
                    encoder_params.dither,
                );

                let mut buf = Cursor::new(Vec::new());
                buf.write_all(
                    &ImageDescriptor::new(
                        ImageHeader::new(
                            encoder_params.lvgl_version,
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

        log::trace!("Decoding image with color format: {:?}", header.cf());
        log::trace!("Decoded image header: {:#?}", img_desc.header);
        log::trace!("Converting image data to RGBA");

        // Convert image data to RGBA
        let img_buffer = RgbaImage::from_vec(
            header.w() as u32,
            header.h() as u32,
            rgba8888_from(
                img_desc.data.clone().as_mut(),
                header.cf(),
                header.w() as u32,
                header.h() as u32,
                header.stride() as u32,
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

    fn info(&self, data: &[u8]) -> ImageInfo {
        let header_size = std::mem::size_of::<ImageHeader>();

        let header_data = &data[..header_size];

        let header = ImageHeader::decode(Vec::from(header_data));

        let mut other_info = std::collections::HashMap::new();
        other_info.insert(
            "LVGL Version".to_string(),
            format!("{:?}", header.version()),
        );
        other_info.insert("Color Format".to_string(), format!("{:?}", header.cf()));
        other_info.insert("Flags".to_string(), format!("{:?}", header.flags()));
        if header.version() == LVGLVersion::V9 {
            other_info.insert("Stride".to_string(), format!("{:?}", header.stride()));
        }

        ImageInfo {
            width: header.w() as u32,
            height: header.h() as u32,
            data_size: data.len() as u32,
            format: format!("LVGL.{:?}({:?})", header.version(), header.cf()),
            other_info,
        }
    }
}
