use image::RgbaImage;
use mirui::render::backends::sw::SwRenderer;
use mirui::render::canvas::Canvas;
use mirui::render::scene::resolver::SliceResolver;
use mirui::render::texture::{ColorFormat, Texture};
use mirui::types::{Fixed, Rect};

pub fn scene_dimensions(scene: &mirx::Scene) -> Option<(u32, u32)> {
    let mut max_x = Fixed::ZERO;
    let mut max_y = Fixed::ZERO;
    let mut any = false;
    for op in &scene.ops {
        let mirui_op: mirui::render::scene::SceneOp = op.clone().into();
        if let Some(bbox) = mirui::render::scene::bbox::op_bbox(&mirui_op) {
            any = true;
            if bbox.x + bbox.w > max_x {
                max_x = bbox.x + bbox.w;
            }
            if bbox.y + bbox.h > max_y {
                max_y = bbox.y + bbox.h;
            }
        }
    }
    if !any {
        return None;
    }
    let w = max_x.to_int().max(1) as u32;
    let h = max_y.to_int().max(1) as u32;
    Some((w, h))
}

pub fn render_scene(scene: &mirx::Scene, width: u32, height: u32) -> RgbaImage {
    if width == 0 || height == 0 {
        return RgbaImage::new(0, 0);
    }
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    render_scene_into(scene, width, height, &mut buffer);
    RgbaImage::from_raw(width, height, buffer).unwrap_or_else(|| RgbaImage::new(0, 0))
}

pub fn render_scene_into(scene: &mirx::Scene, width: u32, height: u32, buffer: &mut [u8]) {
    let expected = (width * height * 4) as usize;
    if buffer.len() < expected {
        return;
    }
    for b in buffer[..expected].iter_mut() {
        *b = 0;
    }
    let w = width.min(u16::MAX as u32) as u16;
    let h = height.min(u16::MAX as u32) as u16;
    let mut texture = Texture::new(buffer, w, h, ColorFormat::RGBA8888);
    texture.alpha_mode = mirui::render::texture::AlphaMode::Opaque;
    let mut renderer = SwRenderer::new(texture);

    let mirui_scene: mirui::render::scene::Scene = scene.clone().into();
    let fonts: &[(&str, &mirui::render::font::Font)] = &[];
    let textures: &[(&str, &Texture)] = &[];
    let resolver = SliceResolver::new(fonts, textures);

    let clip = Rect::new(
        Fixed::ZERO,
        Fixed::ZERO,
        Fixed::from_int(w as i32),
        Fixed::from_int(h as i32),
    );
    let _ = mirui_scene.replay(&mut renderer, &clip, &resolver);
    renderer.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use mirx::{Color, FillRule, Fixed, Paint, Path, PathCmd, Point, SceneOp, Transform};

    #[test]
    fn render_empty_scene_is_transparent() {
        let scene = mirx::Scene { ops: Vec::new() };
        let img = render_scene(&scene, 4, 4);
        for px in img.pixels() {
            assert_eq!(px.0, [0, 0, 0, 0]);
        }
    }

    #[test]
    fn render_fill_path_produces_pixels() {
        let cmds = vec![
            PathCmd::MoveTo(Point::new(Fixed::from_int(0), Fixed::from_int(0))),
            PathCmd::LineTo(Point::new(Fixed::from_int(16), Fixed::from_int(0))),
            PathCmd::LineTo(Point::new(Fixed::from_int(16), Fixed::from_int(16))),
            PathCmd::LineTo(Point::new(Fixed::from_int(0), Fixed::from_int(16))),
            PathCmd::Close,
        ];
        let path = Path { cmds };
        let scene = mirx::Scene {
            ops: vec![SceneOp::FillPath {
                path,
                transform: Transform::IDENTITY,
                paint: Paint::Color(Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                }),
                opa: 255,
                fill_rule: FillRule::EvenOdd,
            }],
        };
        let img = render_scene(&scene, 16, 16);
        let center = img.get_pixel(8, 8);
        assert_eq!(center.0, [255, 0, 0, 255]);
    }
}
