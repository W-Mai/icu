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

    /// LVGL image formats
    LVGL,
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
            ImageFormats::LVGL => &icu_lib::endecoder::lvgl_v9::LVGL {} as &dyn EnDecoder,
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
