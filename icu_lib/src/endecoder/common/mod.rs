use crate::{EncoderParams, PngColorMode, PngCompression};
use png;
use std::io::Cursor;

use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::MiData;
use serde_json::json;

pub struct AutoDetect {}

pub struct PNG {}

pub struct JPEG {}

pub struct BMP {}

pub struct GIF {}

pub struct TIFF {}

pub struct WEBP {}

pub struct ICO {}

pub struct PBM {}

pub struct PGM {}

pub struct PPM {}

pub struct PAM {}

pub struct TGA {}

fn decode_raster(
    data: &[u8],
) -> Result<(image::DynamicImage, image::ImageFormat), image::ImageError> {
    let format = image::guess_format(data)?;
    let image = image::load_from_memory_with_format(data, format)?;
    Ok((image, format))
}

fn decode_fixed(data: &[u8], format: image::ImageFormat, label: &str) -> MiData {
    match image::load_from_memory_with_format(data, format) {
        Ok(image) => MiData::RGBA(image.to_rgba8()),
        Err(error) => {
            log::error!("Failed to decode {label}: {error}");
            MiData::RGBA(image::RgbaImage::new(0, 0))
        }
    }
}

fn empty_image_info(data_size: usize) -> ImageInfo {
    ImageInfo {
        width: 0,
        height: 0,
        data_size: data_size as u32,
        format: "unknown".to_string(),
        other_info: serde_json::Value::Null,
    }
}

impl EnDecoder for AutoDetect {
    fn can_decode(&self, data: &[u8]) -> bool {
        image::guess_format(data).is_ok()
    }

    fn encode(&self, _data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        unimplemented!()
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match decode_raster(&data) {
            Ok((image, _)) => MiData::RGBA(image.to_rgba8()),
            Err(error) => {
                log::error!("Failed to decode raster image: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let (img, img_format) = match decode_raster(data) {
            Ok(decoded) => decoded,
            Err(error) => {
                log::error!("Failed to inspect raster image: {error}");
                return empty_image_info(data.len());
            }
        };

        let mut other_info = serde_json::Map::new();

        other_info.insert(
            "Color Type".to_string(),
            json!(format!("{:?}", img.color())),
        );

        // Try to parse EXIF data
        if let Ok(reader) = exif::Reader::new().read_from_container(&mut std::io::Cursor::new(data))
        {
            let mut exif_map = serde_json::Map::new();
            for field in reader.fields() {
                exif_map.insert(
                    field.tag.to_string(),
                    json!(field.display_value().with_unit(&reader).to_string()),
                );
            }
            if !exif_map.is_empty() {
                other_info.insert("Exif".to_string(), serde_json::Value::Object(exif_map));
            }
        }

        ImageInfo {
            width: img.width(),
            height: img.height(),
            data_size: img.as_bytes().len() as u32,
            format: img_format.to_mime_type().to_owned(),
            other_info: serde_json::Value::Object(other_info),
        }
    }
}

fn png_compression(compression: PngCompression) -> png::Compression {
    match compression {
        PngCompression::Fast => png::Compression::Fast,
        PngCompression::Balanced => png::Compression::Balanced,
        PngCompression::Best => png::Compression::High,
    }
}

fn png_bit_depth(bpp: u8) -> Result<png::BitDepth, String> {
    match bpp {
        1 => Ok(png::BitDepth::One),
        2 => Ok(png::BitDepth::Two),
        4 => Ok(png::BitDepth::Four),
        8 => Ok(png::BitDepth::Eight),
        _ => Err(format!("unsupported indexed PNG bit depth: {bpp}")),
    }
}

fn pack_png_indexes(indexes: &[u8], width: u32, height: u32, bpp: u8) -> Result<Vec<u8>, String> {
    let width = usize::try_from(width).map_err(|_| "PNG width does not fit usize")?;
    let height = usize::try_from(height).map_err(|_| "PNG height does not fit usize")?;
    let pixel_count = width
        .checked_mul(height)
        .ok_or("indexed PNG dimensions overflow")?;
    if indexes.len() != pixel_count {
        return Err(format!(
            "indexed PNG has {} indexes, expected {pixel_count}",
            indexes.len()
        ));
    }
    let pixels_per_byte = 8usize / usize::from(bpp);
    let row_bytes = width
        .checked_add(pixels_per_byte - 1)
        .ok_or("indexed PNG row size overflow")?
        / pixels_per_byte;
    let mut packed = vec![
        0;
        row_bytes
            .checked_mul(height)
            .ok_or("indexed PNG size overflow")?
    ];
    for (row, source) in indexes.chunks_exact(width).enumerate() {
        for (column, index) in source.iter().copied().enumerate() {
            let shift = (pixels_per_byte - 1 - column % pixels_per_byte) * usize::from(bpp);
            packed[row * row_bytes + column / pixels_per_byte] |= index << shift;
        }
    }
    Ok(packed)
}

fn write_indexed_png(
    width: u32,
    height: u32,
    palette: &[[u8; 4]],
    indexes: &[u8],
    bpp: u8,
    compression: PngCompression,
) -> Result<Vec<u8>, String> {
    png_bit_depth(bpp)?;
    if width == 0 || height == 0 {
        return Err("indexed PNG dimensions must be non-zero".to_string());
    }
    let max_colors = 1usize << bpp;
    if palette.is_empty() || palette.len() > max_colors {
        return Err(format!(
            "indexed PNG palette has {} colors, expected 1..={max_colors}",
            palette.len()
        ));
    }
    if indexes
        .iter()
        .any(|index| usize::from(*index) >= palette.len())
    {
        return Err("indexed PNG contains an out-of-range palette index".to_string());
    }
    let packed = pack_png_indexes(indexes, width, height, bpp)?;
    let rgb = palette
        .iter()
        .flat_map(|color| [color[0], color[1], color[2]])
        .collect::<Vec<_>>();
    let alpha = palette.iter().map(|color| color[3]).collect::<Vec<_>>();
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, width, height);
        encoder.set_compression(png_compression(compression));
        encoder.set_filter(png::Filter::NoFilter);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png_bit_depth(bpp)?);
        encoder.set_palette(rgb);
        encoder.set_trns(alpha);
        let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
        writer
            .write_image_data(&packed)
            .map_err(|error| error.to_string())?;
    }
    Ok(output)
}

fn quantized_png(
    image: &image::RgbaImage,
    bpp: u8,
    dither: Option<u32>,
    compression: PngCompression,
) -> Result<Vec<u8>, String> {
    png_bit_depth(bpp)?;
    if image.width() == 0 || image.height() == 0 {
        return Err("PNG dimensions must be non-zero".to_string());
    }
    let quantizer = color_quant::NeuQuant::new(
        dither.unwrap_or(30).clamp(1, 30) as i32,
        1usize << bpp,
        image.as_raw(),
    );
    let palette = quantizer
        .color_map_rgba()
        .chunks_exact(4)
        .map(|c| [c[0], c[1], c[2], c[3]])
        .collect::<Vec<_>>();
    let mut indexed_image = image.clone();
    if dither.is_some() {
        image::imageops::dither(&mut indexed_image, &quantizer);
    }
    let indexes = indexed_image
        .pixels()
        .map(|pixel| quantizer.index_of(&pixel.0) as u8)
        .collect::<Vec<_>>();
    write_indexed_png(
        image.width(),
        image.height(),
        &palette,
        &indexes,
        bpp,
        compression,
    )
}

fn encode_png(data: &MiData, params: &EncoderParams) -> Result<Vec<u8>, String> {
    match (data, params.png_color_mode) {
        (MiData::INDEXED(indexed), PngColorMode::Preserve) => write_indexed_png(
            indexed.width,
            indexed.height,
            &indexed.palette,
            &indexed.indexes,
            indexed.bpp,
            params.png_compression,
        ),
        (_, PngColorMode::Preserve) => {
            Err("PNG preserve mode requires indexed image data".to_string())
        }
        (MiData::RGBA(image), PngColorMode::Indexed(bpp)) => {
            quantized_png(image, bpp, params.dither, params.png_compression)
        }
        (MiData::INDEXED(indexed), PngColorMode::Indexed(bpp)) => {
            quantized_png(&indexed.rgba, bpp, params.dither, params.png_compression)
        }
        (MiData::RGBA(image), mode @ (PngColorMode::Rgb | PngColorMode::Rgba)) => {
            encode_direct_png(image, mode, params.png_compression)
        }
        (MiData::INDEXED(indexed), mode @ (PngColorMode::Rgb | PngColorMode::Rgba)) => {
            encode_direct_png(&indexed.rgba, mode, params.png_compression)
        }
        _ => Err("PNG encoding requires RGBA or indexed image data".to_string()),
    }
}

fn encode_direct_png(
    image: &image::RgbaImage,
    mode: PngColorMode,
    compression: PngCompression,
) -> Result<Vec<u8>, String> {
    if image.width() == 0 || image.height() == 0 {
        return Err("PNG dimensions must be non-zero".to_string());
    }
    let (color_type, bytes) = match mode {
        PngColorMode::Rgb => (
            png::ColorType::Rgb,
            image
                .pixels()
                .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
                .collect::<Vec<_>>(),
        ),
        PngColorMode::Rgba => (png::ColorType::Rgba, image.as_raw().clone()),
        _ => return Err("direct PNG mode must be RGB or RGBA".to_string()),
    };
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, image.width(), image.height());
        encoder.set_compression(png_compression(compression));
        encoder.set_color(color_type);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
        writer
            .write_image_data(&bytes)
            .map_err(|error| error.to_string())?;
    }
    Ok(output)
}

impl EnDecoder for PNG {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Png
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        match encode_png(data, &encoder_params) {
            Ok(output) => output,
            Err(error) => {
                log::error!("Failed to encode PNG: {error}");
                Vec::new()
            }
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match image::load_from_memory_with_format(&data, image::ImageFormat::Png) {
            Ok(image) => MiData::RGBA(image.to_rgba8()),
            Err(error) => {
                log::error!("Failed to decode PNG: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let mut info = AutoDetect {}.info(data);

        // Add PNG specific info
        if let Ok(decoder) = png::Decoder::new(Cursor::new(data)).read_info() {
            let png_info = decoder.info();
            if let serde_json::Value::Object(ref mut map) = info.other_info {
                map.insert(
                    "PNG Color Type".to_string(),
                    json!(format!("{:?}", png_info.color_type)),
                );
                map.insert(
                    "Bit Depth".to_string(),
                    json!(format!("{:?}", png_info.bit_depth)),
                );
                if png_info.trns.is_some() {
                    map.insert("Transparent".to_string(), json!("Yes"));
                }
                map.insert("Interlaced".to_string(), json!(png_info.interlaced));
            }
        }

        info
    }
}

impl EnDecoder for JPEG {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Jpeg
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        let image = match data {
            MiData::RGBA(image) => image,
            MiData::INDEXED(indexed) => &indexed.rgba,
            _ => return Vec::new(),
        };
        if !(1..=100).contains(&encoder_params.jpeg_quality) {
            log::error!("Failed to encode JPEG: quality must be between 1 and 100");
            return Vec::new();
        }
        let background = encoder_params.jpeg_background;
        let rgb = image
            .pixels()
            .flat_map(|pixel| {
                let alpha = u32::from(pixel[3]);
                std::array::from_fn::<_, 3, _>(|channel| {
                    let source = u32::from(pixel[channel]);
                    let background = u32::from(background[channel]);
                    ((source * alpha + background * (255 - alpha) + 127) / 255) as u8
                })
            })
            .collect::<Vec<_>>();
        let mut output = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut output,
            encoder_params.jpeg_quality,
        );
        if let Err(error) = image::ImageEncoder::write_image(
            encoder,
            &rgb,
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgb8,
        ) {
            log::error!("Failed to encode JPEG: {error}");
            return Vec::new();
        }
        output
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match image::load_from_memory_with_format(&data, image::ImageFormat::Jpeg) {
            Ok(image) => MiData::RGBA(image.to_rgba8()),
            Err(error) => {
                log::error!("Failed to decode JPEG: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for BMP {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Bmp
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Bmp).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::Bmp, "BMP")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for GIF {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Gif
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Gif).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match image::load_from_memory_with_format(&data, image::ImageFormat::Gif) {
            Ok(image) => MiData::RGBA(image.to_rgba8()),
            Err(error) => {
                log::error!("Failed to decode GIF: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for TIFF {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Tiff
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Tiff).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::Tiff, "TIFF")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for WEBP {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::WebP
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::WebP).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::WebP, "WEBP")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for ICO {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Ico
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Ico).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::Ico, "ICO")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for PBM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::GRAY(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match image::load_from_memory_with_format(&data, image::ImageFormat::Pnm) {
            Ok(image) => MiData::GRAY(image.to_luma_alpha8()),
            Err(error) => {
                log::error!("Failed to decode PBM: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for PGM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::GRAY(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        match image::load_from_memory_with_format(&data, image::ImageFormat::Pnm) {
            Ok(image) => MiData::GRAY(image.to_luma_alpha8()),
            Err(error) => {
                log::error!("Failed to decode PGM: {error}");
                MiData::RGBA(image::RgbaImage::new(0, 0))
            }
        }
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for PPM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::Pnm, "PNM")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

impl EnDecoder for PAM {
    fn can_decode(&self, data: &[u8]) -> bool {
        if let Ok(format) = image::guess_format(data) {
            format == image::ImageFormat::Pnm
        } else {
            false
        }
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Pnm).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        decode_fixed(&data, image::ImageFormat::Pnm, "PNM")
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::{AutoDetect, BMP, JPEG, PNG};
    use crate::endecoder::EnDecoder;
    use crate::midata::MiData;
    use crate::EncoderParams;
    use image::GenericImageView;

    #[test]
    fn jpeg_encode_accepts_rgba_input() {
        let image = image::RgbaImage::from_vec(2, 1, vec![255, 0, 0, 0, 0, 128, 255, 255]).unwrap();

        let encoded = JPEG {}.encode(&MiData::RGBA(image), EncoderParams::default());

        assert!(!encoded.is_empty());
        assert_eq!(
            image::guess_format(&encoded).unwrap(),
            image::ImageFormat::Jpeg
        );
        assert_eq!(
            image::load_from_memory(&encoded).unwrap().dimensions(),
            (2, 1)
        );
    }

    #[test]
    fn format_probes_reject_non_raster_data_without_panicking() {
        let data = vec![0x19, 0x0a, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(!PNG {}.can_decode(&data));
        assert!(!JPEG {}.can_decode(&data));
        assert!(!BMP {}.can_decode(&data));
    }

    #[test]
    fn autodetect_rejects_truncated_raster_without_panicking() {
        let data = b"\x89PNG\r\n\x1a\n".to_vec();
        assert!(AutoDetect {}.can_decode(&data));
        assert!(matches!(
            AutoDetect {}.decode(data.clone()),
            MiData::RGBA(image) if image.width() == 0 && image.height() == 0
        ));
        let info = AutoDetect {}.info(&data);
        assert_eq!((info.width, info.height), (0, 0));
        assert_eq!(info.data_size, data.len() as u32);
    }
}

impl EnDecoder for TGA {
    fn can_decode(&self, _data: &[u8]) -> bool {
        false
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageFormat::Tga).unwrap();
                buf.into_inner()
            }
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        MiData::RGBA(
            image::load_from_memory_with_format(&data, image::ImageFormat::Tga)
                .unwrap()
                .to_rgba8(),
        )
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        AutoDetect {}.info(data)
    }
}
