use std::io::{Cursor, Read, Write};

use image::RgbaImage;

use crate::endecoder::lvgl_v9::color_converter::{rgba8888_from, rgba8888_to};
use crate::midata::MiData;

mod cf_a1;
mod cf_a2;
mod cf_a4;
mod cf_a8;
mod cf_argb8888;
mod cf_auto_detect;
mod cf_i1;
mod cf_i2;
mod cf_i4;
mod cf_i8;
mod cf_l8;
mod cf_rgb565;
mod cf_rgb565a8;
mod cf_rgb888;
mod cf_xrgb8888;
mod color_converter;

#[derive(Copy, Clone, PartialEq, Debug)]
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

pub struct ColorFormatAutoDectect {}

pub struct ColorFormatL8 {}

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
    NONE = 0,
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

    pub fn decode(data: Vec<u8>) -> Self {
        let header_size = std::mem::size_of::<ImageHeader>();
        let mut header = ImageHeader::new(ColorFormat::RGB888, Flags::NONE, 0, 0, 0);

        if data.len() < header_size {
            return header;
        }

        let mut buf = Cursor::new(data);

        unsafe {
            let header_ptr = &mut header as *mut ImageHeader as *mut u8;
            buf.read_exact(std::slice::from_raw_parts_mut(header_ptr, header_size))
                .unwrap();
        }

        header
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

    pub fn decode(data: Vec<u8>) -> Self {
        let header = ImageHeader::decode(data.clone());
        let header_size = std::mem::size_of::<ImageHeader>();
        let data_size = u32::from_le_bytes(data[header_size..header_size + 4].try_into().unwrap());
        let data = data[header_size + 4..].to_vec();

        Self {
            header,
            data_size,
            data,
        }
    }
}

impl ColorFormat {
    /// Get the number of bits per pixel
    pub fn get_bpp(&self) -> u16 {
        match self {
            ColorFormat::L8 => 8,
            ColorFormat::I1 => 1,
            ColorFormat::I2 => 2,
            ColorFormat::I4 => 4,
            ColorFormat::I8 => 8,
            ColorFormat::A8 => 8,
            ColorFormat::RGB565 => 16,
            ColorFormat::RGB565A8 => 24,
            ColorFormat::RGB888 => 24,
            ColorFormat::ARGB8888 => 32,
            ColorFormat::XRGB8888 => 32,
            ColorFormat::A1 => 1,
            ColorFormat::A2 => 2,
            ColorFormat::A4 => 4,
        }
    }

    pub fn get_size(&self) -> u16 {
        let size = self.get_bpp() / 8;
        if size == 0 {
            1
        } else {
            size
        }
    }
}

pub(crate) fn common_decode_function(data: Vec<u8>, color_format: ColorFormat) -> MiData {
    let img_desc = ImageDescriptor::decode(data);

    assert_eq!(img_desc.header.cf, color_format, "Color format mismatch");

    let img_buffer = RgbaImage::from_vec(
        img_desc.header.h as u32,
        img_desc.header.w as u32,
        rgba8888_from(img_desc.data.clone().as_mut(), color_format),
    )
    .unwrap();

    MiData::RGBA(img_buffer)
}

pub(crate) fn common_encode_function(data: &MiData, color_format: ColorFormat) -> Vec<u8> {
    match data {
        MiData::RGBA(img) => {
            let mut img_data = img.clone();
            let img_data = rgba8888_to(img_data.as_mut(), color_format);

            let mut buf = Cursor::new(Vec::new());
            buf.write_all(
                &ImageDescriptor::new(
                    ImageHeader::new(
                        color_format,
                        Flags::NONE,
                        img.width() as u16,
                        img.height() as u16,
                        img.width() as u16 * color_format.get_size(),
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
