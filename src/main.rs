mod arguments;
mod image_shower;

use crate::arguments::{
    parse_args, ImageFormatCategory, ImageFormats, OutputFileFormatCategory, SubCommands,
};
use crate::image_shower::show_image;
use icu_lib::endecoder::{common, find_endecoder, lvgl, EnDecoder};
use icu_lib::midata::MiData;
use icu_lib::{endecoder, EncoderParams};
use std::fs;
use std::path::{Path, PathBuf};

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

    let commands = args.commands.ok_or("No subcommand provided")?;

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

            let input_folder = input_files
                .first()
                .filter(|&path| input_files.len() == 1 && Path::new(path).is_dir())
                .map(|path| Path::new(path).to_path_buf());

            let file_or_folder = if input_folder.is_some() {
                "folder"
            } else {
                "file"
            };
            log::trace!("{} to be converted: {:#?}", file_or_folder, input_files);
            log::info!("Start converting {}", file_or_folder);
            log::info!("");

            deal_input_file_paths(input_files, &input_folder, |file_path| {
                let file_path = Path::new(&file_path);

                // calculate converting time
                let start_time = std::time::Instant::now();

                let output_file_path =
                    deal_path_without_extension(&file_path, &input_folder, output_folder.clone())
                        .unwrap_or_default()
                        .with_extension(output_format.get_file_extension());

                let output_file_exists = output_file_path.exists();
                let should_convert = !output_file_exists || *override_output;

                if should_convert {
                    if output_file_exists {
                        log::warn!(
                            "Override output file <{}> for converting <{}>",
                            output_file_path.to_string_lossy(),
                            file_path.to_string_lossy()
                        );
                    }
                } else {
                    log::error!(
                        "Can't convert <{}> to <{}>, output file already exists",
                        file_path.to_string_lossy(),
                        output_file_path.to_string_lossy()
                    );
                }

                if should_convert {
                    if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
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
                    })() {
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
            });

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

fn deal_input_file_paths<F: FnMut(&String)>(
    input_files: &[String],
    input_folder: &Option<PathBuf>,
    mut deal_func: F,
) {
    if let Some(folder) = input_folder {
        let mut folder_list = vec![folder.to_path_buf()];

        while !folder_list.is_empty() {
            if let Some(folder) = folder_list.pop() {
                folder.read_dir().unwrap().for_each(|entry| {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    if path.is_file() {
                        let path_string = path.to_string_lossy().into();
                        deal_func(&path_string);
                    } else if path.is_dir() {
                        folder_list.push(path);
                    }
                });
            }
        }
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
            .for_each(|file_name| deal_func(&file_name));
    }
}

fn deal_path_without_extension(
    file_path: &Path,
    folder: &Option<PathBuf>,
    output_folder: Option<String>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let full_path = file_path.canonicalize()?;
    if !file_path.exists() {
        return Err(format!("File not found: {}", file_path.to_string_lossy()).into());
    }

    let file_folder = match folder {
        None => Path::new(&file_path),
        Some(folder) => full_path.strip_prefix(folder.canonicalize()?)?,
    }
    .parent()
    .ok_or("Unable to get parent folder of input file")?;

    let file_name = file_path.file_name().unwrap_or_default();
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

fn decode_with(
    data: Vec<u8>,
    input_format: ImageFormatCategory,
) -> Result<MiData, Box<dyn std::error::Error>> {
    match input_format {
        ImageFormatCategory::Auto => {
            let ed = find_endecoder(&data);
            Ok(ed.ok_or("No supported endecoder found")?.decode(data))
        }
        ImageFormatCategory::Common => Ok(MiData::decode_from(&common::AutoDetect {}, data)),
        ImageFormatCategory::LVGL_V9 => Ok(MiData::decode_from(&lvgl::LVGL {}, data)),
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
        ImageFormatCategory::Common => Ok(common::AutoDetect {}.info(&data)),
        ImageFormatCategory::LVGL_V9 => Ok(lvgl::LVGL {}.info(&data)),
    }
}
