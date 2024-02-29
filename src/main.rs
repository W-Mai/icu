mod arguments;
mod image_shower;

use crate::arguments::{parse_args, ImageFormatCategory, OutputFileFormatCategory, SubCommands};
use crate::image_shower::show_image;
use icu_lib::endecoder::{common, lvgl_v9};
use icu_lib::midata::{decode_from, MiData};
use icu_lib::EncoderParams;
use std::fs;
use std::path::Path;

fn decode_with(data: Vec<u8>, input_format: ImageFormatCategory) -> MiData {
    match input_format {
        ImageFormatCategory::Auto => decode_from(data),
        ImageFormatCategory::Common => MiData::decode_from(&common::AutoDectect {}, data),
        ImageFormatCategory::LVGL_V9 => MiData::decode_from(&lvgl_v9::LVGL {}, data),
    }
}

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
            let mid = decode_with(data, *input_format);

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
            // calculate converting time
            let total_start_time = std::time::Instant::now();
            let mut user_duration = 0.0;
            log::info!("Start converting files");
            log::info!("");

            for file_name in input_files {
                // calculate converting time
                let start_time = std::time::Instant::now();

                let data = fs::read(file_name).expect("Unable to read file");
                let mid = decode_with(data, *input_format);

                let ed = output_format.get_endecoder();
                let mut params = EncoderParams::new()
                    .with_stride_align(*output_stride_align)
                    .with_dither(*dither);

                if let Some(output_color_format) = output_color_format {
                    params = params.with_color_format((*output_color_format).into());
                } else {
                    params = params.with_color_format(lvgl_v9::ColorFormat::UNKNOWN);
                }

                let data = mid.encode_into(ed, params);

                let output_file_name =
                    Path::new(file_name).with_extension(output_format.get_file_extension());

                match output_category {
                    OutputFileFormatCategory::Common | OutputFileFormatCategory::Bin => {
                        fs::write(output_file_name.clone(), data).expect("Unable to write file");
                    }
                    OutputFileFormatCategory::C_Array => {
                        panic!("C_Array output format is not supported yet");
                    }
                }

                let end_time = std::time::Instant::now();
                let duration = end_time - start_time;
                user_duration += duration.as_secs_f64();
                log::info!(
                    "took {:.6}s for converting [{}] to [{}] with format [{:?}] ",
                    duration.as_secs_f64(),
                    file_name,
                    output_file_name.to_str().unwrap_or_default(),
                    output_format
                );
            }

            let end_time = std::time::Instant::now();
            let duration = end_time - total_start_time;
            log::info!("");
            log::info!("Total converting time:");
            log::info!(
                "\tConsuming  : {:.6}s for {} files",
                duration.as_secs_f64(),
                input_files.len()
            );
            log::info!("\tUser   time: {:.6}s", user_duration);
            log::info!(
                "\tSystem time: {:.6}s",
                duration.as_secs_f64() - user_duration
            );
        }
    }
}
