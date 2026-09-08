use super::rgba_to_mirx_pixels;
use crate::endecoder::common::animation::Animation;
use crate::endecoder::ColorFormat;
use mirx::document::EncodeOptions;
use mirx::frames::{FrameEncodingSet, FramePolicy, FrameSequence, FrameWriteReport, FramesEncoder};
use mirx::image::{ColorDescription, SampleLayout, SurfaceDescriptor};
use mirx::reader::{PayloadLimits, ReadOptions};
use mirx::types::ByteAlignment;
use mirx::{ChunkFlags, Document, Reader};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FramesOptions {
    format: ColorFormat,
    timescale_hz: u32,
    default_duration_ticks: u32,
    play_count: u32,
    max_delta_frames: u16,
    tiles: Option<(u32, u32)>,
    input_alignment: ByteAlignment,
    quality: Option<u8>,
}

impl FramesOptions {
    pub const fn new() -> Self {
        Self {
            format: ColorFormat::RGBA8888,
            timescale_hz: 1_000,
            default_duration_ticks: 100,
            play_count: 0,
            max_delta_frames: 8,
            tiles: Some((32, 32)),
            input_alignment: ByteAlignment::ONE,
            quality: None,
        }
    }

    pub const fn with_format(mut self, format: ColorFormat) -> Self {
        self.format = format;
        self
    }

    pub const fn with_timebase(mut self, timescale_hz: u32) -> Self {
        self.timescale_hz = timescale_hz;
        self
    }

    pub const fn with_default_duration(mut self, ticks: u32) -> Self {
        self.default_duration_ticks = ticks;
        self
    }

    pub const fn with_play_count(mut self, play_count: u32) -> Self {
        self.play_count = play_count;
        self
    }

    pub const fn with_max_delta_frames(mut self, frames: u16) -> Self {
        self.max_delta_frames = frames;
        self
    }

    pub const fn with_tiles(mut self, width: u32, height: u32) -> Self {
        self.tiles = Some((width, height));
        self
    }

    pub const fn without_tiles(mut self) -> Self {
        self.tiles = None;
        self
    }

    pub const fn with_input_alignment(mut self, alignment: ByteAlignment) -> Self {
        self.input_alignment = alignment;
        self
    }

    pub const fn with_quality(mut self, quality: u8) -> Self {
        self.quality = Some(quality);
        self
    }
}

impl Default for FramesOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FramesOutput {
    bytes: Vec<u8>,
    reports: Vec<FrameWriteReport>,
}

impl FramesOutput {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn reports(&self) -> &[FrameWriteReport] {
        &self.reports
    }
}

#[derive(Debug)]
pub enum FramesError {
    Empty,
    TooManyFrames(usize),
    UnsupportedFormat(ColorFormat),
    InvalidSurface {
        width: u32,
        height: u32,
        format: ColorFormat,
    },
    InvalidSequence(mirx::frames::FrameSequenceError),
    InvalidQuality(mirx::coding::FrequencyError),
    FrameDuration {
        frame: usize,
        timescale_hz: u32,
    },
    FramePixels {
        frame: usize,
        format: ColorFormat,
    },
    Write(mirx::frames::FrameWriteError),
    Document(mirx::document::EditError),
    Encode(mirx::document::EncodeError),
    Verify(mirx::reader::ReadError),
    MissingFrames,
    Frames(mirx::frames::FramesError),
}

impl fmt::Display for FramesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("animation contains no frames"),
            Self::TooManyFrames(count) => {
                write!(
                    formatter,
                    "animation frame count {count} exceeds MIRX limits"
                )
            }
            Self::UnsupportedFormat(format) => {
                write!(formatter, "MIRX frame output does not support {format:?}")
            }
            Self::InvalidSurface {
                width,
                height,
                format,
            } => write!(
                formatter,
                "invalid MIRX frame surface {width}x{height} {format:?}"
            ),
            Self::InvalidSequence(error) => {
                write!(formatter, "invalid MIRX frame sequence: {error:?}")
            }
            Self::InvalidQuality(error) => {
                write!(formatter, "invalid MIRX frequency quality: {error:?}")
            }
            Self::FrameDuration {
                frame,
                timescale_hz,
            } => write!(
                formatter,
                "frame {frame} duration cannot use a {timescale_hz} Hz timebase"
            ),
            Self::FramePixels { frame, format } => {
                write!(formatter, "frame {frame} cannot convert to {format:?}")
            }
            Self::Write(error) => write!(formatter, "cannot encode MIRX frame: {error:?}"),
            Self::Document(error) => write!(formatter, "cannot build MIRX document: {error:?}"),
            Self::Encode(error) => write!(formatter, "cannot encode MIRX document: {error:?}"),
            Self::Verify(error) => write!(formatter, "encoded MIRX document is invalid: {error:?}"),
            Self::MissingFrames => {
                formatter.write_str("encoded MIRX document has no primary FRAMES chunk")
            }
            Self::Frames(error) => write!(
                formatter,
                "encoded MIRX FRAMES payload is invalid: {error:?}"
            ),
        }
    }
}

impl std::error::Error for FramesError {}

pub fn encode(animation: &Animation, options: FramesOptions) -> Result<FramesOutput, FramesError> {
    let frame_count = u32::try_from(animation.frames().len())
        .map_err(|_| FramesError::TooManyFrames(animation.frames().len()))?;
    if frame_count == 0 {
        return Err(FramesError::Empty);
    }
    let format = options
        .format
        .to_mirx()
        .filter(|format| {
            matches!(
                *format,
                mirx::image::ColorFormat::RGB565
                    | mirx::image::ColorFormat::RGB565Swapped
                    | mirx::image::ColorFormat::RGB888
                    | mirx::image::ColorFormat::XRGB8888
                    | mirx::image::ColorFormat::RGBA8888
                    | mirx::image::ColorFormat::BGRA8888
            )
        })
        .ok_or(FramesError::UnsupportedFormat(options.format))?;
    let surface = SurfaceDescriptor::new(
        animation.width(),
        animation.height(),
        SampleLayout::from_color_format(format),
        ColorDescription::SRGB,
    )
    .map_err(|_| FramesError::InvalidSurface {
        width: animation.width(),
        height: animation.height(),
        format: options.format,
    })?;
    let max_delta_frames = options
        .max_delta_frames
        .min(u16::try_from(animation.frames().len().saturating_sub(1)).unwrap_or(u16::MAX));
    let sequence = FrameSequence::new(
        frame_count,
        options.timescale_hz,
        options.default_duration_ticks,
    )
    .map_err(FramesError::InvalidSequence)?
    .with_play_count(options.play_count)
    .with_max_delta_frames(max_delta_frames)
    .map_err(FramesError::InvalidSequence)?;
    let mut profiles = FrameEncodingSet::lossless();
    if let Some(quality) = options.quality {
        profiles = profiles
            .with_quantized_frequency(quality)
            .map_err(FramesError::InvalidQuality)?;
    }
    let policy = if options.quality.is_some() {
        FramePolicy::new(max_delta_frames).allow_lossy()
    } else {
        FramePolicy::new(max_delta_frames)
    };
    let mut encoder = FramesEncoder::new(sequence, surface)
        .map_err(FramesError::Write)?
        .with_profiles(profiles)
        .map_err(FramesError::Write)?
        .with_policy(policy)
        .map_err(FramesError::Write)?
        .with_input_alignment(options.input_alignment)
        .map_err(FramesError::Write)?;
    encoder = match options.tiles {
        Some((width, height)) => encoder
            .with_tiles(width, height)
            .map_err(FramesError::Write)?,
        None => encoder.without_tiles().map_err(FramesError::Write)?,
    };

    let stride = format
        .minimum_stride(animation.width())
        .ok_or(FramesError::InvalidSurface {
            width: animation.width(),
            height: animation.height(),
            format: options.format,
        })?;
    for (index, frame) in animation.frames().iter().enumerate() {
        let samples = rgba_to_mirx_pixels(frame.pixels(), format, stride).ok_or(
            FramesError::FramePixels {
                frame: index,
                format: options.format,
            },
        )?;
        let source_ticks =
            frame
                .duration()
                .ticks(options.timescale_hz)
                .ok_or(FramesError::FrameDuration {
                    frame: index,
                    timescale_hz: options.timescale_hz,
                })?;
        let duration_ticks = if frame.duration().numerator_ms() == 0 {
            options.default_duration_ticks
        } else {
            source_ticks.max(1)
        };
        encoder
            .push_with_duration(&samples, duration_ticks)
            .map_err(FramesError::Write)?;
    }

    let encoded = encoder.finish().map_err(FramesError::Write)?;
    let reports = encoded.reports().to_vec();
    let mut document = Document::new_with_limits(PayloadLimits::HOST);
    let id = document
        .push_frames_with_flags(encoded, ChunkFlags::CRITICAL)
        .map_err(FramesError::Document)?;
    document.set_primary(id).map_err(FramesError::Document)?;
    let bytes = document
        .encode(&EncodeOptions::new())
        .map_err(FramesError::Encode)?;
    verify(&bytes)?;
    Ok(FramesOutput { bytes, reports })
}

fn verify(bytes: &[u8]) -> Result<(), FramesError> {
    let options = ReadOptions::new().with_payload_limits(PayloadLimits::HOST);
    let reader = Reader::open_with(bytes, &options).map_err(FramesError::Verify)?;
    let primary = reader
        .primary()
        .map_err(FramesError::Verify)?
        .ok_or(FramesError::MissingFrames)?;
    let frames = primary
        .frames(&PayloadLimits::HOST)
        .map_err(FramesError::Frames)?
        .ok_or(FramesError::MissingFrames)?;
    frames.validate_data().map_err(FramesError::Frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endecoder::common::animation;
    use image::codecs::gif::GifEncoder;
    use image::{Delay, Frame, RgbaImage};
    use std::io::Cursor;

    fn animation() -> Animation {
        let mut bytes = Vec::new();
        GifEncoder::new(&mut bytes)
            .encode_frames([
                Frame::from_parts(
                    RgbaImage::from_pixel(3, 2, image::Rgba([1, 2, 3, 255])),
                    0,
                    0,
                    Delay::from_numer_denom_ms(40, 1),
                ),
                Frame::from_parts(
                    RgbaImage::from_pixel(3, 2, image::Rgba([1, 2, 3, 255])),
                    0,
                    0,
                    Delay::from_numer_denom_ms(70, 1),
                ),
            ])
            .unwrap();
        animation::decode(&bytes).unwrap().unwrap()
    }

    #[test]
    fn writes_timing_omission_and_aligned_critical_frames() {
        let output = encode(
            &animation(),
            FramesOptions::new().with_input_alignment(ByteAlignment::new(64).unwrap()),
        )
        .unwrap();
        assert_eq!(output.reports().len(), 2);
        assert_eq!(output.reports()[1].encoding(), None);

        let options = ReadOptions::new().with_payload_limits(PayloadLimits::HOST);
        let reader = Reader::open_with(output.bytes(), &options).unwrap();
        let frames = reader
            .primary()
            .unwrap()
            .unwrap()
            .frames(&PayloadLimits::HOST)
            .unwrap()
            .unwrap();
        assert_eq!(frames.sequence().frame_count(), 2);
        assert_eq!(frames.frame(0).unwrap().duration_ticks(), 40);
        assert_eq!(frames.frame(1).unwrap().duration_ticks(), 70);
    }

    #[test]
    fn lossy_frequency_requires_explicit_quality() {
        let animation = animation();
        let lossless = encode(&animation, FramesOptions::new()).unwrap();
        assert!(lossless.reports().iter().all(|report| {
            report.encoding() != Some(mirx::frames::FrameEncoding::FrequencyQuantized(75))
        }));
        encode(&animation, FramesOptions::new().with_quality(75)).unwrap();
    }

    #[test]
    fn rejects_indexed_output_without_a_shared_palette() {
        let error = encode(
            &animation(),
            FramesOptions::new().with_format(ColorFormat::I8),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            FramesError::UnsupportedFormat(ColorFormat::I8)
        ));
    }

    #[test]
    fn static_png_stays_outside_the_animation_path() {
        let image = RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        assert!(animation::decode(&bytes).unwrap().is_none());
    }
}
