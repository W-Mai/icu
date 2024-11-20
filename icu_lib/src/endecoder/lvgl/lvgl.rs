use crate::endecoder::lvgl::color_converter::{rgba8888_from, rgba8888_to};
use crate::endecoder::lvgl::{
    Flags, ImageCompressedHeader, ImageDescriptor, ImageHeader, LVGLVersion, LVGL,
};
use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::MiData;
use crate::EncoderParams;
use image::imageops;
use image::RgbaImage;
use std::collections::BTreeMap;
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

        let mut other_info = BTreeMap::new();
        other_info.insert(
            "LVGL Version".to_string(),
            format!("{:?}", header.version()),
        );
        other_info.insert("Color Format".to_string(), format!("{:?}", header.cf()));
        other_info.insert("Flags".to_string(), format!("{:?}", header.flags()));
        if header.version() == LVGLVersion::V9 {
            other_info.insert("Stride".to_string(), format!("{:?}", header.stride()));
        }

        // Deal Flag has Compressed
        if header.flags().has_flag(Flags::COMPRESSED) {
            let data = &data[header.header_size()..];
            let compressed_header = ImageCompressedHeader::from_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
                data[9], data[10], data[11],
            ]);

            let compressed_info = BTreeMap::from([
                ("Method", format!("{:#?}", compressed_header.method())),
                (
                    "Size",
                    format!("{:#?}", compressed_header.compressed_size()),
                ),
                (
                    "Decompressed Size",
                    format!("{:#?}", compressed_header.decompressed_size()),
                ),
            ]);
            other_info.insert(
                "Compressed Info".to_owned(),
                format!("{:#?}", compressed_info),
            );
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
