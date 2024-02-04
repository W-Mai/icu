fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use icu_lib::endecoder::{common, lvgl_v9};
    use icu_lib::midata::MiData;
    use std::fs;
    use std::mem::size_of;

    const DATA: &[u8] = include_bytes!("../res/img_0.png");

    macro_rules! test_encode_decode {
        ($data:expr, $cf:ty) => {{
            let data = ($data).clone();
            let mid = MiData::decode_from::<common::AutoDectect>(Vec::from(data));
            let data = mid.encode_into::<$cf>();
            fs::write("img_0.bin", data).expect("Unable to write file");

            let data = fs::read("img_0.bin").expect("Unable to read file");
            MiData::decode_from::<lvgl_v9::ColorFormatAutoDectect>(data);
        }};
    }

    #[test]
    fn it_works() {
        use lvgl_v9::ImageHeader;
        assert_eq!(size_of::<ImageHeader>(), 12);

        test_encode_decode!(DATA, lvgl_v9::ColorFormatRGB565);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatRGB565A8);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatRGB888);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatARGB8888);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatXRGB8888);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatA1);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatA2);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatA4);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatA8);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatL8);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatI1);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatI2);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatI4);
        test_encode_decode!(DATA, lvgl_v9::ColorFormatI8);

        let data = fs::read("img_0.bin").expect("Unable to read file");
        let mid = MiData::decode_from::<lvgl_v9::ColorFormatAutoDectect>(data);
        let data = mid.encode_into::<common::PNG>();
        fs::write("img_0_after.png", data).expect("Unable to write file");
    }
}
