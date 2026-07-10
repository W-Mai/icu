use mirx::{
    Color, FillRule, Fixed, GradientStop, GradientUnits, LineCap, LineJoin, LinearGradient,
    Paint as MirxPaint, Path as MirxPath, PathCmd, Point, RadialGradient, ResourceRef, Scene,
    SceneOp, SpreadMode, Transform,
};
use std::borrow::Cow;
use usvg::tiny_skia_path::Path as SkPath;

fn fixed_from_f32(v: f32) -> Fixed {
    Fixed::from_raw((v * 256.0).round() as i32)
}

fn color_from_usvg(c: usvg::Color) -> Color {
    Color {
        r: c.red,
        g: c.green,
        b: c.blue,
        a: 255,
    }
}

fn transform_from_usvg(t: usvg::Transform) -> Transform {
    Transform {
        m00: fixed_from_f32(t.sx as f32),
        m01: fixed_from_f32(t.kx as f32),
        tx: fixed_from_f32(t.tx as f32),
        m10: fixed_from_f32(t.ky as f32),
        m11: fixed_from_f32(t.sy as f32),
        ty: fixed_from_f32(t.ty as f32),
    }
}

fn convert_path(path: &SkPath) -> MirxPath {
    let mut cmds: Vec<PathCmd> = Vec::new();
    for seg in path.segments() {
        match seg {
            usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                cmds.push(PathCmd::MoveTo(Point::new(
                    fixed_from_f32(p.x as f32),
                    fixed_from_f32(p.y as f32),
                )));
            }
            usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                cmds.push(PathCmd::LineTo(Point::new(
                    fixed_from_f32(p.x as f32),
                    fixed_from_f32(p.y as f32),
                )));
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                cmds.push(PathCmd::QuadTo {
                    ctrl: Point::new(fixed_from_f32(p1.x as f32), fixed_from_f32(p1.y as f32)),
                    end: Point::new(fixed_from_f32(p2.x as f32), fixed_from_f32(p2.y as f32)),
                });
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => {
                cmds.push(PathCmd::CubicTo {
                    ctrl1: Point::new(fixed_from_f32(p1.x as f32), fixed_from_f32(p1.y as f32)),
                    ctrl2: Point::new(fixed_from_f32(p2.x as f32), fixed_from_f32(p2.y as f32)),
                    end: Point::new(fixed_from_f32(p3.x as f32), fixed_from_f32(p3.y as f32)),
                });
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                cmds.push(PathCmd::Close);
            }
        }
    }
    MirxPath { cmds }
}

fn line_cap_from_usvg(c: usvg::LineCap) -> LineCap {
    match c {
        usvg::LineCap::Butt => LineCap::Butt,
        usvg::LineCap::Round => LineCap::Round,
        usvg::LineCap::Square => LineCap::Square,
    }
}

fn line_join_from_usvg(j: usvg::LineJoin) -> LineJoin {
    match j {
        usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => LineJoin::Miter,
        usvg::LineJoin::Round => LineJoin::Round,
        usvg::LineJoin::Bevel => LineJoin::Bevel,
    }
}

fn paint_to_mirx(paint: &usvg::Paint, opacity: u8) -> Option<MirxPaint> {
    match paint {
        usvg::Paint::Color(c) => {
            let mut color = color_from_usvg(*c);
            color.a = ((color.a as u32 * opacity as u32) / 255).min(255) as u8;
            Some(MirxPaint::Color(color))
        }
        usvg::Paint::LinearGradient(g) => {
            let stops = gradient_stops_from_usvg(g.stops());
            Some(MirxPaint::LinearGradient(LinearGradient {
                start: point_from_usvg(g.x1(), g.y1()),
                end: point_from_usvg(g.x2(), g.y2()),
                stops: Cow::Owned(stops),
                spread: spread_from_usvg(g.spread_method()),
                units: GradientUnits::ObjectBoundingBox,
                transform: transform_from_usvg(g.transform()),
            }))
        }
        usvg::Paint::RadialGradient(g) => {
            let stops = gradient_stops_from_usvg(g.stops());
            Some(MirxPaint::RadialGradient(RadialGradient {
                center: point_from_usvg(g.cx(), g.cy()),
                radius: fixed_from_f32(g.r().get() as f32),
                focal: point_from_usvg(g.fx(), g.fy()),
                focal_radius: Fixed::ZERO,
                stops: Cow::Owned(stops),
                spread: spread_from_usvg(g.spread_method()),
                units: GradientUnits::ObjectBoundingBox,
                transform: transform_from_usvg(g.transform()),
            }))
        }
        usvg::Paint::Pattern(_) => None,
    }
}

fn point_from_usvg(x: f32, y: f32) -> Point {
    Point::new(fixed_from_f32(x), fixed_from_f32(y))
}

fn gradient_stops_from_usvg(stops: &[usvg::Stop]) -> Vec<GradientStop> {
    stops
        .iter()
        .map(|s| GradientStop {
            offset: fixed_from_f32(s.offset().get() as f32),
            color: color_from_usvg(s.color()),
        })
        .collect()
}

fn spread_from_usvg(s: usvg::SpreadMethod) -> SpreadMode {
    match s {
        usvg::SpreadMethod::Pad => SpreadMode::Pad,
        usvg::SpreadMethod::Reflect => SpreadMode::Reflect,
        usvg::SpreadMethod::Repeat => SpreadMode::Repeat,
    }
}

fn filter_token(filters: &[std::sync::Arc<usvg::filter::Filter>]) -> String {
    let mut parts = Vec::new();
    for f in filters {
        for prim in f.primitives() {
            match prim.kind() {
                usvg::filter::Kind::GaussianBlur(gb) => {
                    parts.push(format!(
                        "blur:{}:{}",
                        gb.std_dev_x().get(),
                        gb.std_dev_y().get()
                    ));
                }
                usvg::filter::Kind::ColorMatrix(cm) => {
                    let kind = match cm.kind() {
                        usvg::filter::ColorMatrixKind::Matrix(_) => "matrix",
                        usvg::filter::ColorMatrixKind::Saturate(_) => "saturate",
                        usvg::filter::ColorMatrixKind::HueRotate(_) => "hueRotate",
                        usvg::filter::ColorMatrixKind::LuminanceToAlpha => "luminance",
                    };
                    parts.push(format!("cm:{}", kind));
                }
                _ => {}
            }
        }
    }
    parts.join(";")
}

fn walk_group(group: &usvg::Group, ops: &mut Vec<SceneOp>) {
    let group_opacity = group.opacity().get();
    let filters = group.filters();
    let has_filter = !filters.is_empty();
    let push_group = (group_opacity < 1.0 && group_opacity > 0.0) || has_filter;
    let filter_ref = if has_filter {
        Some(ResourceRef::Token(filter_token(filters)))
    } else {
        None
    };
    if push_group {
        ops.push(SceneOp::GroupBegin {
            transform: None,
            opacity: if group_opacity < 1.0 && group_opacity > 0.0 {
                Some(group.opacity().to_u8())
            } else {
                None
            },
            clip: None,
            mask: None,
            filter: filter_ref,
            disjoint_hint: false,
        });
    }

    let clip_paths: Vec<Vec<SceneOp>> = group
        .clip_path()
        .map(|cp| collect_clip_ops(cp))
        .into_iter()
        .collect();
    for clip_ops in &clip_paths {
        ops.extend_from_slice(clip_ops);
    }

    for node in group.children() {
        match node {
            usvg::Node::Group(g) => walk_group(g, ops),
            usvg::Node::Path(p) => emit_path(p, ops),
            usvg::Node::Image(img) => {
                emit_image(img, ops);
            }
            usvg::Node::Text(t) => walk_group(t.flattened(), ops),
        }
    }

    for _ in &clip_paths {
        ops.push(SceneOp::PopClip);
    }

    if push_group {
        ops.push(SceneOp::GroupEnd);
    }
}

fn emit_image(img: &usvg::Image, ops: &mut Vec<SceneOp>) {
    let kind = img.kind();
    let raw_data: Option<Vec<u8>> = match kind {
        usvg::ImageKind::PNG(data)
        | usvg::ImageKind::JPEG(data)
        | usvg::ImageKind::GIF(data)
        | usvg::ImageKind::WEBP(data) => Some(data.to_vec()),
        _ => None,
    };
    let raw_data = match raw_data {
        Some(d) => d,
        None => return,
    };

    let rgba = match image::load_from_memory(&raw_data) {
        Ok(dyn_img) => dyn_img.to_rgba8(),
        Err(_) => return,
    };
    let (w, h) = rgba.dimensions();
    if w == 0 || h == 0 {
        return;
    }

    let mut r_sum: u64 = 0;
    let mut g_sum: u64 = 0;
    let mut b_sum: u64 = 0;
    let count = (w as u64) * (h as u64);
    for px in rgba.pixels() {
        r_sum += px.0[0] as u64;
        g_sum += px.0[1] as u64;
        b_sum += px.0[2] as u64;
    }
    let avg = mirx::Color {
        r: (r_sum / count).min(255) as u8,
        g: (g_sum / count).min(255) as u8,
        b: (b_sum / count).min(255) as u8,
        a: 255,
    };

    let abs_tf = img.abs_transform();
    let size = img.size();
    let transform = if abs_tf.is_identity() {
        Transform::IDENTITY
    } else {
        transform_from_usvg(abs_tf)
    };

    let area = mirx::Rect::new(
        Fixed::ZERO,
        Fixed::ZERO,
        fixed_from_f32(size.width() as f32),
        fixed_from_f32(size.height() as f32),
    );

    ops.push(SceneOp::FillRect {
        area,
        transform,
        quad: None,
        color: avg,
        radius: Fixed::ZERO,
        opa: 255,
    });
}

fn collect_clip_ops(clip: &usvg::ClipPath) -> Vec<SceneOp> {
    let mut ops = Vec::new();
    for node in clip.root().children() {
        if let usvg::Node::Path(p) = node {
            let path = convert_path(p.data());
            if path.cmds.is_empty() {
                continue;
            }
            let abs_tf = p.abs_transform();
            let transform = if abs_tf.is_identity() {
                Transform::IDENTITY
            } else {
                transform_from_usvg(abs_tf)
            };
            let fill_rule = match p.fill().map(|f| f.rule()) {
                Some(usvg::FillRule::EvenOdd) => FillRule::EvenOdd,
                _ => FillRule::NonZero,
            };
            ops.push(SceneOp::PushClip {
                path,
                transform,
                fill_rule,
            });
        }
    }
    ops
}

fn emit_path(p: &usvg::Path, ops: &mut Vec<SceneOp>) {
    let abs_tf = p.abs_transform();
    let path = convert_path(p.data());
    if path.cmds.is_empty() {
        return;
    }
    let transform = if abs_tf.is_identity() {
        Transform::IDENTITY
    } else {
        transform_from_usvg(abs_tf)
    };
    if let Some(fill) = p.fill() {
        if let Some(paint) = paint_to_mirx(fill.paint(), fill.opacity().to_u8()) {
            let fill_rule = match fill.rule() {
                usvg::FillRule::NonZero => FillRule::NonZero,
                usvg::FillRule::EvenOdd => FillRule::EvenOdd,
            };
            let opa = fill.opacity().to_u8();
            ops.push(SceneOp::FillPath {
                path: path.clone(),
                transform,
                paint,
                opa,
                fill_rule,
            });
        }
    }
    if let Some(stroke) = p.stroke() {
        if let Some(paint) = paint_to_mirx(stroke.paint(), stroke.opacity().to_u8()) {
            let width = fixed_from_f32(stroke.width().get() as f32);
            if width > Fixed::ZERO {
                ops.push(SceneOp::StrokePath {
                    path,
                    transform,
                    paint,
                    width,
                    opa: stroke.opacity().to_u8(),
                    line_cap: line_cap_from_usvg(stroke.linecap()),
                    line_join: line_join_from_usvg(stroke.linejoin()),
                    miter_limit: fixed_from_f32(stroke.miterlimit().get() as f32),
                    dash: Cow::Owned(
                        stroke
                            .dasharray()
                            .map(|d| d.iter().map(|&v| fixed_from_f32(v)).collect())
                            .unwrap_or_default(),
                    ),
                });
            }
        }
    }
}

pub fn svg_to_scene(data: &[u8]) -> Scene {
    let opt = usvg::Options::default();
    match usvg::Tree::from_data(data, &opt) {
        Ok(tree) => {
            let mut ops: Vec<SceneOp> = Vec::new();
            walk_group(tree.root(), &mut ops);
            Scene { ops }
        }
        Err(_) => Scene { ops: Vec::new() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_svg_returns_empty_scene() {
        let svg = b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"></svg>";
        let scene = svg_to_scene(svg);
        assert!(scene.ops.is_empty());
    }

    #[test]
    fn parse_path_with_move_line_close() {
        let svg = b"<svg><path d=\"M0 0 L10 10 Z\" fill=\"#FF0000\"/></svg>";
        let scene = svg_to_scene(svg);
        assert_eq!(scene.ops.len(), 1);
        match &scene.ops[0] {
            SceneOp::FillPath { path, paint, .. } => {
                assert_eq!(path.cmds.len(), 3);
                assert!(matches!(paint, MirxPaint::Color(color) if color.r == 255));
            }
            _ => panic!("expected FillPath"),
        }
    }

    #[test]
    fn parse_rect_produces_fill() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect x=\"5\" y=\"6\" width=\"20\" height=\"30\" fill=\"blue\"/></svg>";
        let scene = svg_to_scene(svg);
        let has_fill = scene.ops.iter().any(|op| match op {
            SceneOp::FillPath {
                paint: MirxPaint::Color(color),
                ..
            }
            | SceneOp::FillRect { color, .. } => color.b == 255,
            _ => false,
        });
        assert!(has_fill, "expected a blue fill from <rect>");
    }

    #[test]
    fn parse_line_produces_stroke() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><line x1=\"0\" y1=\"0\" x2=\"10\" y2=\"10\" stroke=\"black\" stroke-width=\"2\"/></svg>";
        let scene = svg_to_scene(svg);
        let has_stroke = scene
            .ops
            .iter()
            .any(|op| matches!(op, SceneOp::StrokePath { .. } | SceneOp::Line { .. }));
        assert!(has_stroke, "expected a stroke from <line>");
    }

    #[test]
    fn parse_stroke_only_path_produces_stroke_op() {
        let svg = b"<svg><path d=\"M0 0 L10 10\" fill=\"none\" stroke=\"#3a4552\" stroke-width=\"5\"/></svg>";
        let scene = svg_to_scene(svg);
        let has_stroke = scene
            .ops
            .iter()
            .any(|op| matches!(op, SceneOp::StrokePath { .. }));
        assert!(has_stroke);
    }

    #[test]
    fn parse_use_resolves_defs_path() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><defs><path id=\"p\" d=\"M0 0 L10 0 L10 10 Z\"/></defs><use href=\"#p\" transform=\"translate(5 5)\" fill=\"#FF0000\"/></svg>";
        let scene = svg_to_scene(svg);
        let fill_count = scene
            .ops
            .iter()
            .filter(|op| matches!(op, SceneOp::FillPath { paint: MirxPaint::Color(color), .. } if color.r == 255))
            .count();
        assert!(
            fill_count >= 1,
            "expected at least one red FillPath from <use>"
        );
    }

    #[test]
    fn parse_group_transform_applies_to_children() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><g transform=\"translate(10 10)\"><rect x=\"0\" y=\"0\" width=\"5\" height=\"5\" fill=\"red\"/></g></svg>";
        let scene = svg_to_scene(svg);
        assert!(!scene.ops.is_empty());
    }
}
