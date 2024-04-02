use std::fs;

use criterion::{criterion_group, criterion_main, Criterion};
use icu_lib::endecoder::{common, lvgl};
use icu_lib::midata::MiData;

macro_rules! test_encode_decode {
    ($data:expr, $cf:ty) => {{
        let data = ($data).clone();
        let mid = MiData::decode_from::<common::AutoDetect>(Vec::from(data));
        let data = mid.encode_into::<$cf>();
        fs::write("img_0.bin", data).expect("Unable to write file");

        let data = fs::read("img_0.bin").expect("Unable to read file");
        MiData::decode_from::<lvgl::LVGL>(data);
    }};
}

// ColorFormatRGB565
// ColorFormatRGB565A8
// ColorFormatRGB888
// ColorFormatARGB8888
// ColorFormatXRGB8888
// ColorFormatA1
// ColorFormatA2
// ColorFormatA4
// ColorFormatA8
// ColorFormatL8
// ColorFormatI1
// ColorFormatI2
// ColorFormatI4
// ColorFormatI8

fn bench_img_0_encode_decode_rgb565(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_rgb565", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatRGB565);
        })
    });
}

fn bench_img_0_encode_decode_rgb565a8(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_rgb565a8", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatRGB565A8);
        })
    });
}

fn bench_img_0_encode_decode_rgb888(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_rgb888", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatRGB888);
        })
    });
}

fn bench_img_0_encode_decode_argb8888(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_argb8888", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatARGB8888);
        })
    });
}

fn bench_img_0_encode_decode_xrgb8888(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_xrgb8888", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatXRGB8888);
        })
    });
}

fn bench_img_0_encode_decode_a1(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_a1", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatA1);
        })
    });
}

fn bench_img_0_encode_decode_a2(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_a2", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatA2);
        })
    });
}

fn bench_img_0_encode_decode_a4(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_a4", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatA4);
        })
    });
}

fn bench_img_0_encode_decode_a8(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_a8", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatA8);
        })
    });
}

fn bench_img_0_encode_decode_l8(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_l8", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatL8);
        })
    });
}

fn bench_img_0_encode_decode_i1(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_i1", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatI1);
        })
    });
}

fn bench_img_0_encode_decode_i2(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_i2", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatI2);
        })
    });
}

fn bench_img_0_encode_decode_i4(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_i4", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatI4);
        })
    });
}

fn bench_img_0_encode_decode_i8(c: &mut Criterion) {
    let data = include_bytes!("../res/img_0.png");
    c.bench_function("img_0_encode_decode_i8", |b| {
        b.iter(|| {
            test_encode_decode!(data, lvgl::ColorFormatI8);
        })
    });
}

criterion_group!(
    benches,
    bench_img_0_encode_decode_rgb565,
    bench_img_0_encode_decode_rgb565a8,
    bench_img_0_encode_decode_rgb888,
    bench_img_0_encode_decode_argb8888,
    bench_img_0_encode_decode_xrgb8888,
    bench_img_0_encode_decode_a1,
    bench_img_0_encode_decode_a2,
    bench_img_0_encode_decode_a4,
    bench_img_0_encode_decode_a8,
    bench_img_0_encode_decode_l8,
    bench_img_0_encode_decode_i1,
    bench_img_0_encode_decode_i2,
    bench_img_0_encode_decode_i4,
    bench_img_0_encode_decode_i8
);

criterion_main!(benches);
