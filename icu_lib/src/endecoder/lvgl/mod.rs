use modular_bitfield::prelude::*;
use std::io::{Cursor, Write};

mod color_converter;
mod lvgl;

#[derive(Specifier)]
#[bits = 8]
#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[repr(u8)]
pub enum LVGLVersion {
    #[default]
    Unknown,

    V8,
    V9,
}

#[derive(Specifier)]
#[bits = 8]
#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[repr(u8)]
pub enum ColorFormat {
    // Unknown
    #[default]
    UNKNOWN = 0x00,

    // V8 formats
    TrueColor = 0x04,
    TrueColorAlpha = 0x05,

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

pub struct LVGL {}

#[derive(Specifier)]
#[bits = 16]
#[derive(Copy, Clone, Debug)]
#[repr(u16)]
pub enum HeaderFlag {
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

type Flags = u16;

#[derive(Specifier)]
#[bits = 4]
#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[repr(u8)]
pub enum Compress {
    #[default]
    NONE = 0,
    Rle = 1, // LVGL custom RLE compression
    LZ4 = 2,
}

#[bitfield]
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct ImageCompressedHeader {
    method: Compress, /*Compression method, see `lv_image_compress_t`*/

    #[allow(unused)]
    reserved: B28, /*Reserved to be used later*/
    compressed_size: u32,   /*Compressed data size in byte*/
    decompressed_size: u32, /*Decompressed data size in byte*/
}

#[bitfield]
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct ImageHeaderV8 {
    cf: ColorFormat,
    reserved: B2,
    w: B11,
    h: B11,
}

#[bitfield]
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct ImageHeaderV9 {
    // Magic number. Must be LV_IMAGE_HEADER_MAGIC
    #[allow(unused)]
    magic: B8,
    // Color format: See `lv_color_format_t`
    cf: ColorFormat,
    // Image flags, see `lv_image_flags_t`
    flags: Flags,

    // Width of the image in pixels
    w: B16,
    // Height of the image in pixels
    h: B16,
    // Number of bytes in a row
    stride: B16,
    // Reserved to be used later
    reserved_2: B16,
}

#[derive(Debug)]
pub enum ImageHeader {
    Unknown,
    V8(ImageHeaderV8),
    V9(ImageHeaderV9),
}

pub fn has_flag(flags: Flags, flag: HeaderFlag) -> bool {
    flags & flag as u16 != 0
}

pub fn with_flag(flags: Flags, flag: HeaderFlag) -> Flags {
    flags | flag as u16
}

impl ImageHeader {
    pub fn from_bytes(data: &[u8]) -> Self {
        assert!(data.len() > 4, "Invalid data size");
        let magic = data[0];

        let mut version = LVGLVersion::Unknown;

        if magic == 0x19 {
            version = LVGLVersion::V9;
        } else if magic <= 0x18 {
            version = LVGLVersion::V8;
        }

        match version {
            LVGLVersion::V8 => {
                let header = ImageHeaderV8::from_bytes([data[0], data[1], data[2], data[3]]);
                log::trace!("Decoded image header: {:#?}", header);
                if header.cf_or_err().is_err() || header.reserved() != 0 {
                    ImageHeader::Unknown
                } else {
                    ImageHeader::V8(header)
                }
            }
            LVGLVersion::V9 => {
                let header = ImageHeaderV9::from_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                    data[8], data[9], data[10], data[11],
                ]);
                if header.cf_or_err().is_err() || header.reserved_2() != 0 {
                    ImageHeader::Unknown
                } else {
                    ImageHeader::V9(header)
                }
            }
            _ => ImageHeader::Unknown,
        }
    }

    pub fn into_bytes(&self) -> Vec<u8> {
        match self {
            ImageHeader::Unknown => vec![],
            ImageHeader::V8(header) => header.into_bytes().to_vec(),
            ImageHeader::V9(header) => header.into_bytes().to_vec(),
        }
    }

    pub fn header_size(&self) -> usize {
        match self {
            ImageHeader::Unknown => 0,
            ImageHeader::V8(_) => size_of::<ImageHeaderV8>(),
            ImageHeader::V9(_) => size_of::<ImageHeaderV9>(),
        }
    }

    pub fn version(&self) -> LVGLVersion {
        match self {
            ImageHeader::Unknown => LVGLVersion::Unknown,
            ImageHeader::V8(_) => LVGLVersion::V8,
            ImageHeader::V9(_) => LVGLVersion::V9,
        }
    }

    pub fn flags(&self) -> Flags {
        match self {
            ImageHeader::Unknown => 0,
            ImageHeader::V8(_) => 0,
            ImageHeader::V9(header) => header.flags(),
        }
    }

    pub fn cf(&self) -> ColorFormat {
        match self {
            ImageHeader::Unknown => ColorFormat::UNKNOWN,
            ImageHeader::V8(header) => header.cf(),
            ImageHeader::V9(header) => header.cf(),
        }
    }

    pub fn w(&self) -> u16 {
        match self {
            ImageHeader::Unknown => 0,
            ImageHeader::V8(header) => header.w(),
            ImageHeader::V9(header) => header.w(),
        }
    }

    pub fn h(&self) -> u16 {
        match self {
            ImageHeader::Unknown => 0,
            ImageHeader::V8(header) => header.h(),
            ImageHeader::V9(header) => header.h(),
        }
    }

    pub fn stride(&self) -> u16 {
        match self {
            ImageHeader::Unknown => 0,
            ImageHeader::V8(_) => self.cf().get_stride_size(self.w() as u32, 1) as u16,
            ImageHeader::V9(header) => header.stride(),
        }
    }
}

impl ImageHeader {
    pub fn new(
        version: LVGLVersion,
        cf: ColorFormat,
        flags: Flags,
        w: u16,
        h: u16,
        stride: u16,
    ) -> Self {
        match version {
            LVGLVersion::V8 => {
                ImageHeader::V8(ImageHeaderV8::new().with_cf(cf).with_w(w).with_h(h))
            }
            LVGLVersion::V9 => ImageHeader::V9(
                ImageHeaderV9::new()
                    .with_magic(0x19)
                    .with_cf(cf)
                    .with_flags(flags)
                    .with_w(w)
                    .with_h(h)
                    .with_stride(stride),
            ),
            LVGLVersion::Unknown => ImageHeader::Unknown,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        self.into_bytes()
    }

    pub fn decode(data: Vec<u8>) -> Self {
        log::trace!("Decoding image header with data size: {}", data.len());

        let header = ImageHeader::from_bytes(data.as_slice());

        log::trace!("Decoded image header: {:#?}", header);
        header
    }
}

#[allow(dead_code)]
#[derive(Debug)]
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
        buf.write_all(self.data.as_slice()).unwrap();
        buf.into_inner()
    }

    pub fn decode(data: Vec<u8>) -> Self {
        log::trace!("Decoding image descriptor with data size: {}", data.len());

        let header = ImageHeader::decode(data.clone());
        let header_size = header.header_size();
        let data = data[header_size..].to_vec();
        let data_size = data.len() as u32;

        match header {
            ImageHeader::V9(header) => {
                let stride = if header.stride() == 0 {
                    let assuming_stride = header.cf().get_stride_size(header.w() as u32, 1);
                    log::error!("Invalid image header, stride is 0, assuming stride to be width * color_format.byte() = {assuming_stride}");
                    assuming_stride
                } else {
                    header.stride() as u32
                };

                let mut idea_data_size = stride * header.h() as u32;
                idea_data_size += match header.cf() {
                    ColorFormat::I1 | ColorFormat::I2 | ColorFormat::I4 | ColorFormat::I8 => {
                        (1u32 << header.cf().get_bpp()) * ColorFormat::ARGB8888.get_size() as u32
                    }
                    ColorFormat::RGB565A8 => header.w() as u32 * header.h() as u32,
                    _ => 0,
                };

                if has_flag(header.flags(), HeaderFlag::COMPRESSED) {
                    log::trace!("Dealing Compressed image");
                    let compressed_header = ImageCompressedHeader::from_bytes([
                        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                        data[8], data[9], data[10], data[11],
                    ]);
                    let method = compressed_header.method();
                    match method {
                        Compress::Rle => {
                            let blk_size = ((header.cf().get_bpp() + 7) >> 3) as usize;
                            use super::utils::rle::RleCoder;
                            let rle_coder = RleCoder::new().with_block_size(blk_size).unwrap();
                            if compressed_header.compressed_size()
                                != data_size - size_of::<ImageCompressedHeader>() as u32
                            {
                                log::error!(
                                    "Compressed data size mismatch, but still try to decode. current: {} expected {}",
                                    compressed_header.compressed_size(),
                                    data_size - size_of::<ImageCompressedHeader>() as u32
                                );
                            }
                            let decoded =
                                rle_coder.decode(&data[size_of::<ImageCompressedHeader>()..]);
                            match decoded {
                                Ok(decoded) => {
                                    return Self {
                                        header: ImageHeader::V9(header),
                                        data_size: decoded.len() as u32,
                                        data: decoded,
                                    };
                                }
                                Err(err) => {
                                    log::error!("Failed to decode RLE data: {:?}", err);
                                }
                            };
                        }
                        _ => {
                            log::error!("Unsupported compression method {:?}", method)
                        }
                    }
                    return Self {
                        header: ImageHeader::V9(header),
                        data_size: 0,
                        data: vec![],
                    };
                } else if idea_data_size != data_size {
                    log::error!("Data size mismatch ideal_data_size: {idea_data_size}, data_size: {data_size}, {:#?}", header);
                }
            }
            ImageHeader::V8(_) => {}
            ImageHeader::Unknown => {
                log::error!("Unknown image header format");

                return Self {
                    header,
                    data_size: 0,
                    data: vec![],
                };
            }
        }

        log::trace!(
            "Decoded image descriptor and returned data size: {}",
            data_size
        );

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
            ColorFormat::UNKNOWN => 0,
            ColorFormat::L8 => 8,
            ColorFormat::I1 => 1,
            ColorFormat::I2 => 2,
            ColorFormat::I4 => 4,
            ColorFormat::I8 => 8,
            ColorFormat::A8 => 8,
            ColorFormat::RGB565 => 16,
            ColorFormat::RGB565A8 => 16,
            ColorFormat::RGB888 => 24,
            ColorFormat::ARGB8888 => 32,
            ColorFormat::XRGB8888 => 32,
            ColorFormat::A1 => 1,
            ColorFormat::A2 => 2,
            ColorFormat::A4 => 4,
            ColorFormat::TrueColor => ColorFormat::XRGB8888.get_bpp(),
            ColorFormat::TrueColorAlpha => ColorFormat::ARGB8888.get_bpp(),
        }
    }

    pub fn get_size(&self) -> u16 {
        (self.get_bpp() + 7) >> 3
    }

    pub fn get_stride_size(&self, width: u32, align: u32) -> u32 {
        let stride = (width * self.get_bpp() as u32 + 7) >> 3;
        (stride + align - 1) & !(align - 1)
    }
}
