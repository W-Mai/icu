
#[cfg(test)]
mod tests {
    use icu_lib::endecoder::{common, lvgl_v9};
    use icu_lib::midata::MiData;
    use icu_lib::EncoderParams;
    use std::fs;
    use std::mem::size_of;

    const DATA: &[u8] = include_bytes!("../res/img_0.png");

    macro_rules! test_encode_decode {
        ($data:expr, $cf:tt) => {{
            let data = ($data).clone();
            let mid = MiData::decode_from(&common::AutoDectect {}, Vec::from(data));
            let data = mid.encode_into(
                &lvgl_v9::LVGL {},
                EncoderParams {
                    color_format: lvgl_v9::ColorFormat::$cf,
                    stride_align: 256,
                    dither: false,
                },
            );
            fs::write("./res/img_0.bin", data).expect("Unable to write file");

            let data = fs::read("./res/img_0.bin").expect("Unable to read file");
            MiData::decode_from(&lvgl_v9::LVGL {}, data);
        }};
    }

    #[test]
    fn it_works() {
        use lvgl_v9::ImageHeader;
        assert_eq!(size_of::<ImageHeader>(), 12);

        test_encode_decode!(DATA, RGB565);
        test_encode_decode!(DATA, RGB565A8);
        test_encode_decode!(DATA, RGB888);
        test_encode_decode!(DATA, ARGB8888);
        test_encode_decode!(DATA, XRGB8888);
        test_encode_decode!(DATA, A1);
        test_encode_decode!(DATA, A2);
        test_encode_decode!(DATA, A4);
        test_encode_decode!(DATA, A8);
        test_encode_decode!(DATA, L8);
        test_encode_decode!(DATA, I1);
        test_encode_decode!(DATA, I2);
        test_encode_decode!(DATA, I4);
        test_encode_decode!(DATA, I8);

        let data = fs::read("./res/img_0.bin").expect("Unable to read file");
        let mid = MiData::decode_from(&lvgl_v9::LVGL {}, data);
        let data = mid.encode_into(&common::PNG {}, Default::default());
        fs::write("img_0_after.png", data).expect("Unable to write file");

        // delete png file and bin file
        fs::remove_file("img_0_after.png").expect("Unable to delete file");
        fs::remove_file("./res/img_0.bin").expect("Unable to delete file");
    }
}
