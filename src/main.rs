fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use icu_lib::endecoder::common::{JPEG, PNG};
    use icu_lib::endecoder::lvgl_v9;
    use icu_lib::midata::MiData;
    use std::fs;
    use std::mem::size_of;

    #[test]
    fn it_works() {
        let data = include_bytes!("../res/img_0.png");
        let mid_before = MiData::decode_from::<PNG>(Vec::from(*data));
        let data = mid_before.encode_into::<PNG>();
        let mid_after = MiData::decode_from::<PNG>(data);

        let image_buffer_before = match mid_before {
            MiData::RGBA(img) => img,
            MiData::GRAY(_) | MiData::PATH => panic!("Unexpected type"),
        };

        let image_buffer_after = match mid_after {
            MiData::RGBA(img) => img,
            MiData::GRAY(_) | MiData::PATH => panic!("Unexpected type"),
        };

        assert_eq!(image_buffer_before.width(), 285);
        assert_eq!(image_buffer_before, image_buffer_after);

        // use fs write to file
        // use icu_lib::endecoder::common::PPM;
        // fs::write("img_0.pbm", mid_after.encode_into::<PPM>()).expect("Unable to write file");

        use lvgl_v9::ImageHeader;
        assert_eq!(size_of::<ImageHeader>(), 12);

        ///////////////
        let data = include_bytes!("../res/img_0.png");
        let mid_after = MiData::decode_from::<PNG>(Vec::from(*data));
        let data = mid_after.encode_into::<lvgl_v9::ColorFormatRGB565>();

        fs::write("img_0.bin", data).expect("Unable to write file");

        let data = fs::read("img_0.bin").expect("Unable to read file");
        let mid_after = MiData::decode_from::<lvgl_v9::ColorFormatRGB565>(data);
        let data = mid_after.encode_into::<JPEG>();

        fs::write("img_0_after.jpeg", data).expect("Unable to write file");
    }
}
