mod arguments;
mod image_shower;

use crate::arguments::{
    parse_args, ImageFormatCategory, ImageFormats, OutputFileFormatCategory, SubCommands,
};
use crate::image_shower::show_image;
use icu_lib::endecoder::{common, find_endecoder, lvgl_v9, EnDecoder};
use icu_lib::midata::MiData;
use icu_lib::{endecoder, EncoderParams};
use std::fs;
use std::path::{Path, PathBuf};

fn decode_with(
    data: Vec<u8>,
    input_format: ImageFormatCategory,
) -> Result<MiData, Box<dyn std::error::Error>> {
    match input_format {
        ImageFormatCategory::Auto => {
            let ed = find_endecoder(&data);
            Ok(ed.ok_or("No supported endecoder found")?.decode(data))
        }
        ImageFormatCategory::Common => Ok(MiData::decode_from(&common::AutoDectect {}, data)),
        ImageFormatCategory::LVGL_V9 => Ok(MiData::decode_from(&lvgl_v9::LVGL {}, data)),
    }
}

fn get_info_with(
    data: Vec<u8>,
    input_format: ImageFormatCategory,
) -> Result<endecoder::ImageInfo, Box<dyn std::error::Error>> {
    match input_format {
        ImageFormatCategory::Auto => {
            let ed = find_endecoder(&data);
            Ok(ed.ok_or("No endecoder found")?.info(&data))
        }
        ImageFormatCategory::Common => Ok(common::AutoDectect {}.info(&data)),
        ImageFormatCategory::LVGL_V9 => Ok(lvgl_v9::LVGL {}.info(&data)),
    }
}

fn main() {
    let res = process();

    if let Err(e) = res {
        log::error!("{}", e);
        std::process::exit(1);
    }
}

fn process() -> Result<(), Box<dyn std::error::Error>> {
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
        SubCommands::Info { file, input_format } => {
            let data = fs::read(file)?;
            let info = get_info_with(data, *input_format)?;

            println!("{:#?}", info);
        }
        SubCommands::Show { file, input_format } => {
            let data = fs::read(file)?;
            let mid = decode_with(data, *input_format);

            show_image(mid?);
        }
        SubCommands::Convert {
            input_files,
            input_format,
            output_folder,
            override_output,
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
            let input_folder = if is_folder_input {
                input_files
                    .first()
                    .map(|path| Path::new(path).canonicalize().unwrap_or_default())
            } else {
                None
            };

            log::trace!("files to be converted: {:#?}", input_files);
            log::info!(
                "Start converting {}",
                if is_folder_input { "folder" } else { "file" }
            );
            log::info!("");

            let input_files_vec = deal_input_file_paths(input_files, &input_folder)?;

            for file_path in input_files_vec {
                let file_path = Path::new(&file_path).canonicalize()?;

                // calculate converting time
                let start_time = std::time::Instant::now();

                let output_file_path =
                    deal_path_without_extension(&file_path, &input_folder, output_folder.clone())?
                        .with_extension(output_format.get_file_extension());

                let output_file_exists = output_file_path.exists();
                if output_file_exists && !*override_output {
                    log::error!(
                        "Can't convert <{}> to <{}>, output file already exists",
                        file_path.to_string_lossy(),
                        output_file_path.to_string_lossy()
                    )
                } else {
                    if output_file_exists {
                        log::warn!(
                            "Override output file <{}> for converting <{}>",
                            output_file_path.to_string_lossy(),
                            file_path.to_string_lossy()
                        );
                    }

                    let deal_one_file = || -> Result<(), Box<dyn std::error::Error>> {
                        let params = EncoderParams::new()
                            .with_stride_align(*output_stride_align)
                            .with_dither(*dither)
                            .with_color_format(
                                (*output_color_format).map(|f| f.into()).unwrap_or_default(),
                            )
                            .with_lvgl_version((*lvgl_version).into());

                        let data = fs::read(&file_path)?;
                        let ed = output_format.get_endecoder();
                        let mid = decode_with(data, *input_format)?;
                        let data = mid.encode_into(ed, params);

                        match output_category {
                            OutputFileFormatCategory::Common | OutputFileFormatCategory::Bin => {
                                fs::write(&output_file_path, data)?;
                            }
                            OutputFileFormatCategory::C_Array => {
                                return Err("C_Array output format is not supported yet".into());
                            }
                        }
                        Ok(())
                    };

                    if let Err(e) = deal_one_file() {
                        log::error!(
                            "Failed to convert <{}> to <{}>: {}",
                            file_path.to_string_lossy(),
                            output_file_path.to_string_lossy(),
                            e
                        );
                    }
                }

                let end_time = std::time::Instant::now();
                let duration = end_time - start_time;
                user_duration += duration.as_secs_f64();
                let output_format_str = if output_format == &ImageFormats::LVGL {
                    format!(
                        "LVGL.{:?}({:?})",
                        lvgl_version,
                        (*output_color_format).unwrap() // safe to unwrap because it's required
                    )
                } else {
                    format!("{:?}", output_format)
                };
                log::info!(
                    "took {:.6}s for converting <{}> to <{}> with format <{}>",
                    duration.as_secs_f64(),
                    file_path.to_string_lossy(),
                    output_file_path.to_string_lossy(),
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

    Ok(())
}

fn deal_input_file_paths(
    input_files: &[String],
    input_folder: &Option<PathBuf>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(if let Some(folder) = input_folder {
        let mut folder_list = vec![folder.to_path_buf()];
        let mut files = Vec::new();

        while !folder_list.is_empty() {
            if let Some(folder) = folder_list.pop() {
                folder.read_dir()?.for_each(|entry| {
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
    })
}

fn deal_path_without_extension(
    file_path: &PathBuf,
    folder: &Option<PathBuf>,
    output_folder: Option<String>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if !file_path.exists() {
        return Err(format!("File not found: {}", file_path.to_string_lossy()).into());
    }

    let file_folder = match folder {
        None => Path::new(&file_path),
        Some(folder) => file_path.strip_prefix(folder)?,
    }
    .parent()
    .ok_or("Unable to get parent folder of input file")?;

    let file_name = Path::new(&file_path).file_name().unwrap_or_default();
    let output_file_name = Path::new(file_name).with_extension("");
    let mut output_file_path = file_folder.join(&output_file_name);

    if let Some(output_folder) = output_folder {
        let output_folder = match folder {
            None => Path::new(&output_folder).to_path_buf(),
            Some(_) => Path::new(&output_folder).join(file_folder),
        };

        if !output_folder.exists() {
            fs::create_dir_all(&output_folder)?;
        }

        output_file_path = output_folder.join(&output_file_name);
    }

    Ok(output_file_path)
}
