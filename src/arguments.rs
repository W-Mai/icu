use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use icu_lib::endecoder::{lvgl_v9, EnDecoder};

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum, Default)]
pub(crate) enum ImageFormatCategory {
    /// Auto-detect the input image format.
    #[default]
    Auto,

    /// Common image formats like: PNG, JPEG, BMP, etc.
    Common,

    /// LVGL image formats like: RGB565, RGB888, ARGB8888, etc.
    LVGL_V9,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum)]
pub(crate) enum OutputFileFormatCategory {
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
    V8,
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

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, ValueEnum)]
pub(crate) enum OutputColorFormats {
    /// Color formats: RGB565, RGB888, ARGB8888, etc.
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

impl From<OutputColorFormats> for lvgl_v9::ColorFormat {
    fn from(color_format: OutputColorFormats) -> lvgl_v9::ColorFormat {
        match color_format {
            OutputColorFormats::RGB565 => lvgl_v9::ColorFormat::RGB565,
            OutputColorFormats::RGB565A8 => lvgl_v9::ColorFormat::RGB565A8,
            OutputColorFormats::RGB888 => lvgl_v9::ColorFormat::RGB888,
            OutputColorFormats::ARGB8888 => lvgl_v9::ColorFormat::ARGB8888,
            OutputColorFormats::XRGB8888 => lvgl_v9::ColorFormat::XRGB8888,
            OutputColorFormats::A1 => lvgl_v9::ColorFormat::A1,
            OutputColorFormats::A2 => lvgl_v9::ColorFormat::A2,
            OutputColorFormats::A4 => lvgl_v9::ColorFormat::A4,
            OutputColorFormats::A8 => lvgl_v9::ColorFormat::A8,
            OutputColorFormats::L8 => lvgl_v9::ColorFormat::L8,
            OutputColorFormats::I1 => lvgl_v9::ColorFormat::I1,
            OutputColorFormats::I2 => lvgl_v9::ColorFormat::I2,
            OutputColorFormats::I4 => lvgl_v9::ColorFormat::I4,
            OutputColorFormats::I8 => lvgl_v9::ColorFormat::I8,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, long_about)]
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
        #[arg(short = 'f', long, value_enum, default_value = "auto")]
        input_format: ImageFormatCategory,
    },

    /// Convert image files to any other image format including LVGL image formats.
    Convert {
        /// input files
        #[arg(required = true, value_hint = clap::ValueHint::FilePath)]
        input_files: Vec<String>,

        /// input image formats
        #[arg(short = 'f', long, value_enum, default_value = "auto")]
        input_format: ImageFormatCategory,

        /// output folder
        #[arg(short = 'O', long, value_hint = clap::ValueHint::DirPath)]
        output_folder: Option<String>,

        /// override exist output files, and you will get a warning message for sure if the output file already exists.
        #[arg(short = 'r', long)]
        override_output: bool,

        /// output image format categories
        #[arg(short = 'G', long, value_enum, default_value = "common")]
        output_category: OutputFileFormatCategory,

        /// output image formats
        #[arg(short = 'F', long, value_enum)]
        output_format: ImageFormats,

        /// stride of the output image
        #[arg(short = 'S', long, default_value = "1")]
        output_stride_align: u32,

        /// output color formats
        #[arg(short = 'C', long, value_enum)]
        output_color_format: Option<OutputColorFormats>,

        /// dither the output image so that it will look better on screens with low color depth
        #[arg(long)]
        dither: bool,

        /// LVGL Version, needed if [`ImageFormats`] is [`ImageFormats::LVGL`]
        #[arg(long, value_enum, default_value = "v9")]
        lvgl_version: LVGL_Version,
    },
}

pub fn parse_args() -> Args {
    let mut command = Args::command();
    let args = Args::parse();

    match &args.commands {
        SubCommands::Show { .. } => {}
        SubCommands::Convert {
            output_format,
            output_color_format,
            ..
        } => {
            if output_format == &ImageFormats::LVGL && output_color_format.is_none() {
                let error = command.error(
                    ErrorKind::MissingRequiredArgument,
                    "Output color format is required for LVGL image format. \
                 Please specify it using the [-C --output-color-format] option.",
                );

                error.exit();
            }
        }
    }

    args
}
