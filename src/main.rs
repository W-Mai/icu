fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use icu_lib::endecoder::common::PNG;
    use icu_lib::midata::MiData;

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
    }
}
