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

                rgb.to_le_bytes()
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

                rgb.to_le_bytes()
            });

            let alpha_iter = data.chunks_exact(4).map(|chunk| chunk[3]);

            let mut tmp = argb_iter.flatten().collect::<Vec<u8>>();
            tmp.extend(alpha_iter);
            tmp
        }
        ColorFormat::A1 | ColorFormat::A2 | ColorFormat::A4 | ColorFormat::A8 => {
            let bpp = match color_format {
                ColorFormat::A1 => 1,
                ColorFormat::A2 => 2,
                ColorFormat::A4 => 4,
                ColorFormat::A8 => 8,
                _ => return Vec::new(),
            };

            let alpha_iter = data.chunks_exact(4).map(|chunk| chunk[3]);

            let mut tmp = Vec::new();
            for (i, alpha) in alpha_iter.enumerate() {
                if i % (8 / bpp) == 0 {
                    tmp.push(0);
                }
                let byte = tmp.last_mut().unwrap();
                *byte |= (alpha >> (8 - bpp)) << (i % (8 / bpp));
            }
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
                let pixel = chunk.to_vec();
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
                let pixel = chunk.to_vec();
                let rgb = u16::from_le_bytes([pixel[0], pixel[1]]);
                let r = ((rgb >> 11) & 0x1F) as u8;
                let g = ((rgb >> 5) & 0x3F) as u8;
                let b = (rgb & 0x1F) as u8;
                vec![r << 3, g << 2, b << 3]
            });

            let alpha_iter = data.chunks_exact(1).map(|chunk| chunk[0]);

            let rgba_iter = argb_iter.zip(alpha_iter).map(|(rgb, alpha)| {
                let mut pixel = rgb;
                pixel.push(alpha);
                pixel
            });
            rgba_iter.flatten().collect()
        }
        ColorFormat::A1 | ColorFormat::A2 | ColorFormat::A4 | ColorFormat::A8 => {
            let bpp = match color_format {
                ColorFormat::A1 => 1,
                ColorFormat::A2 => 2,
                ColorFormat::A4 => 4,
                ColorFormat::A8 => 8,
                _ => return Vec::new(),
            };

            let alpha_iter = data.chunks_exact(1).map(|chunk| chunk[0]);

            let alpha_iter = alpha_iter.flat_map(|alpha| {
                (0u8..8u8 / bpp).map(move |i| {
                    ((((alpha as u16) >> ((8 / bpp - i - 1) * bpp)) & ((1 << bpp) - 1))
                        << (8 / bpp)) as u8
                })
            });

            let rgba_iter = alpha_iter.map(|alpha| vec![0, 0, 0, alpha]);

            rgba_iter.flatten().collect()
        }
        _ => {
            unimplemented!()
        }
    }
}
