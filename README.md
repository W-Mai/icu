# ICU

Image Converter Ultra

# Introduction

The Image Converter Ultra (ICU) is a software that converts images from one format to another. It is designed to be a
versatile tool that can handle a wide range of image formats and convert them to other formats. The ICU is designed to
be a standalone application that can be used on any platform that supports the necessary dependencies. The ICU is
written in RUST.

# Features

- Supports a wide range of image formats
- Supports LVGL binary format
- Supports preview a wide range of image formats and LVGL binary format

# How to install

ICU is written in RUST, so you need to have the RUST environment installed on your system.

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After that, you can install ICU by running the following command:

```shell
cargo install icu_tool
```

# How to build yourself

```shell
cargo build --release
```

# How to use

```shell
icu -h
`Show` or `Convert` image files to any other image format including LVGL image formats.

Usage: icu [OPTIONS] <COMMAND>

Commands:
  show     Show an image file
  convert  Convert image files to any other image format including LVGL image formats
  help     Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  verbose mode
  -h, --help        Print help
  -V, --version     Print version
```

# Example

## Show an common image format

```shell
icu show res/img_0.png
```

You will get a window with the image.

<img src="./snapshots/snapshot_1.png" width="800">

## Show an LVGL image format

```shell
icu show res/img_0.bin -f lvgl-v9
```

And you will get a window with the image like before.

