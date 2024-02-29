# ICU-LIB

Image Converter Ultra Library (ICU-LIB)

# Introduction

The Image Converter Ultra (ICU) is a software that converts images from one format to another. It is designed to be a
versatile tool that can handle a wide range of image formats and convert them to other formats. The ICU is designed to
be a standalone application that can be used on any platform that supports the necessary dependencies. The ICU is
written in RUST.

# Features

- Supports a wide range of image formats
- Supports LVGL binary format

# How to use

```shell
cargo add icu_lib
```

```rust
use icu_lib::endecoder::{common, lvgl_v9};
use icu_lib::midata::MiData;
use icu_lib::EncoderParams;
use std::fs;

fn main() {
    const DATA: &[u8] = include_bytes!("../res/img_0.png");

    // Decode the image data and automatically detect the format
    let mid = MiData::decode_from(&common::AutoDectect {}, Vec::from(DATA));

    // Encode the image data to the LVGL binary format with ARGB8888 color format
    let data = mid.encode_into(
        &lvgl_v9::LVGL {},
        EncoderParams {
            color_format: lvgl_v9::ColorFormat::ARGB8888,
            stride_align: 256,
            dither: false,
        });

    fs::write("img_0.bin", data).expect("Unable to write file");
}
```

# Architecture

```text
       ╔═══════════════╗                       
       ║               ║                       
       ║               ║                       
┌ ─ ─ ─ ─ ─ ─ ┐        ║                       
  ┌ ─ ─ ─ ─ ┐          ║                       
│  EnDecoder  │        ▼                       
  └ ─ ─ ─ ─ ┘   ┌ ─ ─ ─ ─ ─ ─ ┐                
│┌───────────┐│   ┌ ─ ─ ─ ─ ┐                  
 │    PNG    │  │   MidData   │                
│└───────────┘│   └ ─ ─ ─ ─ ┘                  
 ┌───────────┐  │┌───────────┐│                
││   JPEG    ││  │   ARGB    │                 
 └───────────┘  │└───────────┘│ ╔-------------╗
│┌───────────┐│  ┌───────────┐  ║   ICU_LIB   ║
 │    SVG    │  ││   PATH    ││ ╚-------------╝
│└───────────┘│  └───────────┘                 
 ┌───────────┐  │┌── ─── ─── ┐│                
││ LVGL BIN  ││     CUSTOM   │                 
 └───────────┘  │└── ─── ─── ┘│                
│┌── ─── ─── ┐│  ─ ─ ─ ─ ─ ─ ─                 
    CUSTOM   │         ║                       
│└── ─── ─── ┘│        ║                       
 ─ ─ ─ ─ ─ ─ ─         ║                       
       ▲               ║                       
       ║               ║                       
       ╚═══════════════╝                       
```
