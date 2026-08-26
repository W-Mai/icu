use super::model::Frame;
use image::codecs::webp::WebPEncoder;
use image::{ExtendedColorType, ImageEncoder};
use std::time::Duration;

const MAX_U24: u32 = 0x00ff_ffff;

pub(super) fn encode(
    frames: &[Frame],
    interval: Duration,
    loop_count: u16,
) -> Result<Vec<u8>, String> {
    let first = frames
        .first()
        .ok_or("Cannot encode an empty WebP animation")?;
    validate_dimension(first.width, "canvas width")?;
    validate_dimension(first.height, "canvas height")?;

    let mut encoded_frames = Vec::with_capacity(frames.len());
    let mut has_alpha = false;
    for frame in frames {
        let pixels = normalize_frame(frame, first.width, first.height)?;
        has_alpha |= pixels.chunks_exact(4).any(|pixel| pixel[3] != u8::MAX);
        let delay = if frame.delay.is_zero() {
            interval
        } else {
            frame.delay
        };
        encoded_frames.push((
            duration_ms(delay)?,
            encode_image_chunk(&pixels, first.width, first.height)?,
        ));
    }

    let mut chunks = Vec::new();
    let mut vp8x = vec![0x02 | if has_alpha { 0x10 } else { 0 }, 0, 0, 0];
    write_u24(&mut vp8x, first.width - 1)?;
    write_u24(&mut vp8x, first.height - 1)?;
    write_chunk(&mut chunks, b"VP8X", &vp8x)?;

    let mut anim = vec![0; 4];
    anim.extend_from_slice(&loop_count.to_le_bytes());
    write_chunk(&mut chunks, b"ANIM", &anim)?;

    for (duration, image_chunk) in encoded_frames {
        let mut anmf = Vec::with_capacity(16 + image_chunk.len());
        write_u24(&mut anmf, 0)?;
        write_u24(&mut anmf, 0)?;
        write_u24(&mut anmf, first.width - 1)?;
        write_u24(&mut anmf, first.height - 1)?;
        write_u24(&mut anmf, duration)?;
        anmf.push(0x02); // Full-canvas frames replace rather than blend.
        anmf.extend_from_slice(&image_chunk);
        write_chunk(&mut chunks, b"ANMF", &anmf)?;
    }

    let riff_size = chunks
        .len()
        .checked_add(4)
        .and_then(|size| u32::try_from(size).ok())
        .ok_or("WebP animation exceeds the RIFF size limit")?;
    let mut output = Vec::with_capacity(
        chunks
            .len()
            .checked_add(12)
            .ok_or("WebP animation size overflow")?,
    );
    output.extend_from_slice(b"RIFF");
    output.extend_from_slice(&riff_size.to_le_bytes());
    output.extend_from_slice(b"WEBP");
    output.extend_from_slice(&chunks);
    Ok(output)
}

fn validate_dimension(value: u32, label: &str) -> Result<(), String> {
    if value == 0 || value > MAX_U24 + 1 {
        Err(format!("Invalid WebP {label}: {value}"))
    } else {
        Ok(())
    }
}

fn normalize_frame(frame: &Frame, width: u32, height: u32) -> Result<Vec<u8>, String> {
    validate_dimension(frame.width, "frame width")?;
    validate_dimension(frame.height, "frame height")?;
    let right = frame
        .left
        .checked_add(frame.width)
        .ok_or("WebP frame horizontal bounds overflow")?;
    let bottom = frame
        .top
        .checked_add(frame.height)
        .ok_or("WebP frame vertical bounds overflow")?;
    if right > width || bottom > height {
        return Err(format!(
            "WebP frame {}x{} at {},{} exceeds the {}x{} canvas",
            frame.width, frame.height, frame.left, frame.top, width, height
        ));
    }

    let frame_width = usize::try_from(frame.width).map_err(|_| "WebP frame width is too large")?;
    let frame_height =
        usize::try_from(frame.height).map_err(|_| "WebP frame height is too large")?;
    let expected_pixels = frame_width
        .checked_mul(frame_height)
        .ok_or("WebP frame pixel count overflow")?;
    if frame.pixels.len() != expected_pixels {
        return Err(format!(
            "Invalid WebP frame pixel count: expected {expected_pixels}, got {}",
            frame.pixels.len()
        ));
    }

    let canvas_width = usize::try_from(width).map_err(|_| "WebP canvas width is too large")?;
    let canvas_height = usize::try_from(height).map_err(|_| "WebP canvas height is too large")?;
    let byte_len = canvas_width
        .checked_mul(canvas_height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("WebP canvas byte count overflow")?;
    let mut output = vec![0; byte_len];
    let left = usize::try_from(frame.left).map_err(|_| "WebP frame offset is too large")?;
    let top = usize::try_from(frame.top).map_err(|_| "WebP frame offset is too large")?;
    let row_bytes = frame_width
        .checked_mul(4)
        .ok_or("WebP frame row size overflow")?;
    for (y, row) in frame.pixels.chunks_exact(frame_width).enumerate() {
        let start = top
            .checked_add(y)
            .and_then(|y| y.checked_mul(canvas_width))
            .and_then(|offset| offset.checked_add(left))
            .and_then(|offset| offset.checked_mul(4))
            .ok_or("WebP frame row offset overflow")?;
        let end = start
            .checked_add(row_bytes)
            .ok_or("WebP frame row bounds overflow")?;
        let destination = output
            .get_mut(start..end)
            .ok_or("WebP frame row exceeds canvas")?;
        for (target, pixel) in destination.chunks_exact_mut(4).zip(row) {
            target.copy_from_slice(&pixel.to_array());
        }
    }
    Ok(output)
}

fn duration_ms(delay: Duration) -> Result<u32, String> {
    let millis = delay
        .as_nanos()
        .checked_add(500_000)
        .ok_or("WebP frame duration overflow")?
        / 1_000_000;
    let millis = millis.max(1);
    if millis > u128::from(MAX_U24) {
        return Err(format!("WebP frame duration exceeds 24 bits: {millis} ms"));
    }
    Ok(millis as u32)
}

fn encode_image_chunk(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let expected = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|count| count.checked_mul(4))
        .and_then(|count| usize::try_from(count).ok())
        .ok_or("WebP frame byte count overflow")?;
    if pixels.len() != expected {
        return Err(format!(
            "Invalid WebP RGBA byte count: expected {expected}, got {}",
            pixels.len()
        ));
    }

    let mut encoded = Vec::new();
    WebPEncoder::new_lossless(&mut encoded)
        .write_image(pixels, width, height, ExtendedColorType::Rgba8)
        .map_err(|error| error.to_string())?;
    extract_image_chunk(&encoded)
}

fn extract_image_chunk(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.get(..4) != Some(b"RIFF") || data.get(8..12) != Some(b"WEBP") {
        return Err("Static WebP encoder returned an invalid RIFF header".to_string());
    }
    let declared = read_u32(data, 4)? as usize;
    let riff_end = declared
        .checked_add(8)
        .ok_or("Static WebP RIFF size overflow")?;
    if riff_end != data.len() {
        return Err("Static WebP RIFF size does not match its payload".to_string());
    }

    let mut offset = 12usize;
    let mut image_chunk = None;
    while offset < riff_end {
        let header_end = offset.checked_add(8).ok_or("WebP chunk header overflow")?;
        let header = data
            .get(offset..header_end)
            .ok_or("Truncated WebP chunk header")?;
        let size = u32::from_le_bytes(
            header[4..8]
                .try_into()
                .map_err(|_| "Invalid WebP chunk size")?,
        ) as usize;
        let padded = size
            .checked_add(size & 1)
            .ok_or("WebP chunk size overflow")?;
        let chunk_end = header_end
            .checked_add(padded)
            .ok_or("WebP chunk bounds overflow")?;
        let chunk = data
            .get(offset..chunk_end)
            .ok_or("Truncated WebP chunk payload")?;
        match &header[..4] {
            b"VP8L" if image_chunk.replace(chunk.to_vec()).is_some() => {
                return Err("Static WebP contains multiple VP8L chunks".to_string());
            }
            b"VP8 " | b"ALPH" => {
                return Err("Static WebP contains incompatible lossy image chunks".to_string());
            }
            _ => {}
        }
        offset = chunk_end;
    }
    if offset != riff_end {
        return Err("Static WebP chunks do not fill the RIFF payload".to_string());
    }
    image_chunk.ok_or_else(|| "Static WebP contains no VP8L image chunk".to_string())
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, String> {
    let bytes = data
        .get(offset..offset.saturating_add(4))
        .ok_or("Truncated WebP integer")?;
    Ok(u32::from_le_bytes(
        bytes.try_into().map_err(|_| "Invalid WebP integer")?,
    ))
}

fn write_u24(output: &mut Vec<u8>, value: u32) -> Result<(), String> {
    if value > MAX_U24 {
        return Err(format!(
            "Value does not fit in a WebP 24-bit field: {value}"
        ));
    }
    output.extend_from_slice(&value.to_le_bytes()[..3]);
    Ok(())
}

fn write_chunk(output: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) -> Result<(), String> {
    let size = u32::try_from(payload.len()).map_err(|_| "WebP chunk exceeds 32 bits")?;
    let added = 8usize
        .checked_add(payload.len())
        .and_then(|value| value.checked_add(payload.len() & 1))
        .ok_or("WebP chunk size overflow")?;
    output
        .len()
        .checked_add(added)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or("WebP container exceeds the RIFF size limit")?;
    output.extend_from_slice(fourcc);
    output.extend_from_slice(&size.to_le_bytes());
    output.extend_from_slice(payload);
    if payload.len() & 1 != 0 {
        output.push(0);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Color32;
    use image::AnimationDecoder;
    use std::io::Cursor;

    fn frames() -> Vec<Frame> {
        vec![
            Frame {
                pixels: vec![Color32::RED, Color32::TRANSPARENT],
                width: 2,
                height: 1,
                left: 0,
                top: 0,
                delay: Duration::from_millis(80),
            },
            Frame {
                pixels: vec![Color32::BLUE, Color32::GREEN],
                width: 2,
                height: 1,
                left: 0,
                top: 0,
                delay: Duration::from_millis(120),
            },
        ]
    }

    #[test]
    fn round_trip_preserves_lossless_frames_and_delays() {
        let data = encode(&frames(), Duration::from_millis(100), 0).unwrap();
        let decoder = image::codecs::webp::WebPDecoder::new(Cursor::new(data)).unwrap();
        assert!(decoder.has_animation());
        let decoded = decoder.into_frames().collect_frames().unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].buffer().as_raw(), &[255, 0, 0, 255, 0, 0, 0, 0]);
        assert_eq!(
            decoded[1].buffer().as_raw(),
            &[0, 0, 255, 255, 0, 255, 0, 255]
        );
        assert_eq!(decoded[0].delay().numer_denom_ms(), (80, 1));
        assert_eq!(decoded[1].delay().numer_denom_ms(), (120, 1));
    }

    #[test]
    fn partial_frames_are_composited_into_the_canvas() {
        let mut partial = frames();
        partial[1] = Frame {
            pixels: vec![Color32::GREEN],
            width: 1,
            height: 1,
            left: 1,
            top: 0,
            delay: Duration::from_millis(120),
        };

        let data = encode(&partial, Duration::from_millis(100), 0).unwrap();
        let decoder = image::codecs::webp::WebPDecoder::new(Cursor::new(data)).unwrap();
        let decoded = decoder.into_frames().collect_frames().unwrap();
        assert_eq!(decoded[1].buffer().as_raw(), &[0, 0, 0, 0, 0, 255, 0, 255]);
    }

    #[test]
    fn writes_requested_loop_count_and_valid_riff_size() {
        let data = encode(&frames(), Duration::from_millis(100), 7).unwrap();
        assert_eq!(read_u32(&data, 4).unwrap() as usize + 8, data.len());
        let anim = data.windows(4).position(|bytes| bytes == b"ANIM").unwrap();
        assert_eq!(&data[anim + 12..anim + 14], &7u16.to_le_bytes());
    }

    #[test]
    fn rejects_invalid_frames_and_durations() {
        let mut invalid = frames();
        invalid[1].width = 3;
        assert!(encode(&invalid, Duration::from_millis(100), 0).is_err());

        let mut delayed = frames();
        delayed[0].delay = Duration::from_millis(u64::from(MAX_U24) + 1);
        assert!(encode(&delayed, Duration::from_millis(100), 0).is_err());
    }

    #[test]
    fn rejects_malformed_static_webp_containers() {
        assert!(extract_image_chunk(b"RIFF\0\0\0\0WEBP").is_err());
        assert!(extract_image_chunk(b"not a webp").is_err());

        let pixels = [255, 0, 0, 255];
        let mut encoded = Vec::new();
        WebPEncoder::new_lossless(&mut encoded)
            .write_image(&pixels, 1, 1, ExtendedColorType::Rgba8)
            .unwrap();
        let chunk = encoded
            .windows(4)
            .position(|bytes| bytes == b"VP8L")
            .unwrap();
        for incompatible in [b"VP8 ", b"ALPH"] {
            let mut malformed = encoded.clone();
            malformed[chunk..chunk + 4].copy_from_slice(incompatible);
            assert!(extract_image_chunk(&malformed).is_err());
        }
    }
}
