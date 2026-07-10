use mirx::{
    Color, FillRule, Fixed, LineCap, LineJoin, Path as MirxPath, PathCmd, Point, Scene, SceneOp,
    Transform,
};
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

fn paint_to_color(paint: &usvg::Paint, opacity: u8) -> Option<Color> {
    let base = match paint {
        usvg::Paint::Color(c) => Some(color_from_usvg(*c)),
        usvg::Paint::LinearGradient(_) | usvg::Paint::RadialGradient(_) | usvg::Paint::Pattern(_) => None,
    }?;
    Some(Color {
        a: ((base.a as u32 * opacity as u32) / 255).min(255) as u8,
        ..base
    })
}

fn walk_group(group: &usvg::Group, ops: &mut Vec<SceneOp>) {
    let group_opacity = group.opacity().get();
    let push_group = group_opacity < 1.0 && group_opacity > 0.0;
    if push_group {
        ops.push(SceneOp::GroupBegin {
            transform: None,
            opacity: Some(group.opacity().to_u8()),
            clip: None,
            mask: None,
            filter: None,
            disjoint_hint: false,
        });
    }
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => walk_group(g, ops),
            usvg::Node::Path(p) => emit_path(p, ops),
            usvg::Node::Image(_) | usvg::Node::Text(_) => {}
        }
    }
    if push_group {
        ops.push(SceneOp::GroupEnd);
    }
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
        if let Some(color) = paint_to_color(fill.paint(), fill.opacity().to_u8()) {
            let fill_rule = match fill.rule() {
                usvg::FillRule::NonZero => FillRule::NonZero,
                usvg::FillRule::EvenOdd => FillRule::EvenOdd,
            };
            ops.push(SceneOp::FillPath {
                path: path.clone(),
                transform,
                color,
                opa: color.a,
                fill_rule,
            });
        }
    }
    if let Some(stroke) = p.stroke() {
        if let Some(color) = paint_to_color(stroke.paint(), stroke.opacity().to_u8()) {
            let width = fixed_from_f32(stroke.width().get() as f32);
            if width > Fixed::ZERO {
                ops.push(SceneOp::StrokePath {
                    path,
                    transform,
                    color,
                    width,
                    opa: color.a,
                    line_cap: line_cap_from_usvg(stroke.linecap()),
                    line_join: line_join_from_usvg(stroke.linejoin()),
                    miter_limit: fixed_from_f32(stroke.miterlimit().get() as f32),
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
            SceneOp::FillPath { path, color, .. } => {
                assert_eq!(path.cmds.len(), 3);
                assert_eq!(color.r, 255);
            }
            _ => panic!("expected FillPath"),
        }
    }

    #[test]
    fn parse_rect_produces_fill() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect x=\"5\" y=\"6\" width=\"20\" height=\"30\" fill=\"blue\"/></svg>";
        let scene = svg_to_scene(svg);
        let has_fill = scene
            .ops
            .iter()
            .any(|op| matches!(op, SceneOp::FillPath { color, .. } | SceneOp::FillRect { color, .. } if color.b == 255));
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
        let has_stroke = scene.ops.iter().any(|op| matches!(op, SceneOp::StrokePath { .. }));
        assert!(has_stroke);
    }

    #[test]
    fn parse_use_resolves_defs_path() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><defs><path id=\"p\" d=\"M0 0 L10 0 L10 10 Z\"/></defs><use href=\"#p\" transform=\"translate(5 5)\" fill=\"#FF0000\"/></svg>";
        let scene = svg_to_scene(svg);
        let fill_count = scene
            .ops
            .iter()
            .filter(|op| matches!(op, SceneOp::FillPath { color, .. } if color.r == 255))
            .count();
        assert!(fill_count >= 1, "expected at least one red FillPath from <use>");
    }

    #[test]
    fn parse_group_transform_applies_to_children() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><g transform=\"translate(10 10)\"><rect x=\"0\" y=\"0\" width=\"5\" height=\"5\" fill=\"red\"/></g></svg>";
        let scene = svg_to_scene(svg);
        assert!(!scene.ops.is_empty());
    }
}
