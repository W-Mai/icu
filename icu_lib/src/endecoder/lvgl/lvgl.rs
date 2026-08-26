use crate::endecoder::lvgl::color_converter::{extract_indexed, rgba8888_from, rgba8888_to};
use crate::endecoder::lvgl::{
    has_flag, with_flag, Compress, CompressedImage, Flags, HeaderFlag, ImageDescriptor,
    ImageHeader, LVGLVersion, LVGL,
};
use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::{IndexedImageData, MiData};
use crate::EncoderParams;
use image::imageops;
use image::RgbaImage;
use serde_json::{json, Value};
use std::io::{Cursor, Write};

impl EnDecoder for LVGL {
    fn can_decode(&self, data: &[u8]) -> bool {
        ImageHeader::parse(data).is_some()
    }

    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        let color_format: crate::endecoder::lvgl::ColorFormat = encoder_params.color_format.into();
        if color_format == crate::endecoder::lvgl::ColorFormat::UNKNOWN {
            return Vec::new();
        }

        let img = match data {
            MiData::RGBA(img) => img,
            MiData::INDEXED(indexed) => &indexed.rgba,
            _ => return Vec::new(),
        };

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

        if encoder_params.compress != Compress::NONE {
            if encoder_params.compress == Compress::LZ4
                && encoder_params.lvgl_version != LVGLVersion::V9
            {
                log::error!("LVGL LZ4 compression is only supported for v9 images");
                return vec![];
            }
            let block_size = ((color_format.get_bpp() + 7) >> 3) as usize;
            let Some(compressed) =
                CompressedImage::encode(encoder_params.compress, &img_data, block_size)
            else {
                log::error!("Failed to compress LVGL image data");
                return vec![];
            };
            img_data = compressed;
            flags = with_flag(flags, HeaderFlag::COMPRESSED);
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

    fn decode(&self, data: Vec<u8>) -> MiData {
        log::trace!("Decoding image with data size: {}", data.len());
        let img_desc = ImageDescriptor::decode(data);

        if img_desc.data_size == 0 {
            return MiData::RGBA(RgbaImage::new(0, 0));
        }

        let header = &img_desc.header;

        log::trace!("Decoding image with color format: {:?}", header.cf());
        log::trace!("Decoded image header: {:#?}", img_desc.header);
        log::trace!("Converting image data to RGBA");

        let cf = header.cf();
        let w = header.w() as u32;
        let h = header.h() as u32;
        let stride = header.stride() as u32;

        if let Some((palette, indexes, bpp)) = extract_indexed(&img_desc.data, cf, w, h, stride) {
            let rgba_buf = rgba8888_from(&img_desc.data, cf, w, h, stride);
            let rgba = RgbaImage::from_vec(w, h, rgba_buf).unwrap_or_else(|| RgbaImage::new(0, 0));
            return MiData::INDEXED(IndexedImageData {
                rgba,
                palette,
                indexes,
                bpp,
                width: w,
                height: h,
            });
        }

        let img_buffer =
            RgbaImage::from_vec(w, h, rgba8888_from(&img_desc.data, cf, w, h, stride)).unwrap();

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
        let Some((header, payload)) = ImageHeader::split(data) else {
            return ImageInfo {
                width: 0,
                height: 0,
                data_size: data.len() as u32,
                format: "LVGL.Unknown(UNKNOWN)".to_string(),
                other_info: Value::Null,
            };
        };

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

        if has_flag(header.flags(), HeaderFlag::COMPRESSED) {
            if let Some(compressed) = header.expected_data_size().and_then(|expected| {
                let block_size = ((header.cf().get_bpp() + 7) >> 3) as usize;
                CompressedImage::parse(payload, expected, block_size)
            }) {
                let (method, compressed_size, decompressed_size) = compressed.info();
                other_info.insert(
                    "Compressed Info".to_owned(),
                    json!({
                        "Method": format!("{method:#?}"),
                        "Size": compressed_size,
                        "Decompressed Size": decompressed_size
                    }),
                );
            }
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
