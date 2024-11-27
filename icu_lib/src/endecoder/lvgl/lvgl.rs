use crate::endecoder::lvgl::color_converter::{rgba8888_from, rgba8888_to};
use crate::endecoder::lvgl::{
    has_flag, with_flag, Compress, Flags, HeaderFlag, ImageCompressedHeader, ImageDescriptor,
    ImageHeader, LVGLVersion, LVGL,
};
use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::MiData;
use crate::EncoderParams;
use image::imageops;
use image::RgbaImage;
use serde_json::{json, Value};
use std::io::{Cursor, Write};

impl EnDecoder for LVGL {
    fn can_decode(&self, data: &[u8]) -> bool {
        let header_size = size_of::<ImageHeader>();
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

                let mut img_data = rgba8888_to(
                    img_data.as_mut(),
                    color_format,
                    img.width(),
                    img.height(),
                    stride,
                    encoder_params.dither,
                );

                let mut flags = Flags::from(0u16);

                match encoder_params.compress {
                    Compress::NONE => {}
                    Compress::Rle => {
                        use super::super::utils::rle::RleCoder;
                        let blk_size = ((color_format.get_bpp() + 7) >> 3) as usize;
                        let rle_coder = RleCoder::new().with_block_size(blk_size).unwrap();
                        let mut compressed_data = match rle_coder.encode(&img_data) {
                            Ok(data) => data,
                            Err(err) => {
                                log::error!("RLE encoding failed: {:?}", err);
                                return vec![];
                            }
                        };

                        let image_compressed_header = ImageCompressedHeader::new()
                            .with_method(encoder_params.compress)
                            .with_compressed_size(compressed_data.len() as u32)
                            .with_decompressed_size(img_data.len() as u32);
                        let mut ich_vec = image_compressed_header.into_bytes().to_vec();
                        ich_vec.append(&mut compressed_data);

                        img_data = ich_vec;
                        flags = with_flag(flags, HeaderFlag::COMPRESSED);
                    }
                    _ => {}
                }

                let mut buf = Cursor::new(Vec::new());
                buf.write_all(
                    &ImageDescriptor::new(
                        ImageHeader::new(
                            encoder_params.lvgl_version,
                            color_format,
                            flags,
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
        let header_size = size_of::<ImageHeader>();

        let header_data = &data[..header_size];

        let header = ImageHeader::decode(Vec::from(header_data));

        let mut other_info = serde_json::Map::new();

        other_info.insert(
            "LVGL Version".to_string(),
            Value::from(format!("{:#?}", header.version())),
        );
        other_info.insert(
            "Color Format".to_string(),
            Value::from(format!("{:#?}", header.cf())),
        );
        other_info.insert(
            "Flags".to_string(),
            Value::from(format!("{:#?}", header.flags())),
        );
        if header.version() == LVGLVersion::V9 {
            other_info.insert("Stride".to_string(), Value::from(header.stride()));
        }

        // Deal Flag has Compressed
        if has_flag(header.flags(), HeaderFlag::COMPRESSED) {
            let data = &data[header.header_size()..];
            let compressed_header = ImageCompressedHeader::from_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
                data[9], data[10], data[11],
            ]);

            other_info.insert(
                "Compressed Info".to_owned(),
                json!({
                    "Method": format!("{:#?}", compressed_header.method()),
                    "Size": compressed_header.compressed_size(),
                    "Decompressed Size": compressed_header.decompressed_size()
                }),
            );
        }

        ImageInfo {
            width: header.w() as u32,
            height: header.h() as u32,
            data_size: data.len() as u32,
            format: format!("LVGL.{:?}({:?})", header.version(), header.cf()),
            other_info: Value::from(other_info),
        }
    }
}
