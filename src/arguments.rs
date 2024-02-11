use clap::{Parser, Subcommand, ValueEnum};
use icu_lib::endecoder::EnDecoder;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum)]
pub(crate) enum ImageFormatCategory {
    /// Common image formats like: PNG, JPEG, BMP, etc.
    Common,

    /// LVGL image formats like: RGB565, RGB888, ARGB8888, etc.
    LVGL_V9,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum)]
pub(crate) enum ImageOutputFormatCategory {
    /// Common image formats like: PNG, JPEG, BMP, etc.
    Common,

    /// Bin formats.
    Bin,

    /// C Array format.
    C_Array,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum)]
pub(crate) enum LVGL_Version {
    V9,
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum)]
pub(crate) enum ImageFormats {
    /// Common image formats like: PNG, JPEG, BMP, etc.
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

    /// LVGL image formats like: RGB565, RGB888, ARGB8888, etc.
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
            ImageFormats::RGB565 => {
                &icu_lib::endecoder::lvgl_v9::ColorFormatRGB565 {} as &dyn EnDecoder
            }
            ImageFormats::RGB565A8 => {
                &icu_lib::endecoder::lvgl_v9::ColorFormatRGB565A8 {} as &dyn EnDecoder
            }
            ImageFormats::RGB888 => {
                &icu_lib::endecoder::lvgl_v9::ColorFormatRGB888 {} as &dyn EnDecoder
            }
            ImageFormats::ARGB8888 => {
                &icu_lib::endecoder::lvgl_v9::ColorFormatARGB8888 {} as &dyn EnDecoder
            }
            ImageFormats::XRGB8888 => {
                &icu_lib::endecoder::lvgl_v9::ColorFormatXRGB8888 {} as &dyn EnDecoder
            }
            ImageFormats::A1 => &icu_lib::endecoder::lvgl_v9::ColorFormatA1 {} as &dyn EnDecoder,
            ImageFormats::A2 => &icu_lib::endecoder::lvgl_v9::ColorFormatA2 {} as &dyn EnDecoder,
            ImageFormats::A4 => &icu_lib::endecoder::lvgl_v9::ColorFormatA4 {} as &dyn EnDecoder,
            ImageFormats::A8 => &icu_lib::endecoder::lvgl_v9::ColorFormatA8 {} as &dyn EnDecoder,
            ImageFormats::L8 => &icu_lib::endecoder::lvgl_v9::ColorFormatL8 {} as &dyn EnDecoder,
            ImageFormats::I1 => &icu_lib::endecoder::lvgl_v9::ColorFormatI1 {} as &dyn EnDecoder,
            ImageFormats::I2 => &icu_lib::endecoder::lvgl_v9::ColorFormatI2 {} as &dyn EnDecoder,
            ImageFormats::I4 => &icu_lib::endecoder::lvgl_v9::ColorFormatI4 {} as &dyn EnDecoder,
            ImageFormats::I8 => &icu_lib::endecoder::lvgl_v9::ColorFormatI8 {} as &dyn EnDecoder,
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
            ImageFormats::RGB565
            | ImageFormats::RGB565A8
            | ImageFormats::RGB888
            | ImageFormats::ARGB8888
            | ImageFormats::XRGB8888
            | ImageFormats::A1
            | ImageFormats::A2
            | ImageFormats::A4
            | ImageFormats::A8
            | ImageFormats::L8
            | ImageFormats::I1
            | ImageFormats::I2
            | ImageFormats::I4
            | ImageFormats::I8 => "bin",
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
#[command(
    about = "`Show` or `Convert` image files to any other image format including LVGL image formats."
)]
pub struct Args {
    #[command(subcommand)]
    pub(crate) commands: SubCommands,

    /// verbose mode
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub(crate) verbose: u8,
}

#[derive(Subcommand, Debug)]
pub(crate) enum SubCommands {
    /// Show an image file
    Show {
        /// an image file to show
        file: String,

        /// input image formats
        #[arg(short = 'f', long, value_enum, default_value = "common")]
        input_format: ImageFormatCategory,
    },

    /// Convert image files to any other image format including LVGL image formats.
    Convert {
        /// input files
        #[arg(short = 'i', long, required = true, value_hint = clap::ValueHint::FilePath)]
        input_files: Vec<String>,

        /// input image formats
        #[arg(short = 'f', long, value_enum, default_value = "common")]
        input_format: ImageFormatCategory,

        /// output image format categories
        #[arg(short = 'c', long, value_enum)]
        output_category: ImageOutputFormatCategory,

        /// output image formats
        #[arg(short = 't', long, value_enum)]
        output_format: ImageFormats,

        /// lvgl version
        #[arg(short = 'l', long, value_enum, default_value = "v9")]
        lvgl_version: LVGL_Version,
    },
}

pub fn parse_args() -> Args {
    Args::parse()
}
