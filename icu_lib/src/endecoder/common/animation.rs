use image::codecs::{gif::GifDecoder, png::PngDecoder, webp::WebPDecoder};
use image::{AnimationDecoder, RgbaImage};
use std::fmt;
use std::io::Cursor;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameDuration {
    numerator_ms: u32,
    denominator: u32,
}

impl FrameDuration {
    pub const fn numerator_ms(self) -> u32 {
        self.numerator_ms
    }

    pub const fn denominator(self) -> u32 {
        self.denominator
    }

    pub fn as_duration(self) -> Duration {
        Duration::from_secs_f64(
            f64::from(self.numerator_ms) / f64::from(self.denominator) / 1_000.0,
        )
    }

    pub fn ticks(self, timescale_hz: u32) -> Option<u32> {
        let numerator = u64::from(self.numerator_ms).checked_mul(u64::from(timescale_hz))?;
        let denominator = u64::from(self.denominator).checked_mul(1_000)?;
        let rounded = numerator.checked_add(denominator / 2)? / denominator;
        u32::try_from(rounded).ok()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationFrame {
    pixels: RgbaImage,
    duration: FrameDuration,
}

impl AnimationFrame {
    pub fn pixels(&self) -> &RgbaImage {
        &self.pixels
    }

    pub fn into_pixels(self) -> RgbaImage {
        self.pixels
    }

    pub const fn duration(&self) -> FrameDuration {
        self.duration
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    width: u32,
    height: u32,
    frames: Vec<AnimationFrame>,
}

impl Animation {
    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn frames(&self) -> &[AnimationFrame] {
        &self.frames
    }

    pub fn into_frames(self) -> Vec<AnimationFrame> {
        self.frames
    }
}

#[derive(Debug)]
pub enum AnimationError {
    Decode(image::ImageError),
    Empty,
    Offset {
        frame: usize,
        left: u32,
        top: u32,
    },
    Dimensions {
        frame: usize,
        width: u32,
        height: u32,
        expected_width: u32,
        expected_height: u32,
    },
}

impl fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(formatter, "cannot decode animation: {error}"),
            Self::Empty => formatter.write_str("animation contains no frames"),
            Self::Offset { frame, left, top } => {
                write!(
                    formatter,
                    "frame {frame} starts at unsupported offset ({left}, {top})"
                )
            }
            Self::Dimensions {
                frame,
                width,
                height,
                expected_width,
                expected_height,
            } => write!(
                formatter,
                "frame {frame} is {width}x{height}, expected {expected_width}x{expected_height}"
            ),
        }
    }
}

impl std::error::Error for AnimationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

pub fn decode(data: &[u8]) -> Result<Option<Animation>, AnimationError> {
    let frames = if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        GifDecoder::new(Cursor::new(data))
            .map_err(AnimationError::Decode)?
            .into_frames()
            .collect_frames()
            .map_err(AnimationError::Decode)?
    } else if data.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]) {
        let decoder = PngDecoder::new(Cursor::new(data)).map_err(AnimationError::Decode)?;
        if !decoder.is_apng().map_err(AnimationError::Decode)? {
            return Ok(None);
        }
        decoder
            .apng()
            .map_err(AnimationError::Decode)?
            .into_frames()
            .collect_frames()
            .map_err(AnimationError::Decode)?
    } else if is_webp(data) {
        let decoder = WebPDecoder::new(Cursor::new(data)).map_err(AnimationError::Decode)?;
        if !decoder.has_animation() {
            return Ok(None);
        }
        decoder
            .into_frames()
            .collect_frames()
            .map_err(AnimationError::Decode)?
    } else {
        return Ok(None);
    };

    let first = frames.first().ok_or(AnimationError::Empty)?;
    let width = first.buffer().width();
    let height = first.buffer().height();
    let mut decoded = Vec::with_capacity(frames.len());
    for (index, frame) in frames.into_iter().enumerate() {
        if frame.left() != 0 || frame.top() != 0 {
            return Err(AnimationError::Offset {
                frame: index,
                left: frame.left(),
                top: frame.top(),
            });
        }
        if frame.buffer().dimensions() != (width, height) {
            return Err(AnimationError::Dimensions {
                frame: index,
                width: frame.buffer().width(),
                height: frame.buffer().height(),
                expected_width: width,
                expected_height: height,
            });
        }
        let (numerator_ms, denominator) = frame.delay().numer_denom_ms();
        decoded.push(AnimationFrame {
            pixels: frame.into_buffer(),
            duration: FrameDuration {
                numerator_ms,
                denominator,
            },
        });
    }
    Ok(Some(Animation {
        width,
        height,
        frames: decoded,
    }))
}

fn is_webp(data: &[u8]) -> bool {
    data.get(..4) == Some(b"RIFF") && data.get(8..12) == Some(b"WEBP")
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame};

    fn gif() -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut bytes);
            encoder.set_repeat(Repeat::Infinite).unwrap();
            encoder
                .encode_frames([
                    Frame::from_parts(
                        RgbaImage::from_pixel(2, 1, image::Rgba([1, 2, 3, 255])),
                        0,
                        0,
                        Delay::from_numer_denom_ms(40, 1),
                    ),
                    Frame::from_parts(
                        RgbaImage::from_pixel(2, 1, image::Rgba([4, 5, 6, 255])),
                        0,
                        0,
                        Delay::from_numer_denom_ms(75, 1),
                    ),
                ])
                .unwrap();
        }
        bytes
    }

    #[test]
    fn decodes_full_frames_and_container_timing() {
        let animation = decode(&gif()).unwrap().unwrap();
        assert_eq!((animation.width(), animation.height()), (2, 1));
        assert_eq!(animation.frames().len(), 2);
        assert_eq!(animation.frames()[0].duration().ticks(1_000), Some(40));
        assert_eq!(animation.frames()[1].duration().ticks(1_000), Some(70));
        assert_eq!(
            animation.frames()[1].pixels().get_pixel(0, 0).0,
            [4, 5, 6, 255]
        );
    }

    #[test]
    fn static_png_is_not_reported_as_animation() {
        let image = RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        assert!(decode(&bytes).unwrap().is_none());
    }
}
