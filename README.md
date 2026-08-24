# ICU

Image Converter Ultra (ICU) is a Rust image and font toolkit. It provides a native command-line interface, an `egui` desktop viewer, a WebAssembly viewer, and the reusable [`icu_lib`](icu_lib/) crate.

## Features

- Decode, inspect, preview, and convert common raster image formats.
- Read and write LVGL v8/v9 image data with configurable color format, stride, dithering, and compression.
- Read and write MIRX flat images and inspect MIRX vector, indexed-image, and font chunks.
- Import and export SVG scene data.
- Inspect TTF, OTF, and TTC fonts and individual glyph outlines; WOFF and WOFF2 signatures are recognized for format detection.
- Bake TTF/OTF glyphs into MIRX SDF or grayscale font atlases.
- Merge multiple MIRX font files into one bundle.
- Compare images and glyphs in the desktop and web viewer.
- Generate shell completion scripts for Bash, Zsh, Fish, Elvish, and PowerShell.

## Installation

### Homebrew

```shell
brew install W-Mai/homebrew-cellar/icu_tool
```

Alternatively, add the tap first:

```shell
brew tap W-Mai/homebrew-cellar
brew install icu_tool
```

### Shell installer

```shell
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/W-Mai/icu/releases/latest/download/icu_tool-installer.sh | sh
```

### PowerShell installer

```powershell
powershell -c "irm https://github.com/W-Mai/icu/releases/latest/download/icu_tool-installer.ps1 | iex"
```

### Windows MSI

Download the latest MSI installer from the [releases page](https://github.com/W-Mai/icu/releases/latest).

### Cargo

```shell
cargo install icu_tool
```

The installed executable is named `icu`.

## Command-line interface

Run `icu --help` or `icu <command> --help` for the complete, version-specific option list.

| Command | Purpose |
| --- | --- |
| `icu info <FILE>` | Print detected file metadata as YAML. |
| `icu show [FILES]...` | Open the native viewer. With no files, it opens an empty viewer. |
| `icu convert <INPUTS>... -F <FORMAT>` | Convert files or a directory to another format. |
| `icu bake-font <TTF>` | Bake a TTF/OTF font into a MIRX SDF or grayscale atlas. |
| `icu merge-fonts <INPUTS>... -O <OUTPUT>` | Merge MIRX font files into one multi-font bundle. |

Increase log verbosity with `-v`, `-vv`, or `-vvv` before the subcommand.

### Inspect and preview

ICU auto-detects supported input formats by default.

```shell
icu info res/img_0.png
icu show res/img_0.png res/img_0.bin
```

Use `--input-format common` or `--input-format lvgl-v9` only when automatic detection is not appropriate.

### Convert images

Convert one or more files:

```shell
icu convert res/img_0.png res/img_0.jpeg -F webp
```

Convert a directory recursively while preserving its relative directory structure:

```shell
icu convert res -O output -F jpeg -r
```

Important conversion options include:

- `-F, --output-format`: `png`, `jpeg`, `bmp`, `gif`, `tiff`, `webp`, `ico`, `pbm`, `pgm`, `ppm`, `pam`, `lvgl`, or `mirx`.
- `-O, --output-folder`: write output under a different directory.
- `-r, --override-output`: replace existing output files.
- `-C, --output-color-format`: select an LVGL or MIRX pixel format.
- `-S, --output-stride-align`: align output rows; the default is `1`.
- `--dither`: set indexed-color quantization from `1` to `30`.
- `--output-compressed-method`: select `none`, `rle`, or `lz4` where supported.
- `--lvgl-version`: select LVGL `v8` or `v9`; the default is `v9`.
- `--stdout`: write one converted result to standard output.

LVGL output requires an explicit color format:

```shell
icu convert res/img_0.png -O output -F lvgl -C i8 --lvgl-version v9
```

MIRX flat-image output accepts `rgb565`, `rgb565-swapped`, `rgb888`, `rgba8888`, `bgra8888`, and `xrgb8888` pixel formats:

```shell
icu convert res/img_0.png -O output -F mirx -C rgba8888
```

`--output-category c-array` is reserved by the CLI but is not implemented.

### Bake and merge fonts

Bake an SDF atlas from an inline character set:

```shell
icu bake-font path/to/font.ttf \
  --charset "Hello 世界" \
  --size 32 \
  --bit-depth 4 \
  --format sdf \
  -O output
```

Use `--charset-file <FILE>` to read the character set from a UTF-8 text file. SDF atlases accept bit depths `4` and `8`; grayscale atlases accept `1`, `2`, `4`, and `8`.

Merge multiple baked font files:

```shell
icu merge-fonts output/latin_sdf_32.mirx output/cjk_sdf_32.mirx \
  -O output/fonts.mirx
```

### Shell completion

Add the matching command to the shell startup file.

```shell
# Bash
source <(icu -I bash)

# Zsh
eval "$(icu -I zsh)"

# Fish
icu -I fish | source
```

PowerShell and Elvish are also supported; run `icu -I <shell>` to emit the completion script.

## Viewer

The viewer is implemented with `egui`/`eframe` and runs as a native application or in a browser. It supports drag-and-drop and file selection, raster and animated-image preview, image diffing, MIRX scene inspection, indexed-image inspection, font atlas and glyph-grid views, glyph outline inspection, and font comparison.

Open the native viewer with:

```shell
icu show
```

The WebAssembly build starts the same viewer without the native CLI layer.

## Build from source

The repository pins its Rust toolchain in [`rust-toolchain.toml`](rust-toolchain.toml).

```shell
git clone https://github.com/W-Mai/icu.git
cd icu
cargo build --release
```

The native executable is written to `target/release/icu` on Unix-like systems or `target/release/icu.exe` on Windows.

### WebAssembly

Install the target and [Trunk](https://trunkrs.dev/), then build the web application:

```shell
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk build --release
```

Use `trunk serve` for local development.

## Library

Add the reusable library crate with:

```shell
cargo add icu_lib
```

The library converts external formats through the shared `MiData` model:

```text
input bytes -> EnDecoder::decode -> MiData -> EnDecoder::encode -> output bytes
```

`MiData` represents RGBA images, grayscale images, vector scenes, fonts, and indexed images. Format-specific implementations live under [`icu_lib/src/endecoder`](icu_lib/src/endecoder), while [`icu_lib/src/midata`](icu_lib/src/midata) defines the intermediate model. See [`icu_lib/README.md`](icu_lib/README.md) for a library example.

## Repository layout

```text
src/                    Native CLI and egui/eframe viewer
icu_lib/                Reusable encoders, decoders, and intermediate data
locales/                English and Simplified Chinese UI translations
assets/                 Fonts and web assets
res/                    Sample conversion inputs
.github/workflows/      Release, website, and WebAssembly automation
```

Development conventions and the required quality gate are documented in [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

ICU is available under the [MIT License](LICENSE).
