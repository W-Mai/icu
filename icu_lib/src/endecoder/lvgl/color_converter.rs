use crate::endecoder::lvgl::ColorFormat;
use image::imageops;
use image::RgbaImage;
use std::iter;

pub fn rgba8888_to(
    data: &[u8],
    color_format: ColorFormat,
    width: u32,
    height: u32,
    stride: u32,
    dither: bool,
) -> Vec<u8> {
    let stride_bytes = stride as usize;
    let color_bytes = ColorFormat::ARGB8888.get_size() as usize;
    let width_bytes = width as usize * color_bytes;

    match color_format {
        ColorFormat::RGB888 => data
            .chunks_exact(width_bytes)
            .flat_map(|row| {
                row.chunks_exact(color_bytes)
                    .flat_map(|chunk| chunk[0..3].iter().rev().copied())
                    .chain(std::iter::repeat(0))
                    .take(stride_bytes)
            })
            .collect(),
        ColorFormat::ARGB8888
        | ColorFormat::XRGB8888
        | ColorFormat::TrueColor
        | ColorFormat::TrueColorAlpha => data
            .chunks_exact(width_bytes)
            .flat_map(|row| {
                row.chunks_exact(color_bytes)
                    .flat_map(|chunk| {
                        chunk
                            .iter()
                            .rev()
                            .cycle()
                            .skip(1)
                            .take(ColorFormat::ARGB8888.get_size() as usize)
                            .copied()
                    })
                    .chain(std::iter::repeat(0))
                    .take(stride_bytes)
            })
            .collect(),
        ColorFormat::RGB565 => data
            .chunks_exact(width_bytes)
            .flat_map(|row| {
                row.chunks_exact(color_bytes)
                    .flat_map(|chunk| {
                        let r = (chunk[0] >> 3) as u16;
                        let g = (chunk[1] >> 2) as u16;
                        let b = (chunk[2] >> 3) as u16;
                        let rgb = (r << 11) | (g << 5) | b;
                        rgb.to_le_bytes()
                    })
                    .chain(std::iter::repeat(0))
                    .take(stride_bytes)
            })
            .collect(),
        ColorFormat::RGB565A8 => data
            .chunks_exact(width_bytes)
            .flat_map(|row| {
                row.chunks_exact(color_bytes)
                    .flat_map(|chunk| {
                        let r = (chunk[0] >> 3) as u16;
                        let g = (chunk[1] >> 2) as u16;
                        let b = (chunk[2] >> 3) as u16;
                        let rgb = (r << 11) | (g << 5) | b;
                        rgb.to_le_bytes()
                    })
                    .chain(std::iter::repeat(0))
                    .take(stride_bytes)
            })
            .chain(data.chunks_exact(color_bytes).map(|chunk| chunk[3]))
            .collect(),
        ColorFormat::A1 | ColorFormat::A2 | ColorFormat::A4 => {
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
        ColorFormat::A8 => {
            let argb_iter = data.chunks_exact(width_bytes).flat_map(|row| {
                row.iter()
                    .copied()
                    .skip(3)
                    .step_by(color_bytes)
                    .chain(iter::repeat(0))
                    .take(stride_bytes)
            });
            argb_iter.collect()
        }
        ColorFormat::L8 => {
            // (R+R+R+B+G+G+G+G) >> 3
            let argb_iter = data.chunks_exact(width_bytes).flat_map(|row| {
                row.chunks_exact(color_bytes)
                    .map(|chunk| {
                        let r = chunk[0] as u16;
                        let g = chunk[1] as u16;
                        let b = chunk[2] as u16;
                        let a = chunk[3] as u16;
                        (((3 * r + b + 4 * g) >> 3) * a / 0xFF) as u8
                    })
                    .chain(iter::repeat(0))
                    .take(stride_bytes)
            });
            argb_iter.collect()
        }
        ColorFormat::I1 | ColorFormat::I2 | ColorFormat::I4 | ColorFormat::I8 => {
            let bpp = color_format.get_bpp();
            let color_map_size = 1 << bpp;
            let nq = color_quant::NeuQuant::new(30, color_map_size, data);
            let mut data = data.to_vec();

            if dither {
                let mut rgba_image = RgbaImage::from_raw(width, height, data.to_vec()).unwrap();
                imageops::dither(&mut rgba_image, &nq);
                data = rgba_image.clone().into_raw();
            }

            let color_map = rgba8888_to(
                &nq.color_map_rgba(),
                ColorFormat::ARGB8888,
                color_map_size as u32,
                1,
                ColorFormat::ARGB8888.get_stride_size(color_map_size as u32, 1),
                dither,
            );

            let mut indexes_iter = data.chunks(color_bytes).map(|pix| nq.index_of(pix) as u8);

            if color_format == ColorFormat::I8 {
                let indexes = indexes_iter.collect::<Vec<u8>>();
                let argb_iter = indexes
                    .chunks_exact(width as usize)
                    .flat_map(|row| row.iter().chain(iter::repeat(&0)).take(stride_bytes));
                return color_map.iter().chain(argb_iter).copied().collect();
            } else {
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
                color_map.iter().chain(indexes.iter()).copied().collect()
            }
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
        ColorFormat::RGB888 => data
            .chunks_exact(stride_bytes)
            .flat_map(|row| {
                row[..width_bytes]
                    .chunks_exact(color_bytes)
                    .flat_map(|chunk| chunk.iter().rev().chain(iter::once(&0xFFu8)).copied())
            })
            .collect(),
        ColorFormat::ARGB8888
        | ColorFormat::XRGB8888
        | ColorFormat::TrueColor
        | ColorFormat::TrueColorAlpha => data
            .chunks_exact(stride_bytes)
            .flat_map(|row| {
                row[..width_bytes]
                    .chunks_exact(color_bytes)
                    .flat_map(|chunk| {
                        chunk
                            .iter()
                            .rev()
                            .cycle()
                            .skip(1)
                            .take(ColorFormat::ARGB8888.get_size() as usize)
                            .copied()
                    })
            })
            .collect(),
        ColorFormat::RGB565 => data
            .chunks_exact(stride_bytes)
            .flat_map(|row| {
                row[..width_bytes]
                    .chunks_exact(color_bytes)
                    .flat_map(|rgb| {
                        let rgb = u16::from_le_bytes([rgb[0], rgb[1]]);
                        iter::once((((rgb >> 11) & 0x1F) as u8) << 3) // R
                            .chain(iter::once((((rgb >> 5) & 0x3F) as u8) << 2)) // G
                            .chain(iter::once(((rgb & 0x1F) as u8) << 3)) // B
                            .chain(iter::once(0xFFu8))
                    })
            })
            .collect(),
        ColorFormat::RGB565A8 => data
            .chunks_exact(stride_bytes)
            .zip(data[(stride_bytes * height as usize)..].chunks_exact(width as usize))
            .flat_map(|(row_rgb, row_alpha)| {
                row_rgb[..width_bytes]
                    .chunks_exact(color_bytes)
                    .zip(row_alpha)
                    .flat_map(|(rgb, alpha)| {
                        let rgb = u16::from_le_bytes([rgb[0], rgb[1]]);
                        iter::once((((rgb >> 11) & 0x1F) as u8) << 3) // R
                            .chain(iter::once((((rgb >> 5) & 0x3F) as u8) << 2)) // G
                            .chain(iter::once(((rgb & 0x1F) as u8) << 3)) // B
                            .chain(iter::once(*alpha))
                    })
            })
            .collect(),
        ColorFormat::A1 | ColorFormat::A2 | ColorFormat::A4 => {
            let bpp = color_format.get_bpp() as u8;

            let alpha_iter = data.chunks_exact(stride_bytes).flat_map(|row| {
                row.iter()
                    .flat_map(|alpha| {
                        (0u8..8u8 / bpp).flat_map(move |i| {
                            iter::repeat(0).take(3).chain(iter::once(
                                (alpha >> ((8 / bpp - 1 - i) * bpp)) << (8 - bpp),
                            ))
                        })
                    })
                    .take((width * ColorFormat::ARGB8888.get_size() as u32) as usize)
            });
            alpha_iter.collect()
        }
        ColorFormat::A8 => {
            let argb_iter = data.chunks_exact(stride_bytes).flat_map(|row| {
                row.iter()
                    .take(width_bytes)
                    .flat_map(|alpha| iter::repeat(0).take(3).chain(iter::once(*alpha)))
            });
            argb_iter.collect()
        }
        ColorFormat::L8 => {
            let argb_iter = data.chunks_exact(stride_bytes).flat_map(|row| {
                row.iter()
                    .take(width_bytes)
                    .flat_map(|chunk| iter::repeat(*chunk).take(3).chain(iter::once(0xFF)))
            });
            argb_iter.collect()
        }
        ColorFormat::I1 | ColorFormat::I2 | ColorFormat::I4 | ColorFormat::I8 => {
            let bpp = color_format.get_bpp() as u8;
            let color_map_size = 1 << bpp;
            let color_map_size_bytes = color_map_size * ColorFormat::ARGB8888.get_size() as usize;
            let color_map = rgba8888_from(
                &data[0..color_map_size_bytes],
                ColorFormat::ARGB8888,
                color_map_size as u32,
                1,
                ColorFormat::ARGB8888.get_stride_size(color_map_size as u32, 1),
            );

            if color_format == ColorFormat::I8 {
                data[color_map_size_bytes..]
                    .chunks_exact(stride_bytes)
                    .flat_map(|row| {
                        row.iter()
                            .flat_map(|&index| {
                                color_map.iter().skip(index as usize * 4).take(4).copied()
                            })
                            .take((width * ColorFormat::ARGB8888.get_size() as u32) as usize)
                    })
                    .collect()
            } else {
                data[color_map_size_bytes..]
                    .chunks_exact(stride_bytes)
                    .flat_map(|row| {
                        row.iter()
                            .flat_map(|alpha| {
                                (0u8..8u8 / bpp)
                                    .map(move |i| {
                                        let alpha = *alpha;
                                        (alpha >> ((8 / bpp - 1 - i) * bpp))
                                            & ((1u16 << bpp) - 1) as u8
                                    })
                                    .flat_map(|index| {
                                        color_map.iter().skip(index as usize * 4).take(4).copied()
                                    })
                            })
                            .take((width * ColorFormat::ARGB8888.get_size() as u32) as usize)
                    })
                    .collect()
            }
        }
        _ => {
            unimplemented!()
        }
    }
}
