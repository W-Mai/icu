mod arguments;
mod image_shower;

use crate::arguments::{parse_args, ImageFormatCategory, ImageOutputFormatCategory, SubCommands};
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
                ImageFormatCategory::LVGL_V9 => {
                    MiData::decode_from(&lvgl_v9::ColorFormatAutoDectect {}, data)
                }
            };

            show_image(mid);
        }
        SubCommands::Convert {
            input_files,
            input_format,
            output_category,
            output_format,
            lvgl_version: _lvgl_version,
        } => {
            for file_name in input_files {
                let data = fs::read(file_name).expect("Unable to read file");
                let mid = match input_format {
                    ImageFormatCategory::Common => {
                        MiData::decode_from(&common::AutoDectect {}, data)
                    }
                    ImageFormatCategory::LVGL_V9 => {
                        MiData::decode_from(&lvgl_v9::ColorFormatAutoDectect {}, data)
                    }
                };

                let ed = output_format.get_endecoder();
                let data = mid.encode_into(
                    ed,
                    EncoderParams {
                        stride_align: 256,
                        dither: false,
                    },
                );

                match output_category {
                    ImageOutputFormatCategory::Common | ImageOutputFormatCategory::Bin => {
                        fs::write(
                            Path::new(file_name).with_extension(output_format.get_file_extension()),
                            data,
                        )
                        .expect("Unable to write file");
                    }
                    ImageOutputFormatCategory::C_Array => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use icu_lib::endecoder::{common, lvgl_v9};
    use icu_lib::midata::MiData;
    use icu_lib::EncoderParams;
    use std::fs;
    use std::mem::size_of;

    const DATA: &[u8] = include_bytes!("../res/img_0.png");

    macro_rules! test_encode_decode {
        ($data:expr, $cf:expr) => {{
            let data = ($data).clone();
            let mid = MiData::decode_from(&common::AutoDectect {}, Vec::from(data));
            let data = mid.encode_into(
                $cf,
                EncoderParams {
                    stride_align: 256,
                    dither: false,
                },
            );
            fs::write("./res/img_0.bin", data).expect("Unable to write file");

            let data = fs::read("./res/img_0.bin").expect("Unable to read file");
            MiData::decode_from(&lvgl_v9::ColorFormatAutoDectect {}, data);
        }};
    }

    #[test]
    fn it_works() {
        use lvgl_v9::ImageHeader;
        assert_eq!(size_of::<ImageHeader>(), 12);

        test_encode_decode!(DATA, &lvgl_v9::ColorFormatRGB565 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatRGB565A8 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatRGB888 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatARGB8888 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatXRGB8888 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatA1 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatA2 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatA4 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatA8 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatL8 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatI1 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatI2 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatI4 {});
        test_encode_decode!(DATA, &lvgl_v9::ColorFormatI8 {});

        let data = fs::read("./res/img_0.bin").expect("Unable to read file");
        let mid = MiData::decode_from(&lvgl_v9::ColorFormatAutoDectect {}, data);
        let data = mid.encode_into(&common::PNG {}, Default::default());
        fs::write("img_0_after.png", data).expect("Unable to write file");
    }
}
