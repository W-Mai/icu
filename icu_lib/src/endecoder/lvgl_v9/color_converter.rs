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
        ColorFormat::ARGB8888 | ColorFormat::XRGB8888 => {
            let argb_iter = data.chunks_exact(4).map(|chunk| {
                let mut pixel = chunk.to_vec();
                pixel.rotate_right(1);
                pixel.reverse();
                pixel
            });

            argb_iter.flatten().collect()
        }
        ColorFormat::RGB565 => {
            let argb_iter = data.chunks_exact(4).map(|chunk| {
                let pixel = chunk[0..3].to_vec();

                let r = (pixel[0] >> 3) as u16;
                let g = (pixel[1] >> 2) as u16;
                let b = (pixel[2] >> 3) as u16;
                let rgb = (r << 11) | (g << 5) | b;
                let rgb = rgb.to_le_bytes();

                let pixel = vec![rgb[0], rgb[1]];

                pixel
            });

            argb_iter.flatten().collect()
        }
        ColorFormat::RGB565A8 => {
            let argb_iter = data.chunks_exact(4).map(|chunk| {
                let pixel = chunk[0..3].to_vec();

                let r = (pixel[0] >> 3) as u16;
                let g = (pixel[1] >> 2) as u16;
                let b = (pixel[2] >> 3) as u16;
                let rgb = (r << 11) | (g << 5) | b;
                let rgb = rgb.to_le_bytes();

                let pixel = vec![rgb[0], rgb[1]];

                pixel
            });

            let alpha_iter = data.chunks_exact(4).map(|chunk| chunk[3]);

            let mut tmp = argb_iter.flatten().collect::<Vec<u8>>();
            tmp.extend(alpha_iter);
            tmp
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
        ColorFormat::ARGB8888 | ColorFormat::XRGB8888 => {
            let argb_iter = data.chunks_exact(4).map(|chunk| {
                let mut pixel = chunk.to_vec();
                pixel.reverse();
                pixel.rotate_left(1);
                pixel
            });

            argb_iter.flatten().collect()
        }
        ColorFormat::RGB565 => {
            let argb_iter = data.chunks_exact(2).map(|chunk| {
                let mut pixel = chunk.to_vec();
                pixel.reverse();
                let rgb = u16::from_le_bytes([pixel[0], pixel[1]]);
                let r = ((rgb >> 11) & 0x1F) as u8;
                let g = ((rgb >> 5) & 0x3F) as u8;
                let b = (rgb & 0x1F) as u8;
                vec![r << 3, g << 2, b << 3, 0xFF]
            });

            argb_iter.flatten().collect()
        }
        ColorFormat::RGB565A8 => {
            let argb_iter = data.chunks_exact(2).map(|chunk| {
                let mut pixel = chunk.to_vec();
                pixel.reverse();
                let rgb = u16::from_le_bytes([pixel[0], pixel[1]]);
                let r = ((rgb >> 11) & 0x1F) as u8;
                let g = ((rgb >> 5) & 0x3F) as u8;
                let b = (rgb & 0x1F) as u8;
                vec![r << 3, g << 2, b << 3]
            });

            let alpha_iter = data.chunks_exact(1).map(|chunk| chunk[0]);

            let mut tmp = argb_iter.flatten().collect::<Vec<u8>>();
            tmp.extend(alpha_iter);
            tmp
        }
        _ => {
            unimplemented!()
        }
    }
}
