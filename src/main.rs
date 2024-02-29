mod arguments;
mod image_shower;

use crate::arguments::{parse_args, ImageFormatCategory, OutputFileFormatCategory, SubCommands};
use crate::image_shower::show_image;
use icu_lib::endecoder::{common, lvgl_v9};
use icu_lib::midata::MiData;
use icu_lib::EncoderParams;
use std::fs;
use std::path::Path;

fn main() {
    let args = parse_args();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(
        match args.verbose {
            0 => "error",
            1 => "warn",
            2 => "info",
            3 => "debug",
            _ => "trace",
        },
    ))
    .init();

    let commands = args.commands;

    match &commands {
        SubCommands::Show { file, input_format } => {
            // check file exists
            if fs::metadata(file).is_err() {
                println!("File not found: {}", file);
                return;
            }

            let data = fs::read(file).expect("Unable to read file");
            let mid = match input_format {
                ImageFormatCategory::Common => MiData::decode_from(&common::AutoDectect {}, data),
                ImageFormatCategory::LVGL_V9 => MiData::decode_from(&lvgl_v9::LVGL {}, data),
            };

            show_image(mid);
        }
        SubCommands::Convert {
            input_files,
            input_format,
            output_category,
            output_format,
            output_stride_align,
            output_color_format,
            dither,
            lvgl_version: _,
        } => {
            for file_name in input_files {
                let data = fs::read(file_name).expect("Unable to read file");
                let mid = match input_format {
                    ImageFormatCategory::Common => {
                        MiData::decode_from(&common::AutoDectect {}, data)
                    }
                    ImageFormatCategory::LVGL_V9 => MiData::decode_from(&lvgl_v9::LVGL {}, data),
                };

                let ed = output_format.get_endecoder();
                let params = EncoderParams::new()
                    .with_stride_align(*output_stride_align)
                    .with_dither(*dither)
                    .with_color_format((*output_color_format).into());

                let data = mid.encode_into(ed, params);

                match output_category {
                    OutputFileFormatCategory::Common | OutputFileFormatCategory::Bin => {
                        fs::write(
                            Path::new(file_name).with_extension(output_format.get_file_extension()),
                            data,
                        )
                        .expect("Unable to write file");
                    }
                    OutputFileFormatCategory::C_Array => {
                        panic!("C_Array output format is not supported yet");
                    }
                }
            }
        }
    }
}
