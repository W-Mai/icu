use modular_bitfield::prelude::*;
use std::io::{Cursor, Write};

pub mod color_converter;
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

const MAX_IMAGE_DATA_SIZE: usize = 256 * 1024 * 1024;

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
    #[allow(unused)]
    method: Compress, /*Compression method, see `lv_image_compress_t`*/

    #[allow(unused)]
    reserved: B28, /*Reserved to be used later*/
    compressed_size: u32,   /*Compressed data size in byte*/
    decompressed_size: u32, /*Decompressed data size in byte*/
}

#[derive(Debug)]
struct CompressedImage<'a> {
    method: Compress,
    decompressed_size: usize,
    payload: &'a [u8],
}

impl<'a> CompressedImage<'a> {
    fn parse(data: &'a [u8], expected_size: usize) -> Option<Self> {
        let header_size = size_of::<ImageCompressedHeader>();
        let header = ImageCompressedHeader::from_bytes(data.get(..header_size)?.try_into().ok()?);
        let payload = data.get(header_size..)?;
        let compressed_size = usize::try_from(header.compressed_size()).ok()?;
        let decompressed_size = usize::try_from(header.decompressed_size()).ok()?;

        if header.reserved() != 0
            || compressed_size != payload.len()
            || decompressed_size != expected_size
        {
            return None;
        }

        Some(Self {
            method: header.method_or_err().ok()?,
            decompressed_size,
            payload,
        })
    }

    fn encode(method: Compress, raw: &[u8], block_size: usize) -> Option<Vec<u8>> {
        let compressed = match method {
            Compress::NONE => return Some(raw.to_vec()),
            Compress::Rle => super::utils::rle::RleCoder::new()
                .with_block_size(block_size)
                .ok()?
                .encode(raw)
                .ok()?,
            Compress::LZ4 => lz4_flex::block::compress(raw),
        };
        let compressed_size = u32::try_from(compressed.len()).ok()?;
        let decompressed_size = u32::try_from(raw.len()).ok()?;
        let header = ImageCompressedHeader::new()
            .with_method(method)
            .with_compressed_size(compressed_size)
            .with_decompressed_size(decompressed_size);
        let mut data = header.into_bytes().to_vec();
        data.extend_from_slice(&compressed);
        Some(data)
    }

    fn info(&self) -> (Compress, usize, usize) {
        (self.method, self.payload.len(), self.decompressed_size)
    }

    fn decode(&self, block_size: usize) -> Option<Vec<u8>> {
        let decoded = match self.method {
            Compress::NONE => return None,
            Compress::Rle => super::utils::rle::RleCoder::new()
                .with_block_size(block_size)
                .ok()?
                .decode(self.payload)
                .ok()?,
            Compress::LZ4 => {
                let mut decoded = vec![0; self.decompressed_size];
                let len = lz4_flex::block::decompress_into(self.payload, &mut decoded).ok()?;
                (len == self.decompressed_size).then_some(decoded)?
            }
        };
        (decoded.len() == self.decompressed_size).then_some(decoded)
    }
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
    pub fn parse(data: &[u8]) -> Option<Self> {
        let version = match *data.first()? {
            0x19 => LVGLVersion::V9,
            magic if magic <= 0x18 => LVGLVersion::V8,
            _ => return None,
        };

        let header = match version {
            LVGLVersion::V8 => {
                let bytes = data.get(..size_of::<ImageHeaderV8>())?.try_into().ok()?;
                let header = ImageHeaderV8::from_bytes(bytes);
                (header.cf_or_err().is_ok() && header.reserved() == 0)
                    .then_some(ImageHeader::V8(header))
            }
            LVGLVersion::V9 => {
                let bytes = data.get(..size_of::<ImageHeaderV9>())?.try_into().ok()?;
                let header = ImageHeaderV9::from_bytes(bytes);
                (header.cf_or_err().is_ok() && header.reserved_2() == 0)
                    .then_some(ImageHeader::V9(header))
            }
            LVGLVersion::Unknown => None,
        }?;

        log::trace!("Decoded image header: {header:#?}");
        Some(header)
    }

    pub fn split(data: &[u8]) -> Option<(Self, &[u8])> {
        let header = Self::parse(data)?;
        let payload = data.get(header.header_size()..)?;
        Some((header, payload))
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

    pub fn expected_data_size(&self) -> Option<usize> {
        let stride = if self.stride() == 0 {
            self.cf().get_stride_size(self.w() as u32, 1) as usize
        } else {
            self.stride() as usize
        };
        let pixels = stride.checked_mul(self.h() as usize)?;
        let extra = match self.cf() {
            ColorFormat::I1 | ColorFormat::I2 | ColorFormat::I4 | ColorFormat::I8 => (1usize
                << self.cf().get_bpp())
            .checked_mul(ColorFormat::ARGB8888.get_size() as usize)?,
            ColorFormat::RGB565A8 => (self.w() as usize).checked_mul(self.h() as usize)?,
            _ => 0,
        };
        let size = pixels.checked_add(extra)?;
        (size <= MAX_IMAGE_DATA_SIZE).then_some(size)
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
        Self::parse(&data).unwrap_or(ImageHeader::Unknown)
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

        let Some((header, payload)) = ImageHeader::split(&data) else {
            log::error!("Invalid LVGL image header");
            return Self::invalid(ImageHeader::Unknown);
        };
        let Some(expected_size) = header.expected_data_size() else {
            log::error!("LVGL image exceeds the supported data size");
            return Self::invalid(header);
        };
        let image_data = if has_flag(header.flags(), HeaderFlag::COMPRESSED) {
            let block_size = ((header.cf().get_bpp() + 7) >> 3) as usize;
            CompressedImage::parse(payload, expected_size)
                .and_then(|compressed| compressed.decode(block_size))
        } else {
            Some(payload.to_vec())
        };

        let Some(image_data) = image_data else {
            log::error!("Invalid LVGL compressed image data");
            return Self::invalid(header);
        };
        if image_data.len() != expected_size {
            log::error!(
                "Image data size mismatch: actual={}, expected={expected_size}",
                image_data.len()
            );
            return Self::invalid(header);
        }

        Self::new(header, image_data)
    }

    fn invalid(header: ImageHeader) -> Self {
        Self {
            header,
            data_size: 0,
            data: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_header_parsing_is_safe_for_truncated_input() {
        for len in 0..12 {
            let mut data = vec![0; len];
            if let Some(magic) = data.first_mut() {
                *magic = 0x19;
            }
            assert!(ImageHeader::parse(&data).is_none());
        }
    }

    #[test]
    fn image_header_parses_exact_v8_and_v9_headers() {
        let v8 = ImageHeader::new(LVGLVersion::V8, ColorFormat::TrueColor, 0, 8, 4, 0);
        let v9 = ImageHeader::new(LVGLVersion::V9, ColorFormat::I8, 0, 8, 4, 8);

        assert_eq!(
            ImageHeader::parse(&v8.encode()).unwrap().version(),
            LVGLVersion::V8
        );
        assert_eq!(
            ImageHeader::parse(&v9.encode()).unwrap().version(),
            LVGLVersion::V9
        );
    }

    #[test]
    fn image_header_split_returns_payload_after_actual_header() {
        for header in [
            ImageHeader::new(LVGLVersion::V8, ColorFormat::TrueColor, 0, 8, 4, 0),
            ImageHeader::new(LVGLVersion::V9, ColorFormat::I8, 0, 8, 4, 8),
        ] {
            let mut data = header.encode();
            data.extend_from_slice(&[1, 2, 3]);
            let (parsed, payload) = ImageHeader::split(&data).unwrap();
            assert_eq!(parsed.version(), header.version());
            assert_eq!(payload, &[1, 2, 3]);
        }
    }
}
