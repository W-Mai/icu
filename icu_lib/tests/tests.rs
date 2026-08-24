#[cfg(test)]
mod tests {
    use icu_lib::endecoder::{common, lvgl, ColorFormat, EnDecoder};
    use icu_lib::midata::MiData;
    use icu_lib::EncoderParams;
    use std::fs;
    use std::mem::size_of;

    const DATA: &[u8] = include_bytes!("../res/img_0.png");

    macro_rules! test_encode_decode {
        ($data:expr, $cf:tt) => {{
            let data = ($data).clone();
            let mid = MiData::decode_from(&common::AutoDetect {}, Vec::from(data));
            let data = mid.encode_into(
                &lvgl::LVGL {},
                EncoderParams {
                    color_format: ColorFormat::$cf,
                    stride_align: 256,
                    lvgl_version: lvgl::LVGLVersion::V9,
                    ..Default::default()
                },
            );
            fs::write("./res/img_0.bin", data).expect("Unable to write file");

            let data = fs::read("./res/img_0.bin").expect("Unable to read file");
            MiData::decode_from(&lvgl::LVGL {}, data);
        }};
    }

    #[test]
    fn lz4_round_trip_and_header() {
        let image = image::RgbaImage::from_fn(8, 4, |x, y| {
            image::Rgba([x as u8 * 17, y as u8 * 31, (x + y) as u8, 255])
        });
        let encoded = MiData::RGBA(image.clone()).encode_into(
            &lvgl::LVGL {},
            EncoderParams {
                color_format: ColorFormat::I8,
                lvgl_version: lvgl::LVGLVersion::V9,
                compress: lvgl::Compress::LZ4,
                ..Default::default()
            },
        );
        assert_eq!(&encoded[..4], &[0x19, 0x0a, 0x08, 0x00]);
        assert_eq!(u32::from_le_bytes(encoded[12..16].try_into().unwrap()), 2);
        assert_eq!(
            u32::from_le_bytes(encoded[20..24].try_into().unwrap()),
            8 * 4 + 256 * 4
        );

        let decoded = MiData::decode_from(&lvgl::LVGL {}, encoded);
        match decoded {
            MiData::INDEXED(indexed) => {
                assert_eq!((indexed.width, indexed.height), (8, 4));
                assert_eq!(indexed.palette.len(), 256);
                assert_eq!(indexed.indexes.len(), 32);
            }
            other => panic!("expected indexed output, got {}", other.variant_name()),
        }
    }

    #[test]
    fn compression_version_matrix_is_preserved() {
        let image = MiData::RGBA(image::RgbaImage::from_pixel(
            4,
            4,
            image::Rgba([10, 20, 30, 255]),
        ));
        let encode = |version, compress| {
            image.clone().encode_into(
                &lvgl::LVGL {},
                EncoderParams {
                    color_format: ColorFormat::ARGB8888,
                    lvgl_version: version,
                    compress,
                    ..Default::default()
                },
            )
        };

        assert!(!encode(lvgl::LVGLVersion::V8, lvgl::Compress::Rle).is_empty());
        assert!(encode(lvgl::LVGLVersion::V8, lvgl::Compress::LZ4).is_empty());
        for method in [lvgl::Compress::Rle, lvgl::Compress::LZ4] {
            let decoded = lvgl::LVGL {}.decode(encode(lvgl::LVGLVersion::V9, method));
            assert_eq!(decoded.variant_name(), "RGBA");
        }
    }

    fn assert_empty_rgba(data: Vec<u8>) {
        match (lvgl::LVGL {}).decode(data) {
            MiData::RGBA(image) => assert_eq!(image.dimensions(), (0, 0)),
            other => panic!("expected empty RGBA, got {}", other.variant_name()),
        }
    }

    #[test]
    fn truncated_uncompressed_payloads_fail_safely() {
        let image = MiData::RGBA(image::RgbaImage::new(4, 4));
        for version in [lvgl::LVGLVersion::V8, lvgl::LVGLVersion::V9] {
            for color_format in [
                ColorFormat::ARGB8888,
                ColorFormat::RGB565,
                ColorFormat::RGB565A8,
                ColorFormat::I8,
            ] {
                let mut encoded = image.clone().encode_into(
                    &lvgl::LVGL {},
                    EncoderParams {
                        color_format,
                        lvgl_version: version,
                        ..Default::default()
                    },
                );
                encoded.pop();
                assert_empty_rgba(encoded);
            }
        }
    }

    #[test]
    fn lz4_rejects_truncated_and_mismatched_payloads() {
        let mut encoded = MiData::RGBA(image::RgbaImage::new(2, 2)).encode_into(
            &lvgl::LVGL {},
            EncoderParams {
                color_format: ColorFormat::ARGB8888,
                lvgl_version: lvgl::LVGLVersion::V9,
                compress: lvgl::Compress::LZ4,
                ..Default::default()
            },
        );
        let truncated = encoded[..encoded.len() - 1].to_vec();
        assert_empty_rgba(truncated);
        assert_empty_rgba(vec![0x19, 0, 0, 0]);

        let mut invalid_method = encoded.clone();
        invalid_method[12] = 0x0f;
        assert_empty_rgba(invalid_method);

        encoded[16..20].copy_from_slice(&0u32.to_le_bytes());
        assert_empty_rgba(encoded);
    }

    #[test]
    fn it_works() {
        use lvgl::ImageHeaderV9;
        assert_eq!(size_of::<ImageHeaderV9>(), 12);

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
        let mid = MiData::decode_from(&lvgl::LVGL {}, data);
        let data = mid.encode_into(&common::PNG {}, Default::default());
        fs::write("img_0_after.png", data).expect("Unable to write file");

        // delete png file and bin file
        fs::remove_file("img_0_after.png").expect("Unable to delete file");
        fs::remove_file("./res/img_0.bin").expect("Unable to delete file");
    }
}
