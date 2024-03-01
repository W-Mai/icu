use std::io::{Cursor, Read, Write};

mod color_converter;
mod lvgl;

#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[repr(u8)]
pub enum ColorFormat {
    // Unkonw
    #[default]
    UNKNOWN = 0x00,

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

#[derive(Copy, Clone, Debug)]
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

#[derive(Debug)]
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
        log::trace!("Decoding image header with data size: {}", data.len());

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

        if header.magic != 0x19 {
            log::error!(
                "Invalid magic number in image header with value: {}",
                header.magic
            );
            assert_eq!(header.magic, 0x19, "Invalid magic number in image header");
        }

        log::trace!("Decoded image header: {:#?}", header);
        header
    }
}

/*typedef struct {
    lv_image_header_t header; /**< A header describing the basics of the image*/
    uint32_t data_size;     /**< Size of the image in bytes*/
    const uint8_t * data;   /**< Pointer to the data of the image*/
} lv_image_dsc_t;*/

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
        let header_size = std::mem::size_of::<ImageHeader>();
        let data = data[header_size..].to_vec();
        let data_size = data.len() as u32;

        let mut idea_data_size = header.stride as u32 * header.h as u32;
        idea_data_size += match header.cf {
            ColorFormat::I1 | ColorFormat::I2 | ColorFormat::I4 | ColorFormat::I8 => {
                (1u32 << header.cf.get_bpp()) * ColorFormat::ARGB8888.get_size() as u32
            }
            ColorFormat::RGB565A8 => header.w as u32 * header.h as u32,
            _ => 0,
        };

        assert_eq!(idea_data_size, data_size, "Data size mismatch {:?}", header);

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
