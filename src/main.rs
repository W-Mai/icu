mod arguments;
mod image_shower;

use crate::arguments::{
    parse_args, ImageFormatCategory, ImageFormats, OutputFileFormatCategory, SubCommands,
};
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
            output_folder,
            output_category,
            output_format,
            output_stride_align,
            output_color_format,
            dither,
            lvgl_version,
        } => {
            // calculate converting time
            let total_start_time = std::time::Instant::now();
            let mut user_duration = 0.0;
            let mut converted_files = 0;
            let is_folder_input = input_files.len() == 1 && Path::new(&input_files[0]).is_dir();

            log::trace!("files to be converted: {:#?}", input_files);
            log::info!(
                "Start converting {}",
                if is_folder_input { "folder" } else { "file" }
            );
            log::info!("");

            let input_files_vec = if is_folder_input {
                let input_folder = Path::new(&input_files[0]).to_path_buf();
                let mut folder_list = vec![input_folder];
                let mut files = Vec::new();

                while !folder_list.is_empty() {
                    if let Some(folder) = folder_list.pop() {
                        folder.read_dir().unwrap().for_each(|entry| {
                            let entry = entry.unwrap();
                            let path = entry.path();
                            if path.is_file() {
                                log::trace!("converting file: {}", path.to_str().unwrap());
                                files.push(path.to_string_lossy().into())
                            } else if path.is_dir() {
                                folder_list.push(path);
                            }
                        });
                    }
                }
                files
            } else {
                input_files
                    .iter()
                    .filter_map(|file_name| {
                        let metadata = fs::metadata(file_name);

                        match metadata {
                            Ok(metadata) => {
                                if metadata.is_dir() {
                                    log::trace!("{} is a directory, skip it", file_name);
                                    return None;
                                }
                                Some(file_name.clone())
                            }
                            Err(_) => {
                                log::error!("File not found: {}", file_name);
                                None
                            }
                        }
                    })
                    .collect::<Vec<String>>()
            };

            for file_path in input_files_vec {
                // calculate converting time
                let start_time = std::time::Instant::now();

                let data = fs::read(&file_path).expect("Unable to read file");
                let mid = decode_with(data, *input_format);

                let ed = output_format.get_endecoder();
                let params = EncoderParams::new()
                    .with_stride_align(*output_stride_align)
                    .with_dither(*dither)
                    .with_color_format(
                        (*output_color_format).map(|f| f.into()).unwrap_or_default(),
                    );

                let data = mid.encode_into(ed, params);

                let file_folder = Path::new(&file_path).parent().unwrap();
                let file_name = Path::new(&file_path).file_name().unwrap_or_default();

                let output_file_name =
                    Path::new(file_name).with_extension(output_format.get_file_extension());

                let mut output_file_path = file_folder.join(&output_file_name);

                if let Some(output_folder) = output_folder {
                    let output_folder = Path::new(output_folder);
                    if !output_folder.exists() {
                        fs::create_dir_all(output_folder).expect("Unable to create output folder");
                    }

                    output_file_path = output_folder.join(&output_file_name);
                }

                match output_category {
                    OutputFileFormatCategory::Common | OutputFileFormatCategory::Bin => {
                        fs::write(&output_file_path, data).expect("Unable to write file");
                    }
                    OutputFileFormatCategory::C_Array => {
                        panic!("C_Array output format is not supported yet");
                    }
                }

                let end_time = std::time::Instant::now();
                let duration = end_time - start_time;
                user_duration += duration.as_secs_f64();
                let output_format_str = if output_format == &ImageFormats::LVGL {
                    format!(
                        "LVGL.{:?}({:?})",
                        lvgl_version,
                        (*output_color_format).unwrap()
                    )
                } else {
                    format!("{:?}", output_format)
                };
                log::info!(
                    "took {:.6}s for converting <{}> to <{}> with format <{}>",
                    duration.as_secs_f64(),
                    &file_path,
                    output_file_path.to_str().unwrap_or_default(),
                    output_format_str
                );

                converted_files += 1;
            }

            let end_time = std::time::Instant::now();
            let duration = end_time - total_start_time;
            log::info!("");
            log::info!("Total converting time:");
            log::info!(
                "\tConsuming  : {:.6}s for {} files",
                duration.as_secs_f64(),
                converted_files
            );
            log::info!("\tUser   time: {:.6}s", user_duration);
            log::info!(
                "\tSystem time: {:.6}s",
                duration.as_secs_f64() - user_duration
            );
        }
    }
}
