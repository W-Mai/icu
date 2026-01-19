use clap::ValueEnum;
use icu_lib::endecoder::EnDecoder;
use serde::{Deserialize, Serialize};

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum, Default, Serialize, Deserialize)]
pub enum ImageFormatCategory {
    /// Auto-detect the input image format.
    #[default]
    Auto,

    /// Common image formats like: PNG, JPEG, BMP, etc.
    Common,

    /// LVGL image formats like: RGB565, RGB888, ARGB8888, etc.
    LVGL_V9,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum, Serialize, Deserialize)]
pub enum OutputFileFormatCategory {
    /// Common image formats like: PNG, JPEG, BMP, etc.
    Common,

    /// Bin formats.
    Bin,

    /// C Array format.
    C_Array,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum, Serialize, Deserialize)]
pub enum LVGL_Version {
    V9,
    V8,
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum, Serialize, Deserialize)]
pub enum ImageFormats {
    // Common image formats like: PNG, JPEG, BMP, etc.
    PNG,
    JPEG,
    BMP,
    GIF,
    TIFF,
    WEBP,
    ICO,
    PBM,
    PGM,
    PPM,
    PAM,

    /// LVGL image formats
    LVGL,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum, Serialize, Deserialize)]
pub enum OutputColorFormats {
    // Color formats: RGB565, RGB888, ARGB8888, etc.
    RGB565,
    RGB565A8,
    RGB888,
    ARGB8888,
    XRGB8888,
    A1,
    A2,
    A4,
    A8,
    L8,
    I1,
    I2,
    I4,
    I8,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum, Serialize, Deserialize)]
pub enum OutputCompressedMethod {
    None,
    Rle,
    LZ4,
}

impl ImageFormats {
    pub fn get_endecoder(&self) -> &dyn EnDecoder {
        match &self {
            ImageFormats::PNG => &icu_lib::endecoder::common::PNG {} as &dyn EnDecoder,
            ImageFormats::JPEG => &icu_lib::endecoder::common::JPEG {} as &dyn EnDecoder,
            ImageFormats::BMP => &icu_lib::endecoder::common::BMP {} as &dyn EnDecoder,
            ImageFormats::GIF => &icu_lib::endecoder::common::GIF {} as &dyn EnDecoder,
            ImageFormats::TIFF => &icu_lib::endecoder::common::TIFF {} as &dyn EnDecoder,
            ImageFormats::WEBP => &icu_lib::endecoder::common::WEBP {} as &dyn EnDecoder,
            ImageFormats::ICO => &icu_lib::endecoder::common::ICO {} as &dyn EnDecoder,
            ImageFormats::PBM => &icu_lib::endecoder::common::PBM {} as &dyn EnDecoder,
            ImageFormats::PGM => &icu_lib::endecoder::common::PGM {} as &dyn EnDecoder,
            ImageFormats::PPM => &icu_lib::endecoder::common::PPM {} as &dyn EnDecoder,
            ImageFormats::PAM => &icu_lib::endecoder::common::PAM {} as &dyn EnDecoder,
            ImageFormats::LVGL => &icu_lib::endecoder::lvgl::LVGL {} as &dyn EnDecoder,
        }
    }

    pub fn get_file_extension(&self) -> &'static str {
        match &self {
            ImageFormats::PNG => "png",
            ImageFormats::JPEG => "jpeg",
            ImageFormats::BMP => "bmp",
            ImageFormats::GIF => "gif",
            ImageFormats::TIFF => "tiff",
            ImageFormats::WEBP => "webp",
            ImageFormats::ICO => "ico",
            ImageFormats::PBM => "pbm",
            ImageFormats::PGM => "pgm",
            ImageFormats::PPM => "ppm",
            ImageFormats::PAM => "pam",
            ImageFormats::LVGL => "bin",
        }
    }
}

impl From<LVGL_Version> for icu_lib::endecoder::lvgl::LVGLVersion {
    fn from(version: LVGL_Version) -> Self {
        match version {
            LVGL_Version::V9 => icu_lib::endecoder::lvgl::LVGLVersion::V9,
            LVGL_Version::V8 => icu_lib::endecoder::lvgl::LVGLVersion::V8,
        }
    }
}

impl From<OutputColorFormats> for icu_lib::endecoder::lvgl::ColorFormat {
    fn from(format: OutputColorFormats) -> Self {
        match format {
            OutputColorFormats::RGB565 => icu_lib::endecoder::lvgl::ColorFormat::RGB565,
            OutputColorFormats::RGB565A8 => icu_lib::endecoder::lvgl::ColorFormat::RGB565A8,
            OutputColorFormats::RGB888 => icu_lib::endecoder::lvgl::ColorFormat::RGB888,
            OutputColorFormats::ARGB8888 => icu_lib::endecoder::lvgl::ColorFormat::ARGB8888,
            OutputColorFormats::XRGB8888 => icu_lib::endecoder::lvgl::ColorFormat::XRGB8888,
            OutputColorFormats::A1 => icu_lib::endecoder::lvgl::ColorFormat::A1,
            OutputColorFormats::A2 => icu_lib::endecoder::lvgl::ColorFormat::A2,
            OutputColorFormats::A4 => icu_lib::endecoder::lvgl::ColorFormat::A4,
            OutputColorFormats::A8 => icu_lib::endecoder::lvgl::ColorFormat::A8,
            OutputColorFormats::L8 => icu_lib::endecoder::lvgl::ColorFormat::L8,
            OutputColorFormats::I1 => icu_lib::endecoder::lvgl::ColorFormat::I1,
            OutputColorFormats::I2 => icu_lib::endecoder::lvgl::ColorFormat::I2,
            OutputColorFormats::I4 => icu_lib::endecoder::lvgl::ColorFormat::I4,
            OutputColorFormats::I8 => icu_lib::endecoder::lvgl::ColorFormat::I8,
        }
    }
}

impl From<OutputCompressedMethod> for icu_lib::endecoder::lvgl::Compress {
    fn from(method: OutputCompressedMethod) -> Self {
        match method {
            OutputCompressedMethod::None => icu_lib::endecoder::lvgl::Compress::NONE,
            OutputCompressedMethod::Rle => icu_lib::endecoder::lvgl::Compress::Rle,
            OutputCompressedMethod::LZ4 => icu_lib::endecoder::lvgl::Compress::LZ4,
        }
    }
}


