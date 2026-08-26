use clap::error::ErrorKind;
use clap::{Command, CommandFactory, Parser, Subcommand};
use std::io;

pub use crate::converter::{
    ImageFormatCategory, ImageFormats, LVGL_Version, OutputColorFormats, OutputCompressedMethod,
    OutputFileFormatCategory,
};

#[derive(Parser, Debug)]
#[command(author, version, long_about)]
#[command(
    name = "icu",
    about = "`Show` or `Convert` image files to any other image format including LVGL image formats."
)]
pub struct Args {
    #[command(subcommand)]
    pub(crate) commands: Option<SubCommands>,

    /// Generate auto-completion script for the specified shell
    #[arg(short = 'I', long, value_name = "SHELL", value_enum)]
    pub(crate) init: Option<clap_complete::Shell>,

    /// verbose mode
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub(crate) verbose: u8,
}

#[derive(Subcommand, Debug)]
pub(crate) enum SubCommands {
    /// Show some information about an image file
    Info {
        /// an image file to show
        #[arg(required = true, value_hint = clap::ValueHint::FilePath)]
        file: String,

        /// input image formats
        #[arg(short = 'f', long, value_enum, default_value = "auto")]
        input_format: ImageFormatCategory,
    },

    /// Show an image file
    Show {
        /// an image file to show
        #[arg(value_hint = clap::ValueHint::FilePath)]
        files: Option<Vec<String>>,

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

        #[arg(long, value_enum)]
        output_compressed_method: Option<OutputCompressedMethod>,

        /// Output converted result to stdout
        #[arg(long)]
        stdout: bool,

        /// dither the output image so that it will look better on screens with low color depth
        /// 1 to 30, 1 is the best quality and 30 is the worst quality.
        /// 10 is recommended.
        #[arg(long)]
        dither: Option<u32>,

        /// LVGL Version, needed if [`ImageFormats`] is [`ImageFormats::LVGL`]
        #[arg(long, value_enum, default_value = "v9")]
        lvgl_version: LVGL_Version,

        /// PNG output color mode
        #[arg(long, value_enum)]
        png_mode: Option<PngMode>,

        /// PNG compression level
        #[arg(long, value_enum)]
        png_compression: Option<PngCompressionMode>,

        /// JPEG quality from 1 to 100
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
        quality: Option<u8>,

        /// JPEG background color used when flattening alpha, as #RRGGBB
        #[arg(long, value_parser = parse_hex_color)]
        background: Option<[u8; 3]>,
    },

    /// Bake a TTF/OTF font into a mirx FONT chunk (SDF or grayscale atlas)
    BakeFont {
        #[arg(value_hint = clap::ValueHint::FilePath)]
        ttf: String,

        #[arg(long, default_value = " ")]
        charset: String,

        #[arg(long, value_hint = clap::ValueHint::FilePath)]
        charset_file: Option<String>,

        #[arg(long, default_value_t = 24)]
        size: u16,

        #[arg(long, default_value_t = 4)]
        bit_depth: u8,

        #[arg(long)]
        spread: Option<u16>,

        #[arg(long, value_enum, default_value = "sdf")]
        format: BakeFormat,

        #[arg(short = 'O', long, value_hint = clap::ValueHint::DirPath)]
        output_folder: Option<String>,

        #[arg(short = 'r', long)]
        override_output: bool,
    },

    /// Merge multiple mirx font files into one multi-FONT-chunk bundle
    MergeFonts {
        #[arg(required = true, value_hint = clap::ValueHint::FilePath)]
        inputs: Vec<String>,

        #[arg(short = 'O', long, value_hint = clap::ValueHint::FilePath)]
        output: String,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PngMode {
    Rgba,
    Rgb,
    Preserve,
    Indexed1,
    Indexed2,
    Indexed4,
    Indexed8,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PngCompressionMode {
    Fast,
    Balanced,
    Best,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BakeFormat {
    Sdf,
    Gray,
}

fn parse_hex_color(value: &str) -> Result<[u8; 3], String> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 {
        return Err("background must use the #RRGGBB format".to_string());
    }
    let parse = |range| {
        u8::from_str_radix(&hex[range], 16)
            .map_err(|_| "background must use the #RRGGBB format".to_string())
    };
    Ok([parse(0..2)?, parse(2..4)?, parse(4..6)?])
}

pub fn parse_args() -> Args {
    let mut command = Args::command();
    let args = Args::parse();

    if let Some(generator) = &args.init {
        let mut cmd = Args::command();
        fn print_completions<G: clap_complete::Generator>(g: G, cmd: &mut Command) {
            clap_complete::generate(g, cmd, cmd.get_name().to_string(), &mut io::stdout());
        }
        print_completions(*generator, &mut cmd);
        std::process::exit(0);
    }

    if let Some(sub_commands) = &args.commands {
        match sub_commands {
            SubCommands::Show { .. } | SubCommands::Info { .. } => {}
            SubCommands::Convert {
                output_format,
                output_color_format,
                dither,
                png_mode,
                png_compression,
                quality,
                background,
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
                if output_format == &ImageFormats::LVGL
                    && output_color_format.is_some_and(|format| !format.supports_lvgl())
                {
                    command
                        .error(
                            ErrorKind::InvalidValue,
                            "The selected output color format is not supported by LVGL.",
                        )
                        .exit();
                }
                if let Some(dither) = *dither {
                    if !(1..=30).contains(&dither) {
                        let error = command.error(
                            ErrorKind::InvalidValue,
                            "Dither value must be between 1 and 30.",
                        );
                        error.exit();
                    }
                }
                if output_format != &ImageFormats::PNG
                    && (png_mode.is_some() || png_compression.is_some())
                {
                    command
                        .error(
                            ErrorKind::InvalidValue,
                            "--png-mode and --png-compression require PNG output.",
                        )
                        .exit();
                }
                if output_format != &ImageFormats::JPEG
                    && (quality.is_some() || background.is_some())
                {
                    command
                        .error(
                            ErrorKind::InvalidValue,
                            "--quality and --background require JPEG output.",
                        )
                        .exit();
                }
            }
            SubCommands::BakeFont { .. } | SubCommands::MergeFonts { .. } => {}
        }
    } else {
        command.flatten_help(true).print_long_help().unwrap();
        std::process::exit(0);
    }

    args
}
