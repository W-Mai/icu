use crate::endecoder::EnDecoder;
use crate::midata::MiData;
use std::io::{Cursor, Write};

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum ColorFormat {
    // 1 byte (+alpha) formats
    L8 = 0x06,
    I1 = 0x07,
    I2 = 0x08,
    I4 = 0x09,
    I8 = 0x0A,
    A8 = 0x0E,

    // 2 bytes (+alpha) formats
    RGB565 = 0x12,
    RGB565A8 = 0x14,

    // 3 bytes formats
    RGB888 = 0x0F,
    ARGB8888 = 0x10,
    XRGB8888 = 0x11,

    // Formats not supported by software renderer but kept here so GPU can use it
    A1 = 0x0B,
    A2 = 0x0C,
    A4 = 0x0D,
}

pub struct ColorFormatL1 {}

pub struct ColorFormatI1 {}

pub struct ColorFormatI2 {}

pub struct ColorFormatI4 {}

pub struct ColorFormatI8 {}

pub struct ColorFormatA8 {}

pub struct ColorFormatRGB565 {}

pub struct ColorFormatRGB565A8 {}

pub struct ColorFormatRGB888 {}

pub struct ColorFormatARGB8888 {}

pub struct ColorFormatXRGB8888 {}

pub struct ColorFormatA1 {}

pub struct ColorFormatA2 {}

pub struct ColorFormatA4 {}

#[derive(Copy, Clone)]
#[repr(u16)]
pub enum Flags {
    PREMULTIPLIED = 1 << 0,
    MODIFIABLE = 1 << 1,
    VECTORS = 1 << 2,
    COMPRESSED = 1 << 3,
    ALLOCATED = 1 << 4,
    USER1 = 0x1000,
    USER2 = 0x2000,
    USER3 = 0x4000,
    USER4 = 0x8000,
    USER5 = 0x0100,
    USER6 = 0x0200,
    USER7 = 0x0400,
    USER8 = 0x0800,
}

#[repr(C, packed)]
pub struct ImageHeader {
    // Magic number. Must be LV_IMAGE_HEADER_MAGIC
    magic: u8,
    // Color format: See `lv_color_format_t`
    cf: ColorFormat,
    // Image flags, see `lv_image_flags_t`
    flags: Flags,

    // Width of the image in pixels
    w: u16,
    // Height of the image in pixels
    h: u16,
    // Number of bytes in a row
    stride: u16,
    // Reserved to be used later
    reserved_2: u16,
}

impl ImageHeader {
    pub fn new(cf: ColorFormat, flags: Flags, w: u16, h: u16, stride: u16) -> Self {
        Self {
            magic: 0x19,
            cf,
            flags,
            w,
            h,
            stride,
            reserved_2: 0,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        buf.write_all(&self.magic.to_le_bytes()).unwrap();
        buf.write_all(&(self.cf as u8).to_le_bytes()).unwrap();
        buf.write_all(&(self.flags as u16).to_le_bytes()).unwrap();
        buf.write_all(&self.w.to_le_bytes()).unwrap();
        buf.write_all(&self.h.to_le_bytes()).unwrap();
        buf.write_all(&self.stride.to_le_bytes()).unwrap();
        buf.write_all(&self.reserved_2.to_le_bytes()).unwrap();
        buf.into_inner()
    }
}

/*typedef struct {
    lv_image_header_t header; /**< A header describing the basics of the image*/
    uint32_t data_size;     /**< Size of the image in bytes*/
    const uint8_t * data;   /**< Pointer to the data of the image*/
} lv_image_dsc_t;*/

pub struct ImageDescriptor {
    header: ImageHeader,
    data_size: u32,
    data: Vec<u8>,
}

impl ImageDescriptor {
    pub fn new(header: ImageHeader, data: Vec<u8>) -> Self {
        Self {
            header,
            data_size: data.len() as u32,
            data,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        buf.write_all(self.header.encode().as_slice()).unwrap();
        buf.write_all(&self.data_size.to_le_bytes()).unwrap();
        buf.write_all(self.data.as_slice()).unwrap();
        buf.into_inner()
    }
}

fn rgba8888_to_argb8888(bytes: &mut Vec<u8>) {
    for i in (0..bytes.len()).step_by(4) {
        let end = std::cmp::min(i + 4, bytes.len());
        let slice_to_convert = &mut bytes[i..end];
        slice_to_convert.rotate_right(1);
        slice_to_convert.reverse();
    }
}

impl EnDecoder for ColorFormatARGB8888 {
    fn encode(data: &MiData) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut img_data = img.clone().to_vec();
                rgba8888_to_argb8888(&mut img_data);

                let mut buf = Cursor::new(Vec::new());
                buf.write_all(
                    &ImageDescriptor::new(
                        ImageHeader::new(
                            ColorFormat::ARGB8888,
                            Flags::ALLOCATED,
                            img.width() as u16,
                            img.height() as u16,
                            img.width() as u16 * 4,
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

    fn decode(_data: Vec<u8>) -> MiData {
        todo!()
    }
}
