fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use icu_lib::midata::MiData;
    use icu_lib::endecoder::common::PNG;

    #[test]
    fn it_works() {
        let data = include_bytes!("../res/img_0.png");
        let mid = MiData::decode_from(&PNG {}, Vec::from(*data));
        let data = mid.encode_into(&PNG {});
        let mid = MiData::decode_from(&PNG {}, data);
        assert_eq!(mid.encode_into(&PNG {}).len(), 172953);
    }
}
