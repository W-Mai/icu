use icu_lib::endecoder::{
    common::{JPEG, PNG},
    EnDecoder,
};
use icu_lib::midata::{IndexedImageData, MiData};
use icu_lib::{EncoderParams, PngColorMode, PngCompression};
use std::io::Cursor;

#[test]
fn png_encodes_direct_color_modes() {
    let image = MiData::RGBA(image::RgbaImage::from_pixel(
        2,
        1,
        image::Rgba([10, 20, 30, 40]),
    ));
    for (mode, expected) in [
        (PngColorMode::Rgb, png::ColorType::Rgb),
        (PngColorMode::Rgba, png::ColorType::Rgba),
    ] {
        let encoded = PNG {}.encode(&image, EncoderParams::default().with_png_color_mode(mode));
        let reader = png::Decoder::new(Cursor::new(encoded)).read_info().unwrap();
        assert_eq!(reader.info().color_type, expected);
        assert_eq!(reader.info().bit_depth, png::BitDepth::Eight);
    }
}

#[test]
fn png_encodes_all_indexed_depths() {
    let image = MiData::RGBA(image::RgbaImage::from_fn(5, 2, |x, y| {
        image::Rgba([(x * 31) as u8, (y * 97) as u8, (x + y) as u8, 255])
    }));
    for (bpp, depth) in [
        (1, png::BitDepth::One),
        (2, png::BitDepth::Two),
        (4, png::BitDepth::Four),
        (8, png::BitDepth::Eight),
    ] {
        let encoded = PNG {}.encode(
            &image,
            EncoderParams::default().with_png_color_mode(PngColorMode::Indexed(bpp)),
        );
        let reader = png::Decoder::new(Cursor::new(encoded)).read_info().unwrap();
        assert_eq!(reader.info().color_type, png::ColorType::Indexed);
        assert_eq!(reader.info().bit_depth, depth);
        assert!(reader.info().palette.as_ref().unwrap().len() / 3 <= 1usize << bpp);
    }
}

#[test]
fn png_preserve_retains_palette_transparency_and_indexes() {
    let indexed = IndexedImageData {
        rgba: image::RgbaImage::from_vec(
            3,
            2,
            vec![
                1, 2, 3, 255, 9, 8, 7, 64, 1, 2, 3, 255, 9, 8, 7, 64, 9, 8, 7, 64, 1, 2, 3, 255,
            ],
        )
        .unwrap(),
        palette: vec![[1, 2, 3, 255], [9, 8, 7, 64]],
        indexes: vec![0, 1, 0, 1, 1, 0],
        bpp: 1,
        width: 3,
        height: 2,
    };
    let encoded = PNG {}.encode(
        &MiData::INDEXED(indexed),
        EncoderParams::default().with_png_color_mode(PngColorMode::Preserve),
    );
    let mut reader = png::Decoder::new(Cursor::new(encoded)).read_info().unwrap();
    assert_eq!(
        reader.info().palette.as_deref(),
        Some(&[1, 2, 3, 9, 8, 7][..])
    );
    assert_eq!(reader.info().trns.as_deref(), Some(&[255, 64][..]));
    assert_eq!(reader.info().bit_depth, png::BitDepth::One);
    let mut output = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut output).unwrap();
    assert_eq!(&output[..info.buffer_size()], &[0b0100_0000, 0b1100_0000]);
}

#[test]
fn png_rejects_malformed_indexed_models() {
    let malformed = IndexedImageData {
        rgba: image::RgbaImage::new(1, 1),
        palette: vec![[0, 0, 0, 255]],
        indexes: vec![1],
        bpp: 3,
        width: 1,
        height: 1,
    };
    assert!(PNG {}
        .encode(
            &MiData::INDEXED(malformed),
            EncoderParams::default().with_png_color_mode(PngColorMode::Preserve)
        )
        .is_empty());
}

#[test]
fn png_dither_changes_indexed_output() {
    let image = MiData::RGBA(image::RgbaImage::from_fn(32, 32, |x, y| {
        let value = ((x + y * 3) * 255 / 124) as u8;
        image::Rgba([value, value.wrapping_add(31), 255 - value, 255])
    }));
    let plain = PNG {}.encode(
        &image,
        EncoderParams::default().with_png_color_mode(PngColorMode::Indexed(2)),
    );
    let dithered = PNG {}.encode(
        &image,
        EncoderParams::default()
            .with_png_color_mode(PngColorMode::Indexed(2))
            .with_dither(Some(10)),
    );
    assert_ne!(plain, dithered);
}

#[test]
fn png_compression_modes_decode_to_identical_pixels() {
    let image = MiData::RGBA(image::RgbaImage::from_fn(8, 8, |x, y| {
        image::Rgba([x as u8, y as u8, (x + y) as u8, 200])
    }));
    let decoded = [
        PngCompression::Fast,
        PngCompression::Balanced,
        PngCompression::Best,
    ]
    .map(|compression| {
        let encoded = PNG {}.encode(
            &image,
            EncoderParams::default().with_png_compression(compression),
        );
        image::load_from_memory(&encoded).unwrap().to_rgba8()
    });
    assert_eq!(decoded[0], decoded[1]);
    assert_eq!(decoded[1], decoded[2]);
}

#[test]
fn jpeg_composites_alpha_and_validates_quality() {
    let transparent = MiData::RGBA(image::RgbaImage::from_pixel(
        16,
        16,
        image::Rgba([255, 0, 0, 0]),
    ));
    for quality in [1, 100] {
        let encoded = JPEG {}.encode(
            &transparent,
            EncoderParams::default()
                .with_jpeg_quality(quality)
                .with_jpeg_background([10, 120, 240]),
        );
        assert!(!encoded.is_empty());
    }
    let encoded = JPEG {}.encode(
        &transparent,
        EncoderParams::default()
            .with_jpeg_quality(100)
            .with_jpeg_background([10, 120, 240]),
    );
    let pixel = *image::load_from_memory(&encoded)
        .unwrap()
        .to_rgb8()
        .get_pixel(0, 0);
    assert!((i16::from(pixel[0]) - 10).abs() <= 8);
    assert!((i16::from(pixel[1]) - 120).abs() <= 8);
    assert!((i16::from(pixel[2]) - 240).abs() <= 8);
    assert!(JPEG {}
        .encode(&transparent, EncoderParams::default().with_jpeg_quality(0))
        .is_empty());
    assert!(JPEG {}
        .encode(
            &transparent,
            EncoderParams::default().with_jpeg_quality(101)
        )
        .is_empty());
}

#[test]
fn jpeg_quality_changes_encoded_output() {
    let image = MiData::RGBA(image::RgbaImage::from_fn(32, 32, |x, y| {
        image::Rgba([((x * y) % 255) as u8, (x * 7) as u8, (y * 11) as u8, 255])
    }));
    let low = JPEG {}.encode(&image, EncoderParams::default().with_jpeg_quality(10));
    let high = JPEG {}.encode(&image, EncoderParams::default().with_jpeg_quality(100));
    assert_ne!(low, high);
    assert_ne!(low.len(), high.len());
}
