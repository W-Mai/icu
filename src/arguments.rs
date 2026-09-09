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

        /// MIRX sample coding profile
        #[arg(long, value_enum, default_value = "raw")]
        mirx_coding: MirxCodingMode,

        /// MIRX quantized-frequency quality from 1 to 100
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
        mirx_quality: Option<u8>,

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

    /// Encode an animated GIF, APNG, or WebP as a MIRX FRAMES timeline
    EncodeFrames(EncodeFramesArgs),

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

        #[arg(long, default_value_t = 8)]
        bit_depth: u8,

        #[arg(long)]
        spread: Option<u16>,

        #[arg(long)]
        min_ppem: Option<u16>,

        #[arg(long)]
        max_ppem: Option<u16>,

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

#[derive(clap::Args, Debug)]
pub(crate) struct EncodeFramesArgs {
    /// Animated GIF, APNG, or WebP source
    #[arg(value_hint = clap::ValueHint::FilePath)]
    pub(crate) input: String,

    /// MIRX output file
    #[arg(short = 'O', long, value_hint = clap::ValueHint::FilePath)]
    pub(crate) output: String,

    /// Decoded frame sample format
    #[arg(long, value_enum, default_value = "rgba8888")]
    pub(crate) format: MirxFrameFormat,

    /// Timeline ticks per second
    #[arg(long, default_value_t = 1_000, value_parser = clap::value_parser!(u32).range(1..=1_000_000))]
    pub(crate) timebase: u32,

    /// Duration in ticks used when a source frame has zero duration
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..))]
    pub(crate) default_duration: u32,

    /// Total play count; zero repeats without a limit
    #[arg(long, default_value_t = 0)]
    pub(crate) play_count: u32,

    /// Maximum dependent frames between independently decodable frames
    #[arg(long, default_value_t = 8)]
    pub(crate) max_delta_frames: u16,

    /// Sparse tile geometry as WIDTHxHEIGHT, or none
    #[arg(long, default_value = "32x32")]
    pub(crate) tile: FramesTile,

    /// Required alignment for every encoded DATA input address
    #[arg(long, default_value = "1", value_parser = parse_power_of_two)]
    pub(crate) input_align: u32,

    /// Enable quantized frequency candidates with a quality from 1 to 100
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
    pub(crate) quality: Option<u8>,

    /// Replace an existing output file
    #[arg(short = 'r', long)]
    pub(crate) override_output: bool,
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
pub(crate) enum MirxCodingMode {
    Raw,
    Pixel,
    Rle,
    Lz4,
    FrequencyReversible,
    FrequencyQuantized,
}

impl MirxCodingMode {
    pub(crate) fn into_coding(self, quality: Option<u8>) -> icu_lib::MirxCoding {
        match self {
            Self::Raw => icu_lib::MirxCoding::Raw,
            Self::Pixel => icu_lib::MirxCoding::Pixel,
            Self::Rle => icu_lib::MirxCoding::Rle,
            Self::Lz4 => icu_lib::MirxCoding::Lz4,
            Self::FrequencyReversible => icu_lib::MirxCoding::FrequencyReversible,
            Self::FrequencyQuantized => {
                icu_lib::MirxCoding::FrequencyQuantized(quality.unwrap_or(75))
            }
        }
    }

    const fn is_frequency(self) -> bool {
        matches!(self, Self::FrequencyReversible | Self::FrequencyQuantized)
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MirxFrameFormat {
    Rgb565,
    Rgb565Swapped,
    Rgb888,
    Xrgb8888,
    Rgba8888,
    Bgra8888,
}

impl MirxFrameFormat {
    pub(crate) const fn color_format(self) -> icu_lib::endecoder::ColorFormat {
        match self {
            Self::Rgb565 => icu_lib::endecoder::ColorFormat::RGB565,
            Self::Rgb565Swapped => icu_lib::endecoder::ColorFormat::RGB565Swapped,
            Self::Rgb888 => icu_lib::endecoder::ColorFormat::RGB888,
            Self::Xrgb8888 => icu_lib::endecoder::ColorFormat::XRGB8888,
            Self::Rgba8888 => icu_lib::endecoder::ColorFormat::RGBA8888,
            Self::Bgra8888 => icu_lib::endecoder::ColorFormat::BGRA8888,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FramesTile(Option<(u32, u32)>);

impl FramesTile {
    pub(crate) const fn dimensions(self) -> Option<(u32, u32)> {
        self.0
    }
}

impl std::str::FromStr for FramesTile {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("none") {
            return Ok(Self(None));
        }
        let (width, height) = value
            .split_once('x')
            .or_else(|| value.split_once('X'))
            .ok_or_else(|| "tile must use WIDTHxHEIGHT or none".to_string())?;
        let width = width
            .parse::<u32>()
            .map_err(|_| "tile width must be positive".to_string())?;
        let height = height
            .parse::<u32>()
            .map_err(|_| "tile height must be positive".to_string())?;
        if width == 0 || height == 0 {
            return Err("tile dimensions must be positive".to_string());
        }
        Ok(Self(Some((width, height))))
    }
}

fn parse_power_of_two(value: &str) -> Result<u32, String> {
    let alignment = value
        .parse::<u32>()
        .map_err(|_| "alignment must be a positive power of two".to_string())?;
    if !alignment.is_power_of_two() {
        return Err("alignment must be a positive power of two".to_string());
    }
    Ok(alignment)
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
                output_compressed_method,
                mirx_coding,
                mirx_quality,
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
                if output_format != &ImageFormats::LVGL && output_compressed_method.is_some() {
                    command
                        .error(
                            ErrorKind::InvalidValue,
                            "--output-compressed-method requires LVGL output.",
                        )
                        .exit();
                }
                if output_format != &ImageFormats::MIRX && *mirx_coding != MirxCodingMode::Raw {
                    command
                        .error(
                            ErrorKind::InvalidValue,
                            "--mirx-coding requires MIRX output.",
                        )
                        .exit();
                }
                if output_format == &ImageFormats::MIRX
                    && *mirx_coding == MirxCodingMode::Pixel
                    && !matches!(
                        output_color_format,
                        Some(OutputColorFormats::RGB888 | OutputColorFormats::RGBA8888)
                    )
                {
                    command
                        .error(
                            ErrorKind::InvalidValue,
                            "MIRX Pixel coding requires RGB888 or RGBA8888 output samples.",
                        )
                        .exit();
                }
                if mirx_quality.is_some() && *mirx_coding != MirxCodingMode::FrequencyQuantized {
                    command
                        .error(
                            ErrorKind::InvalidValue,
                            "--mirx-quality requires --mirx-coding frequency-quantized.",
                        )
                        .exit();
                }
                if output_format == &ImageFormats::MIRX
                    && mirx_coding.is_frequency()
                    && !matches!(
                        output_color_format,
                        Some(
                            OutputColorFormats::RGB888
                                | OutputColorFormats::RGBA8888
                                | OutputColorFormats::BGRA8888
                                | OutputColorFormats::I8
                        )
                    )
                {
                    command
                        .error(
                            ErrorKind::InvalidValue,
                            "MIRX frequency coding requires RGB888, RGBA8888, BGRA8888, or I8 output samples.",
                        )
                        .exit();
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
            SubCommands::EncodeFrames(_)
            | SubCommands::BakeFont { .. }
            | SubCommands::MergeFonts { .. } => {}
        }
    } else {
        command.flatten_help(true).print_long_help().unwrap();
        std::process::exit(0);
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mirx_frame_options() {
        let args = Args::try_parse_from([
            "icu",
            "encode-frames",
            "motion.webp",
            "-O",
            "motion.mirx",
            "--format",
            "rgb565-swapped",
            "--tile",
            "16x8",
            "--input-align",
            "64",
            "--quality",
            "75",
        ])
        .unwrap();
        let Some(SubCommands::EncodeFrames(EncodeFramesArgs {
            format,
            tile,
            input_align,
            quality,
            ..
        })) = args.commands
        else {
            panic!("expected encode-frames command");
        };
        assert_eq!(format, MirxFrameFormat::Rgb565Swapped);
        assert_eq!(tile.dimensions(), Some((16, 8)));
        assert_eq!(input_align, 64);
        assert_eq!(quality, Some(75));
    }

    #[test]
    fn rejects_invalid_frame_geometry_and_alignment() {
        assert!(
            Args::try_parse_from([
                "icu",
                "encode-frames",
                "motion.gif",
                "-O",
                "motion.mirx",
                "--tile",
                "0x32",
            ])
            .is_err()
        );
        assert!(
            Args::try_parse_from([
                "icu",
                "encode-frames",
                "motion.gif",
                "-O",
                "motion.mirx",
                "--input-align",
                "3",
            ])
            .is_err()
        );
    }

    #[test]
    fn parses_font_representation_range() {
        let args = Args::try_parse_from([
            "icu",
            "bake-font",
            "font.ttf",
            "--charset",
            "Hello",
            "--size",
            "64",
            "--min-ppem",
            "40",
            "--max-ppem",
            "128",
        ])
        .unwrap();
        let Some(SubCommands::BakeFont {
            size,
            bit_depth,
            min_ppem,
            max_ppem,
            ..
        }) = args.commands
        else {
            panic!("expected bake-font command");
        };
        assert_eq!(size, 64);
        assert_eq!(bit_depth, 8);
        assert_eq!(min_ppem, Some(40));
        assert_eq!(max_ppem, Some(128));
    }
}
