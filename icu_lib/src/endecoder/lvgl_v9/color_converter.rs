use crate::endecoder::lvgl_v9::ColorFormat;

pub fn rgba8888_to(data: &[u8], color_format: ColorFormat) -> Vec<u8> {
    match color_format {
        ColorFormat::RGB888 => {
            let argb_iter = data.chunks_exact(4).map(|chunk| {
                let mut pixel = chunk[0..3].to_vec();
                pixel.rotate_right(1);
                pixel.reverse();
                pixel
            });

            argb_iter.flatten().collect()
        }
        ColorFormat::ARGB8888 => {
            let argb_iter = data.chunks_exact(4).map(|chunk| {
                let mut pixel = chunk.to_vec();
                pixel.rotate_right(1);
                pixel.reverse();
                pixel
            });

            argb_iter.flatten().collect()
        }
        ColorFormat::RGB565 => {
            unimplemented!()
        }
        ColorFormat::RGB565A8 => {
            unimplemented!()
        }
        ColorFormat::XRGB8888 => {
            unimplemented!()
        }
        _ => {
            unimplemented!()
        }
    }
}

pub fn rgba8888_from(data: &[u8], color_format: ColorFormat) -> Vec<u8> {
    match color_format {
        ColorFormat::RGB888 => {
            let argb_iter = data.chunks_exact(3).map(|chunk| {
                let mut pixel = chunk.to_vec();
                pixel.reverse();
                pixel.rotate_left(1);
                pixel.push(0);
                pixel
            });

            argb_iter.flatten().collect()
        }
        ColorFormat::ARGB8888 => {
            let argb_iter = data.chunks_exact(4).map(|chunk| {
                let mut pixel = chunk.to_vec();
                pixel.reverse();
                pixel.rotate_left(1);
                pixel
            });

            argb_iter.flatten().collect()
        }
        ColorFormat::RGB565 => {
            unimplemented!()
        }
        ColorFormat::RGB565A8 => {
            unimplemented!()
        }
        ColorFormat::XRGB8888 => {
            unimplemented!()
        }
        _ => {
            unimplemented!()
        }
    }
}
