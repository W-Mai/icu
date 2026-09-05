use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::{FontData, IndexedImageData, MiData, SceneData};
use crate::{EncoderParams, MirxCoding};
use image::RgbaImage;
use mirx::{ColorFormat as MirxColorFormat, FlatImageInput};
use serde_json::json;

pub mod font_bake;
pub mod font_contour;
pub mod font_render;
pub mod frames;
pub mod scene_render;

pub struct Mirx;

fn bytes_per_pixel(format: MirxColorFormat) -> Option<usize> {
    let bits = format.bits_per_pixel();
    bits.is_multiple_of(8).then(|| usize::from(bits / 8))
}

fn aligned_stride(format: MirxColorFormat, width: u32, alignment: u32) -> Option<u32> {
    let minimum = format.minimum_stride(width)?;
    let alignment = alignment.max(1);
    minimum
        .checked_add(alignment - 1)?
        .checked_div(alignment)?
        .checked_mul(alignment)
}

pub(super) fn rgba_to_mirx_pixels(
    img: &RgbaImage,
    cf: MirxColorFormat,
    stride: u32,
) -> Option<Vec<u8>> {
    let (w, h) = img.dimensions();
    let raw = img.as_raw();
    let bpp = bytes_per_pixel(cf)?;
    let row_bytes = usize::try_from(cf.minimum_stride(w)?).ok()?;
    let stride = stride as usize;
    let mut out = vec![0u8; stride * h as usize];
    for y in 0..h as usize {
        let dst = &mut out[y * stride..y * stride + row_bytes];
        for x in 0..w as usize {
            let si = (y * w as usize + x) * 4;
            let di = x * bpp;
            match cf {
                MirxColorFormat::RGBA8888 | MirxColorFormat::XRGB8888 => {
                    dst[di..di + 4].copy_from_slice(&raw[si..si + 4]);
                    if matches!(cf, MirxColorFormat::XRGB8888) {
                        dst[di + 3] = 0xFF;
                    }
                }
                MirxColorFormat::BGRA8888 => {
                    dst[di] = raw[si + 2];
                    dst[di + 1] = raw[si + 1];
                    dst[di + 2] = raw[si];
                    dst[di + 3] = raw[si + 3];
                }
                MirxColorFormat::RGB888 => {
                    dst[di] = raw[si];
                    dst[di + 1] = raw[si + 1];
                    dst[di + 2] = raw[si + 2];
                }
                MirxColorFormat::RGB565 => {
                    let r = (raw[si] >> 3) as u16;
                    let g = (raw[si + 1] >> 2) as u16;
                    let b = (raw[si + 2] >> 3) as u16;
                    let px = (r << 11) | (g << 5) | b;
                    dst[di] = (px & 0xFF) as u8;
                    dst[di + 1] = (px >> 8) as u8;
                }
                MirxColorFormat::RGB565Swapped => {
                    let r = (raw[si] >> 3) as u16;
                    let g = (raw[si + 1] >> 2) as u16;
                    let b = (raw[si + 2] >> 3) as u16;
                    let px = (r << 11) | (g << 5) | b;
                    dst[di] = (px >> 8) as u8;
                    dst[di + 1] = (px & 0xFF) as u8;
                }
                _ => return None,
            }
        }
    }
    Some(out)
}

fn indexed_to_mirx_pixels(
    image: &IndexedImageData,
    format: MirxColorFormat,
    stride: u32,
) -> Option<Vec<u8>> {
    let entries = usize::try_from(format.palette_entries()?).ok()?;
    let bits = format.bits_per_pixel();
    if bits != image.bpp
        || image.indexes.len() != usize::try_from(image.width.checked_mul(image.height)?).ok()?
        || image.palette.len() > entries
        || image
            .indexes
            .iter()
            .any(|index| usize::from(*index) >= image.palette.len())
    {
        return None;
    }
    let row_bytes = usize::try_from(format.minimum_stride(image.width)?).ok()?;
    let stride = usize::try_from(stride).ok()?;
    let height = usize::try_from(image.height).ok()?;
    let width = usize::try_from(image.width).ok()?;
    let mut output = vec![0; stride.checked_mul(height)?];
    for y in 0..height {
        let row = &mut output[y * stride..y * stride + row_bytes];
        for x in 0..width {
            let index = image.indexes[y * width + x];
            let bit = x.checked_mul(usize::from(bits))?;
            let shift = 8usize.checked_sub(usize::from(bits) + bit % 8)?;
            row[bit / 8] |= index << shift;
        }
    }
    Some(output)
}

fn indexed_palette(image: &IndexedImageData, format: MirxColorFormat) -> Option<Vec<u8>> {
    let entries = usize::try_from(format.palette_entries()?).ok()?;
    if image.palette.len() > entries {
        return None;
    }
    let mut rgba = vec![0; entries.checked_mul(4)?];
    for (source, target) in image.palette.iter().zip(rgba.chunks_exact_mut(4)) {
        target.copy_from_slice(source);
    }
    Some(rgba)
}

fn mirx_pixels_to_rgba(
    main: &[u8],
    width: u32,
    height: u32,
    stride: u32,
    cf: MirxColorFormat,
) -> Option<RgbaImage> {
    let w = width as usize;
    let h = height as usize;
    let stride = stride as usize;
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let si = y * stride + x * bytes_per_pixel(cf)?;
            let di = (y * w + x) * 4;
            match cf {
                MirxColorFormat::RGBA8888 | MirxColorFormat::XRGB8888 => {
                    out[di..di + 4].copy_from_slice(&main[si..si + 4]);
                }
                MirxColorFormat::BGRA8888 => {
                    out[di] = main[si + 2];
                    out[di + 1] = main[si + 1];
                    out[di + 2] = main[si];
                    out[di + 3] = main[si + 3];
                }
                MirxColorFormat::RGB888 => {
                    out[di] = main[si];
                    out[di + 1] = main[si + 1];
                    out[di + 2] = main[si + 2];
                    out[di + 3] = 255;
                }
                MirxColorFormat::RGB565 => {
                    let px = u16::from_le_bytes([main[si], main[si + 1]]);
                    out[di] = (((px >> 11) & 0x1F) as u8) << 3;
                    out[di + 1] = (((px >> 5) & 0x3F) as u8) << 2;
                    out[di + 2] = ((px & 0x1F) as u8) << 3;
                    out[di + 3] = 255;
                }
                MirxColorFormat::RGB565Swapped => {
                    let px = u16::from_be_bytes([main[si], main[si + 1]]);
                    out[di] = (((px >> 11) & 0x1F) as u8) << 3;
                    out[di + 1] = (((px >> 5) & 0x3F) as u8) << 2;
                    out[di + 2] = ((px & 0x1F) as u8) << 3;
                    out[di + 3] = 255;
                }
                _ => return None,
            }
        }
    }
    RgbaImage::from_vec(width, height, out)
}

fn surface_to_rgba(surface: mirx::image::SurfaceView<'_>) -> Option<RgbaImage> {
    let descriptor = surface.surface();
    let format = descriptor.sample_layout().color_format()?;
    let plane = surface.plane(0)?;
    if let Some(entries) = format.palette_entries() {
        let palette = surface.color_table()?;
        if palette.len() != usize::try_from(entries).ok()? {
            return None;
        }
        let width = usize::try_from(descriptor.width()).ok()?;
        let height = usize::try_from(descriptor.height()).ok()?;
        let bits = usize::from(format.bits_per_pixel());
        let mut rgba = vec![0; width.checked_mul(height)?.checked_mul(4)?];
        for y in 0..height {
            let row = plane.row(y as u32).ok().flatten()?;
            for x in 0..width {
                let bit = x.checked_mul(bits)?;
                let shift = 8usize.checked_sub(bits + bit % 8)?;
                let index = usize::from((row[bit / 8] >> shift) & (0xff >> (8 - bits)));
                let color = palette.get(index)?;
                rgba[(y * width + x) * 4..(y * width + x + 1) * 4]
                    .copy_from_slice(&[color.r, color.g, color.b, color.a]);
            }
        }
        return RgbaImage::from_vec(descriptor.width(), descriptor.height(), rgba);
    }
    mirx_pixels_to_rgba(
        plane.bytes(),
        descriptor.width(),
        descriptor.height(),
        plane.memory().stride(),
        format,
    )
}

struct EncodedStream {
    id: mirx::CodingId,
    revision: u16,
    params: Vec<u8>,
    bytes: Vec<u8>,
}

impl EncodedStream {
    fn new(record: mirx::media::CodingRecord<'_>, bytes: Vec<u8>) -> Self {
        Self {
            id: record.id(),
            revision: record.revision(),
            params: record.params().to_vec(),
            bytes,
        }
    }

    fn record(&self) -> mirx::media::CodingRecord<'_> {
        mirx::media::CodingRecord::new(self.id, self.revision, &self.params)
    }
}

fn encode_stream(
    samples: &[u8],
    layout: mirx::image::SampleLayout,
    width: u32,
    height: u32,
    coding: MirxCoding,
) -> Option<EncodedStream> {
    match coding {
        MirxCoding::Raw => None,
        MirxCoding::Pixel => {
            let codec = mirx::coding::Pixel::new(layout).ok()?;
            let pixels = samples
                .len()
                .checked_div(bytes_per_pixel(layout.color_format()?)?)?;
            let mut output = vec![0; codec.encoded_bound(pixels).ok()?];
            let len = codec.encode_into(samples, &mut output).ok()?;
            output.truncate(len);
            Some(EncodedStream::new(codec.record(), output))
        }
        MirxCoding::Rle => {
            let element = bytes_per_pixel(layout.color_format()?).unwrap_or(1) as u8;
            let codec = mirx::coding::Rle::new().with_element_size(element).ok()?;
            let mut output = vec![0; codec.encoded_bound(samples.len()).ok()?];
            let len = codec.encode_into(samples, &mut output).ok()?;
            output.truncate(len);
            Some(EncodedStream::new(codec.record(), output))
        }
        MirxCoding::Lz4 => {
            let codec = mirx::coding::Lz4::new();
            let mut table = vec![0; mirx::coding::Lz4::TABLE_LEN];
            let mut encoder = codec.encoder(&mut table).ok()?;
            let mut output = vec![0; codec.encoded_bound(samples.len()).ok()?];
            let len = encoder.encode_into(samples, &mut output).ok()?;
            output.truncate(len);
            Some(EncodedStream::new(codec.record(), output))
        }
        MirxCoding::FrequencyReversible | MirxCoding::FrequencyQuantized(_) => {
            let geometry =
                mirx::coding::FrequencyGeometry::for_plane(layout, 0, width, height).ok()?;
            let codec = match coding {
                MirxCoding::FrequencyReversible => mirx::coding::Frequency::reversible(),
                MirxCoding::FrequencyQuantized(quality) => {
                    mirx::coding::Frequency::quantized(quality).ok()?
                }
                _ => unreachable!("frequency coding selected above"),
            };
            let mut output = vec![0; codec.encoded_bound(geometry).ok()?];
            let len = codec.encode_into(geometry, samples, &mut output).ok()?;
            output.truncate(len);
            let mut params = [0];
            Some(EncodedStream::new(codec.record_into(&mut params), output))
        }
    }
}

fn encode_coded_image(
    img: &RgbaImage,
    format: MirxColorFormat,
    coding: MirxCoding,
) -> Option<Vec<u8>> {
    let (width, height) = img.dimensions();
    let layout = mirx::image::SampleLayout::from_color_format(format);
    let color = if layout.is_alpha() {
        mirx::image::ColorDescription::NONE
    } else {
        mirx::image::ColorDescription::SRGB
    };
    let surface = mirx::image::SurfaceDescriptor::new(width, height, layout, color).ok()?;
    let stride = format.minimum_stride(width)?;
    let samples = rgba_to_mirx_pixels(img, format, stride)?;
    let stream = encode_stream(&samples, layout, width, height, coding)?;
    let asset = mirx::image::EncodedImageAsset::new(surface, stream.record(), &stream.bytes);
    asset.preflight(&mirx::PayloadLimits::HOST).ok()?;
    let mut document = mirx::Document::new_with_limits(mirx::PayloadLimits::HOST);
    let id = document.push_encoded_image(&asset).ok()?;
    document.set_primary(id).ok()?;
    document.encode(&mirx::EncodeOptions::new()).ok()
}

fn encode_indexed_image(
    image: &IndexedImageData,
    format: MirxColorFormat,
    coding: MirxCoding,
    stride_alignment: u32,
) -> Option<Vec<u8>> {
    let layout = mirx::image::SampleLayout::from_color_format(format);
    let surface = mirx::image::SurfaceDescriptor::new(
        image.width,
        image.height,
        layout,
        mirx::image::ColorDescription::SRGB,
    )
    .ok()?;
    let palette = indexed_palette(image, format)?;
    if coding == MirxCoding::Raw {
        let stride = aligned_stride(format, image.width, stride_alignment)?;
        let samples = indexed_to_mirx_pixels(image, format, stride)?;
        return Some(mirx::encode_flat(&FlatImageInput {
            width: image.width,
            height: image.height,
            stride,
            format,
            main: &samples,
            extra: Some(&palette),
        }));
    }
    let stride = format.minimum_stride(image.width)?;
    let samples = indexed_to_mirx_pixels(image, format, stride)?;
    let stream = encode_stream(&samples, layout, image.width, image.height, coding)?;
    let asset = mirx::image::EncodedImageAsset::new(surface, stream.record(), &stream.bytes)
        .with_color_table(&palette);
    asset.preflight(&mirx::PayloadLimits::HOST).ok()?;
    let mut document = mirx::Document::new_with_limits(mirx::PayloadLimits::HOST);
    let id = document.push_encoded_image(&asset).ok()?;
    document.set_primary(id).ok()?;
    document.encode(&mirx::EncodeOptions::new()).ok()
}

fn decode_coded_image(image: mirx::image::EncodedImageView<'_>) -> Option<RgbaImage> {
    let mut slots = vec![None; image.group_count()];
    let mut budget = mirx::image::CoverageBudget::new(mirx::PayloadLimits::HOST.max_raster_work());
    let groups = image.groups_into(&mut slots, &mut budget).ok()?;
    let plan = groups
        .decode_plan(
            mirx::image::SurfaceRequirements::new(),
            &mirx::PayloadLimits::HOST,
        )
        .ok()?;
    let mut output = vec![0; plan.memory_plan().byte_len() as usize];
    let mut workspace = vec![0; plan.workspace_requirements().byte_len()];
    let surface = plan.decode_into(&mut output, &mut workspace).ok()?;
    surface_to_rgba(surface)
}

fn decode_image(image: mirx::image::ImageRef<'_>) -> Option<RgbaImage> {
    match image {
        mirx::image::ImageRef::Raw(surface) => surface_to_rgba(surface),
        mirx::image::ImageRef::Encoded(encoded) => decode_coded_image(encoded),
    }
}

fn coding_label(id: mirx::CodingId) -> String {
    if id == mirx::CodingId::PIXEL {
        "PIXEL".to_owned()
    } else if id == mirx::CodingId::RLE {
        "RLE".to_owned()
    } else if id == mirx::CodingId::LZ4 {
        "LZ4".to_owned()
    } else if id == mirx::CodingId::FREQUENCY_REVERSIBLE {
        "FREQUENCY_REVERSIBLE".to_owned()
    } else if id == mirx::CodingId::FREQUENCY_QUANTIZED {
        "FREQUENCY_QUANTIZED".to_owned()
    } else if id == mirx::CodingId::RAW {
        "RAW".to_owned()
    } else {
        format!("{}", id.raw())
    }
}

impl EnDecoder for Mirx {
    fn can_decode(&self, data: &[u8]) -> bool {
        data.len() >= 4 && &data[..4] == b"MIRX"
    }

    fn encode(&self, data: &MiData, params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::RGBA(img) => {
                let mirx_cf = match params.color_format.to_mirx() {
                    Some(cf) => cf,
                    None => return Vec::new(),
                };
                match params.mirx_coding {
                    MirxCoding::Raw => {
                        let (w, h) = img.dimensions();
                        let stride = match aligned_stride(mirx_cf, w, params.stride_align) {
                            Some(stride) => stride,
                            None => return Vec::new(),
                        };
                        let main = match rgba_to_mirx_pixels(img, mirx_cf, stride) {
                            Some(v) => v,
                            None => return Vec::new(),
                        };
                        let input = FlatImageInput {
                            width: w,
                            height: h,
                            stride,
                            format: mirx_cf,
                            main: &main,
                            extra: None,
                        };
                        mirx::encode_flat(&input)
                    }
                    coding => encode_coded_image(img, mirx_cf, coding).unwrap_or_default(),
                }
            }
            MiData::PATH(scene_data) => {
                let payload = match scene_data.scene.encode() {
                    Ok(p) => p,
                    Err(_) => return Vec::new(),
                };
                mirx::encode_chunk_generic(
                    mirx::chunk_type::VECTOR,
                    mirx::ChunkEntry::FLAG_CRITICAL,
                    &payload,
                )
            }
            MiData::FONT(font_data) => match font_data {
                FontData::Mirx(f) => {
                    let Ok(payload) = f.encode() else {
                        return Vec::new();
                    };
                    mirx::encode_chunk_generic(
                        mirx::chunk_type::FONT,
                        mirx::ChunkEntry::FLAG_CRITICAL,
                        &payload,
                    )
                }
                FontData::MirxBundle(fonts) => {
                    let chunks: Option<Vec<(u16, u16, Vec<u8>)>> = fonts
                        .iter()
                        .map(|f| {
                            Some((
                                mirx::chunk_type::FONT,
                                mirx::ChunkEntry::FLAG_CRITICAL,
                                f.encode().ok()?,
                            ))
                        })
                        .collect();
                    let Some(chunks) = chunks else {
                        return Vec::new();
                    };
                    let refs: Vec<(u16, u16, &[u8])> = chunks
                        .iter()
                        .map(|(t, f, p)| (*t, *f, p.as_slice()))
                        .collect();
                    mirx::encode_chunks(&refs)
                }
                FontData::FreeType(_) => Vec::new(),
            },
            MiData::GRAY(_) => Vec::new(),
            MiData::INDEXED(image) => {
                let Some(format) = params.color_format.to_mirx() else {
                    return Vec::new();
                };
                encode_indexed_image(image, format, params.mirx_coding, params.stride_align)
                    .unwrap_or_default()
            }
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        let options = mirx::ReadOptions::new().with_payload_limits(mirx::PayloadLimits::HOST);
        let reader = match mirx::Reader::open_with(&data, &options) {
            Ok(reader) => reader,
            Err(_) => return MiData::RGBA(RgbaImage::new(0, 0)),
        };
        if let Some(flat) = reader.flat_image() {
            return MiData::RGBA(
                flat.surface()
                    .ok()
                    .and_then(surface_to_rgba)
                    .unwrap_or_else(|| RgbaImage::new(0, 0)),
            );
        }
        if let Ok(Some(primary)) = reader.primary() {
            if primary.chunk_type() == mirx::ChunkType::IMAGE {
                return MiData::RGBA(
                    primary
                        .image()
                        .ok()
                        .flatten()
                        .and_then(decode_image)
                        .unwrap_or_else(|| RgbaImage::new(0, 0)),
                );
            }
            if primary.chunk_type() == mirx::ChunkType::VECTOR {
                if let Ok(scene) = mirx::Scene::decode(primary.payload()) {
                    return MiData::PATH(SceneData { scene });
                }
            }
        }
        if let Some(chunk) = reader
            .chunks()
            .find(|chunk| chunk.chunk_type() == mirx::ChunkType::VECTOR)
        {
            if let Ok(scene) = mirx::Scene::decode(chunk.payload()) {
                return MiData::PATH(SceneData { scene });
            }
        }
        let fonts: Vec<mirx::Font> = reader
            .chunks()
            .filter(|chunk| chunk.chunk_type() == mirx::ChunkType::FONT)
            .filter_map(|chunk| mirx::Font::decode(chunk.payload()).ok())
            .collect();
        if fonts.len() == 1 {
            return MiData::FONT(FontData::Mirx(fonts.into_iter().next().unwrap()));
        }
        if !fonts.is_empty() {
            return MiData::FONT(FontData::MirxBundle(fonts));
        }
        if let Some(image) = reader
            .chunks()
            .filter_map(|chunk| chunk.image().ok().flatten())
            .find_map(decode_image)
        {
            return MiData::RGBA(image);
        }
        MiData::RGBA(RgbaImage::new(0, 0))
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let options = mirx::ReadOptions::new().with_payload_limits(mirx::PayloadLimits::HOST);
        match mirx::Reader::open_with(data, &options) {
            Ok(reader) if reader.flat_image().is_some() => {
                let image = reader.flat_image().unwrap();
                ImageInfo {
                    width: image.width(),
                    height: image.height(),
                    data_size: image.main().len() as u32,
                    format: format!("{:?}", image.format()),
                    other_info: json!({"layout": "flat", "coding": "raw"}),
                }
            }
            Ok(reader) => {
                let mut chunks_info = serde_json::Map::new();
                for chunk in reader.chunks() {
                    match chunk.chunk_type() {
                        mirx::ChunkType::VECTOR => {
                            if let Ok(scene) = mirx::Scene::decode(chunk.payload()) {
                                chunks_info
                                    .insert("vector".into(), json!({"op_count": scene.ops.len()}));
                            }
                        }
                        mirx::ChunkType::FONT => {
                            if let Ok(font) = mirx::Font::decode(chunk.payload()) {
                                let representations = (0..font.representation_count())
                                    .filter_map(|index| font.representation(index))
                                    .map(|representation| {
                                        let metadata = representation.metadata();
                                        json!({
                                            "kind": format!("{:?}", metadata.kind()),
                                            "design_ppem": metadata.design_ppem(),
                                            "min_ppem": metadata.min_ppem(),
                                            "max_ppem": metadata.max_ppem(),
                                            "surface": representation.surface_index(),
                                        })
                                    })
                                    .collect::<Vec<_>>();
                                chunks_info.insert(
                                    "font".into(),
                                    json!({
                                        "glyph_count": font.codepoints().len(),
                                        "representation_count": font.representation_count(),
                                        "surface_count": font.surface_count(),
                                        "representations": representations,
                                    }),
                                );
                            }
                        }
                        mirx::ChunkType::IMAGE => {
                            if let Ok(Some(image)) = chunk.image() {
                                let surface = image.surface();
                                let coding = image.encoded().map(|encoded| {
                                    encoded
                                        .codings()
                                        .iter()
                                        .map(|record| coding_label(record.id()))
                                        .collect::<Vec<_>>()
                                });
                                chunks_info.insert(
                                    "image".into(),
                                    json!({
                                        "width": surface.width(),
                                        "height": surface.height(),
                                        "sample_layout": format!("{:?}", surface.sample_layout()),
                                        "coding": coding.unwrap_or_else(|| vec!["RAW".to_owned()]),
                                    }),
                                );
                            }
                        }
                        _ => {}
                    }
                }
                let hints = reader.primary_hints();
                ImageInfo {
                    width: hints.width(),
                    height: hints.height(),
                    data_size: data.len() as u32,
                    format: format!("{:?}", hints.sample_layout()),
                    other_info: json!({"layout": "chunk", "chunks": chunks_info}),
                }
            }
            Err(_) => ImageInfo {
                width: 0,
                height: 0,
                data_size: 0,
                format: "unknown".to_string(),
                other_info: json!({}),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endecoder::ColorFormat;
    use image::Rgba;

    fn sample_rgba(w: u32, h: u32) -> RgbaImage {
        let mut img = RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                img.put_pixel(x, y, Rgba([x as u8, y as u8, 128, 255]));
            }
        }
        img
    }

    fn sample_indexed() -> IndexedImageData {
        let palette = vec![
            [10, 20, 30, 255],
            [200, 40, 50, 128],
            [60, 180, 70, 255],
            [80, 90, 220, 64],
        ];
        let indexes = vec![0, 1, 2, 3, 0, 3, 2, 1, 0, 3];
        let rgba = indexes
            .iter()
            .flat_map(|index| palette[usize::from(*index)])
            .collect();
        IndexedImageData {
            rgba: RgbaImage::from_vec(5, 2, rgba).unwrap(),
            palette,
            indexes,
            bpp: 2,
            width: 5,
            height: 2,
        }
    }

    fn roundtrip(cf: ColorFormat) {
        let img = sample_rgba(4, 4);
        let ed = Mirx;
        let params = EncoderParams::default().with_color_format(cf);
        let bytes = ed.encode(&MiData::RGBA(img.clone()), params);
        assert!(ed.can_decode(&bytes), "can_decode for {:?}", cf);
        match ed.decode(bytes) {
            MiData::RGBA(back) => {
                assert_eq!(back.dimensions(), img.dimensions(), "dims for {:?}", cf);
            }
            _ => panic!("expected RGBA for {:?}", cf),
        }
    }

    fn roundtrip_coding(coding: MirxCoding, expected: mirx::CodingId) {
        let img = sample_rgba(12, 5);
        let ed = Mirx;
        let params = EncoderParams::default()
            .with_color_format(ColorFormat::RGBA8888)
            .with_mirx_coding(coding);
        let bytes = ed.encode(&MiData::RGBA(img.clone()), params);
        let options = mirx::ReadOptions::new().with_payload_limits(mirx::PayloadLimits::HOST);
        let reader = mirx::Reader::open_with(&bytes, &options).unwrap();
        let image = reader
            .primary()
            .unwrap()
            .unwrap()
            .image()
            .unwrap()
            .unwrap()
            .encoded()
            .unwrap();
        assert_eq!(image.codings().get(0).unwrap().id(), expected);
        match ed.decode(bytes) {
            MiData::RGBA(decoded) => assert_eq!(decoded, img),
            other => panic!("expected RGBA, got {}", other.variant_name()),
        }
    }

    #[test]
    fn roundtrip_rgb565() {
        roundtrip(ColorFormat::RGB565);
    }

    #[test]
    fn roundtrip_rgb565_swapped() {
        roundtrip(ColorFormat::RGB565Swapped);
    }

    #[test]
    fn roundtrip_rgb888() {
        roundtrip(ColorFormat::RGB888);
    }

    #[test]
    fn roundtrip_rgba8888() {
        roundtrip(ColorFormat::RGBA8888);
    }

    #[test]
    fn roundtrip_bgra8888() {
        roundtrip(ColorFormat::BGRA8888);
    }

    #[test]
    fn roundtrip_xrgb8888() {
        roundtrip(ColorFormat::XRGB8888);
    }

    #[test]
    fn native_pixel_roundtrip_uses_mirx_pixel_coding() {
        roundtrip_coding(MirxCoding::Pixel, mirx::CodingId::PIXEL);
    }

    #[test]
    fn rle_roundtrip_uses_mirx_rle_coding() {
        roundtrip_coding(MirxCoding::Rle, mirx::CodingId::RLE);
    }

    #[test]
    fn lz4_roundtrip_uses_mirx_lz4_coding() {
        roundtrip_coding(MirxCoding::Lz4, mirx::CodingId::LZ4);
    }

    #[test]
    fn reversible_frequency_roundtrip_uses_mirx_frequency_coding() {
        roundtrip_coding(
            MirxCoding::FrequencyReversible,
            mirx::CodingId::FREQUENCY_REVERSIBLE,
        );
    }

    #[test]
    fn quantized_frequency_preserves_alpha_and_reports_its_profile() {
        let img = RgbaImage::from_fn(13, 9, |x, y| {
            Rgba([
                (x * 31 + y * 7) as u8,
                (x * 3 + y * 47) as u8,
                (x * 19 + y * 23) as u8,
                (x * 17 + y * 29) as u8,
            ])
        });
        let bytes = Mirx.encode(
            &MiData::RGBA(img.clone()),
            EncoderParams::default()
                .with_color_format(ColorFormat::RGBA8888)
                .with_mirx_coding(MirxCoding::FrequencyQuantized(55)),
        );
        let info = Mirx.info(&bytes);
        assert_eq!(
            info.other_info["chunks"]["image"]["coding"][0],
            "FREQUENCY_QUANTIZED"
        );
        let MiData::RGBA(decoded) = Mirx.decode(bytes) else {
            panic!("expected RGBA image");
        };
        assert_eq!(decoded.dimensions(), img.dimensions());
        let mut changed = false;
        for (before, after) in img.pixels().zip(decoded.pixels()) {
            assert_eq!(before[3], after[3]);
            changed |= before.0[..3] != after.0[..3];
        }
        assert!(changed);
    }

    #[test]
    fn native_pixel_rejects_unsupported_sample_layout() {
        let bytes = Mirx.encode(
            &MiData::RGBA(sample_rgba(2, 2)),
            EncoderParams::default()
                .with_color_format(ColorFormat::RGB565)
                .with_mirx_coding(MirxCoding::Pixel),
        );
        assert!(bytes.is_empty());
    }

    #[test]
    fn indexed_raw_rle_and_lz4_roundtrip_with_color_table() {
        for coding in [MirxCoding::Raw, MirxCoding::Rle, MirxCoding::Lz4] {
            let indexed = sample_indexed();
            let expected = indexed.rgba.clone();
            let bytes = Mirx.encode(
                &MiData::INDEXED(indexed),
                EncoderParams::default()
                    .with_color_format(ColorFormat::I2)
                    .with_stride_align(8)
                    .with_mirx_coding(coding),
            );
            assert!(!bytes.is_empty(), "{coding:?}");
            match Mirx.decode(bytes) {
                MiData::RGBA(decoded) => assert_eq!(decoded, expected, "{coding:?}"),
                other => panic!("expected RGBA, got {}", other.variant_name()),
            }
        }
    }

    #[test]
    fn indexed_i8_frequency_profiles_preserve_indexes_and_color_table() {
        for coding in [
            MirxCoding::FrequencyReversible,
            MirxCoding::FrequencyQuantized(25),
        ] {
            let mut indexed = sample_indexed();
            indexed.bpp = 8;
            let expected = indexed.rgba.clone();
            let bytes = Mirx.encode(
                &MiData::INDEXED(indexed),
                EncoderParams::default()
                    .with_color_format(ColorFormat::I8)
                    .with_mirx_coding(coding),
            );
            assert!(!bytes.is_empty(), "{coding:?}");
            match Mirx.decode(bytes) {
                MiData::RGBA(decoded) => assert_eq!(decoded, expected, "{coding:?}"),
                other => panic!("expected RGBA, got {}", other.variant_name()),
            }
        }
    }

    #[test]
    fn indexed_pixel_profile_is_rejected_without_raw_fallback() {
        let bytes = Mirx.encode(
            &MiData::INDEXED(sample_indexed()),
            EncoderParams::default()
                .with_color_format(ColorFormat::I2)
                .with_mirx_coding(MirxCoding::Pixel),
        );
        assert!(bytes.is_empty());
    }

    #[test]
    fn frequency_profile_rejects_packed_indexes_without_raw_fallback() {
        let bytes = Mirx.encode(
            &MiData::INDEXED(sample_indexed()),
            EncoderParams::default()
                .with_color_format(ColorFormat::I2)
                .with_mirx_coding(MirxCoding::FrequencyReversible),
        );
        assert!(bytes.is_empty());
    }

    #[test]
    fn info_reports_flat_layout() {
        let img = sample_rgba(2, 2);
        let ed = Mirx;
        let params = EncoderParams::default().with_color_format(ColorFormat::BGRA8888);
        let bytes = ed.encode(&MiData::RGBA(img), params);
        let info = ed.info(&bytes);
        assert_eq!(info.width, 2);
        assert_eq!(info.height, 2);
        assert!(info.format.contains("BGRA8888"));
    }

    #[test]
    fn info_reports_encoded_image_coding() {
        let bytes = Mirx.encode(
            &MiData::RGBA(sample_rgba(4, 4)),
            EncoderParams::default()
                .with_color_format(ColorFormat::RGBA8888)
                .with_mirx_coding(MirxCoding::Lz4),
        );
        let info = Mirx.info(&bytes);
        assert_eq!((info.width, info.height), (4, 4));
        assert_eq!(info.other_info["layout"], "chunk");
        assert_eq!(info.other_info["chunks"]["image"]["coding"][0], "LZ4");
    }

    #[test]
    fn roundtrip_vector_chunk_preserves_ops() {
        let scene = mirx::Scene {
            ops: vec![mirx::SceneOp::FillPath {
                path: mirx::Path {
                    cmds: vec![
                        mirx::PathCmd::MoveTo(mirx::Point::new(
                            mirx::Fixed::from_int(0),
                            mirx::Fixed::from_int(0),
                        )),
                        mirx::PathCmd::LineTo(mirx::Point::new(
                            mirx::Fixed::from_int(10),
                            mirx::Fixed::from_int(0),
                        )),
                        mirx::PathCmd::LineTo(mirx::Point::new(
                            mirx::Fixed::from_int(10),
                            mirx::Fixed::from_int(10),
                        )),
                        mirx::PathCmd::Close,
                    ],
                },
                transform: mirx::Transform::IDENTITY,
                paint: mirx::Paint::Color(mirx::Color {
                    r: 255,
                    g: 128,
                    b: 0,
                    a: 255,
                }),
                opa: 200,
                fill_rule: mirx::FillRule::EvenOdd,
            }],
        };
        let ed = Mirx;
        let bytes = ed.encode(
            &MiData::PATH(SceneData {
                scene: scene.clone(),
            }),
            EncoderParams::default(),
        );
        assert!(ed.can_decode(&bytes));
        match ed.decode(bytes) {
            MiData::PATH(back) => assert_eq!(back.scene.ops.len(), 1),
            other => panic!("expected PATH, got {}", other.variant_name()),
        }
    }

    #[test]
    fn roundtrip_font_chunk_preserves_face() {
        use mirx::font::{
            FontAsset, GlyphMap, GlyphMetrics, GlyphSurfaceAsset, LineMetrics, RawGlyphs,
            RepresentationAsset,
        };
        use mirx::image::SampleLayout;

        let codepoints = ['A', 'B'];
        let map = GlyphMap::glyph_major(4, 4, codepoints.len()).unwrap();
        let data = [0u8; 16];
        let surface = RawGlyphs::builder(map, SampleLayout::A4)
            .build(&data)
            .unwrap();
        let metrics = [GlyphMetrics::new(
            mirx::Fixed::from_int(4),
            mirx::Fixed::ZERO,
            mirx::Fixed::from_int(3),
        ); 2];
        let line = LineMetrics::new(
            mirx::Fixed::from_int(3),
            mirx::Fixed::from_int(-1),
            mirx::Fixed::from_int(4),
        )
        .unwrap();
        let representation = RepresentationAsset::new(
            mirx::FontRepresentation::signed_distance(4, 1, 4, 2, 16, 16).unwrap(),
            0,
            line,
            &metrics,
        );
        let font = mirx::Font::from_asset(
            FontAsset::new(
                &codepoints,
                &[representation],
                &[GlyphSurfaceAsset::raw(surface)],
            ),
            &mirx::PayloadLimits::HOST,
        )
        .unwrap();
        let ed = Mirx;
        let bytes = ed.encode(
            &MiData::FONT(FontData::Mirx(font.clone())),
            EncoderParams::default(),
        );
        assert!(ed.can_decode(&bytes));
        match ed.decode(bytes) {
            MiData::FONT(FontData::Mirx(back)) => {
                assert_eq!(back.codepoints(), ['A', 'B']);
                assert_eq!(back.representation_count(), 1);
                assert_eq!(back.surface_count(), 1);
            }
            other => panic!("expected FONT Mirx, got {}", other.variant_name()),
        }
    }

    #[test]
    fn info_reports_vector_chunk_op_count() {
        let scene = mirx::Scene {
            ops: vec![mirx::SceneOp::FillPath {
                path: mirx::Path {
                    cmds: vec![mirx::PathCmd::Close],
                },
                transform: mirx::Transform::IDENTITY,
                paint: mirx::Paint::Color(mirx::Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                }),
                opa: 255,
                fill_rule: mirx::FillRule::EvenOdd,
            }],
        };
        let ed = Mirx;
        let bytes = ed.encode(&MiData::PATH(SceneData { scene }), EncoderParams::default());
        let info = ed.info(&bytes);
        let chunks = info
            .other_info
            .get("chunks")
            .and_then(|c| c.as_object())
            .unwrap();
        let vector = chunks.get("vector").unwrap();
        assert_eq!(vector.get("op_count").and_then(|v| v.as_u64()), Some(1));
    }
}
