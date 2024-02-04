use crate::endecoder::lvgl_v9::ColorFormat;

pub fn rgba8888_to(
    data: &[u8],
    color_format: ColorFormat,
    width: u32,
    height: u32,
    stride: u32,
) -> Vec<u8> {
    let stride_bytes = stride as usize;
    let color_bytes = ColorFormat::ARGB8888.get_size() as usize;
    let width_bytes = width as usize * color_bytes;

    match color_format {
        ColorFormat::RGB888 => {
            let argb_iter = data.chunks_exact(width_bytes).map(|row| {
                let mut row = row
                    .chunks_exact(color_bytes)
                    .flat_map(|chunk| {
                        let mut pixel = chunk[0..3].to_vec();
                        pixel.reverse();
                        pixel
                    })
                    .collect::<Vec<u8>>();
                row.resize(stride_bytes, 0);
                row
            });

            argb_iter.flatten().collect()
        }
        ColorFormat::ARGB8888 | ColorFormat::XRGB8888 => {
            let argb_iter = data.chunks_exact(width_bytes).map(|row| {
                let mut row = row
                    .chunks_exact(color_bytes)
                    .flat_map(|chunk| {
                        let mut pixel = chunk.to_vec();
                        pixel.rotate_right(1);
                        pixel.reverse();
                        pixel
                    })
                    .collect::<Vec<u8>>();
                row.resize(stride_bytes, 0);
                row
            });

            argb_iter.flatten().collect()
        }
        ColorFormat::RGB565 => {
            let argb_iter = data.chunks_exact(width_bytes).map(|row| {
                let mut row = row
                    .chunks_exact(color_bytes)
                    .flat_map(|chunk| {
                        let pixel = chunk[0..3].to_vec();

                        let r = (pixel[0] >> 3) as u16;
                        let g = (pixel[1] >> 2) as u16;
                        let b = (pixel[2] >> 3) as u16;
                        let rgb = (r << 11) | (g << 5) | b;

                        rgb.to_le_bytes()
                    })
                    .collect::<Vec<u8>>();
                row.resize(stride_bytes, 0);
                row
            });

            argb_iter.flatten().collect()
        }
        ColorFormat::RGB565A8 => {
            let argb_iter = data.chunks_exact(width_bytes).map(|row| {
                let mut row = row
                    .chunks_exact(color_bytes)
                    .flat_map(|chunk| {
                        let pixel = chunk[0..3].to_vec();

                        let r = (pixel[0] >> 3) as u16;
                        let g = (pixel[1] >> 2) as u16;
                        let b = (pixel[2] >> 3) as u16;
                        let rgb = (r << 11) | (g << 5) | b;

                        rgb.to_le_bytes()
                    })
                    .collect::<Vec<u8>>();
                row.resize(stride_bytes, 0);
                row
            });

            let alpha_iter = data.chunks_exact(color_bytes).map(|chunk| chunk[3]);

            let mut argb = argb_iter.flatten().collect::<Vec<u8>>();
            argb.extend(alpha_iter);
            argb
        }
        ColorFormat::A1 | ColorFormat::A2 | ColorFormat::A4 | ColorFormat::A8 => {
            let bpp = color_format.get_bpp();

            let mut alpha_iter = data.chunks_exact(color_bytes).map(|chunk| chunk[3]);

            let mut alphas = vec![0; stride_bytes * height as usize];
            alphas.chunks_exact_mut(stride_bytes).for_each(|row| {
                let mut iter = row.iter_mut();
                let mut byte = &mut 0u8;

                for i in 0..width as u16 {
                    let alpha = alpha_iter.next().unwrap();
                    if i % (8 / bpp) == 0 {
                        if let Some(next_byte) = iter.next() {
                            byte = next_byte;
                        } else {
                            break;
                        }
                    }
                    *byte |= (alpha >> (8 - bpp)) << ((8 / bpp - 1 - i % (8 / bpp)) * bpp);
                }
            });

            alphas
        }
        ColorFormat::L8 => {
            // (R+R+R+B+G+G+G+G) >> 3
            let argb_iter = data.chunks_exact(width_bytes).flat_map(|row| {
                let mut row = row
                    .chunks_exact(4)
                    .map(|chunk| {
                        let r = chunk[0] as u16;
                        let g = chunk[1] as u16;
                        let b = chunk[2] as u16;
                        let a = chunk[3] as u16;
                        let l = ((r + r + r + b + g + g + g + g) >> 3) * a / 0xFF;

                        l as u8
                    })
                    .collect::<Vec<u8>>();
                row.resize(stride_bytes, 0);
                row
            });

            argb_iter.collect()
        }
        ColorFormat::I1 | ColorFormat::I2 | ColorFormat::I4 | ColorFormat::I8 => {
            let bpp = color_format.get_bpp();
            let color_map_size = 1 << bpp;
            let nq = color_quant::NeuQuant::new(30, color_map_size, data);
            let color_map = nq.color_map_rgba();
            let mut color_map = rgba8888_to(
                &color_map,
                ColorFormat::ARGB8888,
                color_map_size as u32,
                1,
                ColorFormat::ARGB8888.get_stride_size(color_map_size as u32, 1),
            );

            let mut indexes_iter = data.chunks(color_bytes).map(|pix| nq.index_of(pix) as u8);

            let mut indexes = vec![0; stride_bytes * height as usize];
            indexes.chunks_exact_mut(stride_bytes).for_each(|row| {
                let mut iter = row.iter_mut();
                let mut byte = &mut 0u8;

                for i in 0..width as u16 {
                    let alpha = indexes_iter.next().unwrap();
                    if i % (8 / bpp) == 0 {
                        if let Some(next_byte) = iter.next() {
                            byte = next_byte;
                        } else {
                            break;
                        }
                    }
                    *byte |= (alpha) << ((8 / bpp - 1 - i % (8 / bpp)) * bpp);
                }
            });
            color_map.extend(indexes);
            color_map
        }
        _ => {
            unimplemented!()
        }
    }
}

pub fn rgba8888_from(
    data: &[u8],
    color_format: ColorFormat,
    width: u32,
    height: u32,
    stride: u32,
) -> Vec<u8> {
    let stride_bytes = stride as usize;
    let color_bytes = color_format.get_size() as usize;
    let width_bytes = color_format.get_stride_size(width, 1) as usize;

    match color_format {
        ColorFormat::RGB888 => {
            let argb_iter = data.chunks_exact(stride_bytes).flat_map(|row| {
                row.chunks_exact(width_bytes)
                    .next()
                    .unwrap()
                    .chunks_exact(color_bytes)
                    .map(|chunk| {
                        let mut pixel = chunk.to_vec();
                        pixel.reverse();
                        pixel.push(0xFF);
                        pixel
                    })
            });
            argb_iter.flatten().collect()
        }
        ColorFormat::ARGB8888 | ColorFormat::XRGB8888 => {
            let argb_iter = data.chunks_exact(stride_bytes).flat_map(|row| {
                row.chunks_exact(width_bytes)
                    .next()
                    .unwrap()
                    .chunks_exact(color_bytes)
                    .map(|chunk| {
                        let mut pixel = chunk.to_vec();
                        pixel.rotate_right(1);
                        pixel.reverse();
                        pixel
                    })
            });
            argb_iter.flatten().collect()
        }
        ColorFormat::RGB565 => {
            let argb_iter = data.chunks_exact(stride_bytes).flat_map(|row| {
                row.chunks_exact(width_bytes)
                    .next()
                    .unwrap()
                    .chunks_exact(color_bytes)
                    .map(|chunk| {
                        let rgb = u16::from_le_bytes([chunk[0], chunk[1]]);
                        let r = ((rgb >> 11) & 0x1F) as u8;
                        let g = ((rgb >> 5) & 0x3F) as u8;
                        let b = (rgb & 0x1F) as u8;
                        vec![r << 3, g << 2, b << 3, 0xFF]
                    })
            });
            argb_iter.flatten().collect()
        }
        ColorFormat::RGB565A8 => {
            let argb_iter = data.chunks_exact(stride_bytes).flat_map(|row| {
                row.chunks_exact(width_bytes)
                    .next()
                    .unwrap()
                    .chunks_exact(2)
                    .map(|chunk| {
                        let rgb = u16::from_le_bytes([chunk[0], chunk[1]]);
                        let r = ((rgb >> 11) & 0x1F) as u8;
                        let g = ((rgb >> 5) & 0x3F) as u8;
                        let b = (rgb & 0x1F) as u8;
                        vec![r << 3, g << 2, b << 3]
                    })
            });
            let alpha_iter = data[(stride_bytes * height as usize)..].iter().copied();
            let rgba_iter = argb_iter.zip(alpha_iter).map(|(rgb, alpha)| {
                let mut pixel = rgb;
                pixel.push(alpha);
                pixel
            });
            rgba_iter.flatten().collect()
        }
        ColorFormat::A1 | ColorFormat::A2 | ColorFormat::A4 | ColorFormat::A8 => {
            let bpp = color_format.get_bpp() as u8;

            let alpha_iter = data.chunks_exact(stride_bytes).flat_map(|row| {
                row.iter()
                    .flat_map(|alpha| {
                        (0u8..8u8 / bpp).flat_map(move |i| {
                            [0, 0, 0, (alpha >> ((8 / bpp - 1 - i) * bpp)) << (8 - bpp)]
                        })
                    })
                    .take((width * ColorFormat::ARGB8888.get_size() as u32) as usize)
            });
            alpha_iter.collect()
        }
        ColorFormat::L8 => {
            let argb_iter = data.chunks_exact(stride_bytes).flat_map(|row| {
                row.chunks_exact(width_bytes)
                    .next()
                    .unwrap()
                    .chunks_exact(1)
                    .map(|chunk| {
                        let l = chunk[0];
                        vec![l, l, l, 0xFF]
                    })
            });
            argb_iter.flatten().collect()
        }
        ColorFormat::I1 | ColorFormat::I2 | ColorFormat::I4 | ColorFormat::I8 => {
            let bpp = color_format.get_bpp() as u8;
            let color_map_size = 1 << bpp;
            let color_map_size_bytes = color_map_size * ColorFormat::ARGB8888.get_size() as usize;
            let color_map = data[0..color_map_size_bytes].to_vec();
            let color_map = rgba8888_from(
                &color_map,
                ColorFormat::ARGB8888,
                color_map_size as u32,
                1,
                ColorFormat::ARGB8888.get_stride_size(color_map_size as u32, 1),
            );

            let alpha_iter = data[color_map_size_bytes..]
                .chunks_exact(stride_bytes)
                .flat_map(|row| {
                    row.iter()
                        .flat_map(|alpha| {
                            let indexes = (0u8..8u8 / bpp)
                                .map(move |i| {
                                    let alpha = *alpha;
                                    (alpha >> ((8 / bpp - 1 - i) * bpp)) & ((1u16 << bpp) - 1) as u8
                                })
                                .collect::<Vec<u8>>();

                            indexes
                                .iter()
                                .copied()
                                .flat_map(|index| {
                                    color_map[index as usize * 4..(index as usize + 1) * 4].to_vec()
                                })
                                .collect::<Vec<u8>>()
                        })
                        .take((width * ColorFormat::ARGB8888.get_size() as u32) as usize)
                });
            alpha_iter.collect()
        }
        _ => {
            unimplemented!()
        }
    }
}
