use clap::{Parser, ValueEnum};

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

#[derive(Parser, Debug)]
#[command(author, version, about)]
#[command(about = "Convert image files to any other image format including LVGL image formats.")]
pub struct Args {
    /// input files
    #[arg(short = 'i', long, required = true, value_hint = clap::ValueHint::FilePath)]
    pub(crate) input_files: Vec<String>,

    /// input image formats
    #[arg(short = 'f', long, value_enum, default_value = "common")]
    pub(crate) input_format: ImageFormatCategory,

    /// output image formats
    #[arg(short = 't', long, value_enum)]
    pub(crate) output_format: ImageFormats,

    /// lvgl version
    #[arg(short = 'l', long, value_enum, default_value = "v9")]
    pub(crate) lvgl_version: LVGL_Version,

    /// verbose mode
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub(crate) verbose: u8,
}

pub fn parse_args() -> Args {
    Args::parse()
}