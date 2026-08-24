# ICU-LIB

Image Converter Ultra Library (ICU-LIB) is the reusable format and image-processing library behind the [`icu`](../README.md) command-line tool and viewer.

## Features

- Decode and encode common raster image formats.
- Decode and encode LVGL v8/v9 image data.
- Decode and encode MIRX flat images, scenes, indexed images, and font chunks.
- Parse TTF, OTF, and TTC font outlines and metadata.
- Import and export SVG scene data.
- Provide reusable image post-processing, quantization, dithering, and diff helpers.

## Usage

```shell
cargo add icu_lib
```

The library uses `MiData` as the shared representation between format implementations:

```text
input bytes -> EnDecoder::decode -> MiData -> EnDecoder::encode -> output bytes
```

A minimal conversion example:

```rust
use icu_lib::endecoder::{common, lvgl, ColorFormat};
use icu_lib::midata::MiData;
use icu_lib::EncoderParams;
use std::fs;

fn main() {
    let data = fs::read("input.png").expect("failed to read input");
    let input = common::AutoDetect {};
    let mid = MiData::decode_from(&input, data);

    let output = mid.encode_into(
        &lvgl::LVGL {},
        EncoderParams::new()
            .with_color_format(ColorFormat::ARGB8888)
            .with_stride_align(1)
            .with_lvgl_version(lvgl::LVGLVersion::V9),
    );

    fs::write("output.bin", output).expect("failed to write output");
}
```

`EncoderParams` also supports dithering, LVGL compression, and raw image header options. See the public API and the main repository README for the current CLI-level examples.

## Architecture

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

The main modules are:

- `endecoder/`: format-specific implementations and automatic detection.
- `midata/`: shared RGBA, grayscale, path, font, and indexed-image models.
- `postprocess/`: reusable image transformations.

When adding a format, keep format-specific knowledge in `endecoder`, use an existing `MiData` variant when it can represent the data correctly, register automatic detection when appropriate, and add focused tests. See [`CONTRIBUTING.md`](../CONTRIBUTING.md) for the complete integration checklist.

## License

ICU-LIB is licensed under the MIT license.
