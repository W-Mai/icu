use mirx::{Color, FillRule, LineCap, LineJoin, Paint, Path, PathCmd, Scene, SceneOp, Transform};

fn fixed_f(v: mirx::Fixed) -> f32 {
    v.to_f32()
}

fn color_hex(c: &Color) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

fn paint_color(paint: &Paint) -> Color {
    let fallback = Color { r: 0, g: 0, b: 0, a: 255 };
    match paint {
        Paint::Color(color) => *color,
        Paint::LinearGradient(gradient) => gradient.stops.first().map(|stop| stop.color).unwrap_or(fallback),
        Paint::RadialGradient(gradient) => gradient.stops.first().map(|stop| stop.color).unwrap_or(fallback),
    }
}

fn opacity_attr(opa: u8) -> String {
    if opa == 255 {
        String::new()
    } else {
        format!(" opacity=\"{:.3}\"", opa as f32 / 255.0)
    }
}

fn transform_attr(tf: &Transform) -> String {
    if *tf == Transform::IDENTITY {
        return String::new();
    }
    format!(
        " transform=\"matrix({} {} {} {} {} {})\"",
        fixed_f(tf.m00),
        fixed_f(tf.m10),
        fixed_f(tf.m01),
        fixed_f(tf.m11),
        fixed_f(tf.tx),
        fixed_f(tf.ty)
    )
}

fn fill_rule_str(r: FillRule) -> &'static str {
    match r {
        FillRule::NonZero => "nonzero",
        FillRule::EvenOdd => "evenodd",
    }
}

fn line_cap_str(c: LineCap) -> &'static str {
    match c {
        LineCap::Butt => "butt",
        LineCap::Round => "round",
        LineCap::Square => "square",
    }
}

fn line_join_str(j: LineJoin) -> &'static str {
    match j {
        LineJoin::Miter => "miter",
        LineJoin::Round => "round",
        LineJoin::Bevel => "bevel",
    }
}

fn path_to_d(path: &Path) -> String {
    let mut d = String::new();
    for cmd in &path.cmds {
        match cmd {
            PathCmd::MoveTo(p) => {
                d.push_str(&format!("M{} {} ", fixed_f(p.x), fixed_f(p.y)));
            }
            PathCmd::LineTo(p) => {
                d.push_str(&format!("L{} {} ", fixed_f(p.x), fixed_f(p.y)));
            }
            PathCmd::QuadTo { ctrl, end } => {
                d.push_str(&format!(
                    "Q{} {} {} {} ",
                    fixed_f(ctrl.x),
                    fixed_f(ctrl.y),
                    fixed_f(end.x),
                    fixed_f(end.y)
                ));
            }
            PathCmd::CubicTo { ctrl1, ctrl2, end } => {
                d.push_str(&format!(
                    "C{} {} {} {} {} {} ",
                    fixed_f(ctrl1.x),
                    fixed_f(ctrl1.y),
                    fixed_f(ctrl2.x),
                    fixed_f(ctrl2.y),
                    fixed_f(end.x),
                    fixed_f(end.y)
                ));
            }
            PathCmd::Close => {
                d.push_str("Z ");
            }
        }
    }
    d.trim_end().to_string()
}

pub fn scene_to_svg(scene: &Scene, width: u32, height: u32) -> String {
    let (w, h) = if width == 0 || height == 0 {
        let (max_w, max_h) = scene_bbox(scene);
        (max_w.max(1), max_h.max(1))
    } else {
        (width, height)
    };
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        w, h, w, h
    );
    for op in &scene.ops {
        match op {
            SceneOp::GroupBegin {
                transform,
                opacity,
                ..
            } => {
                let mut attrs = String::new();
                if let Some(t) = transform {
                    attrs.push_str(&transform_attr(t));
                }
                if let Some(o) = opacity {
                    attrs.push_str(&opacity_attr(*o));
                }
                svg.push_str(&format!("<g{}>", attrs));
            }
            SceneOp::GroupEnd => {
                svg.push_str("</g>");
            }
            SceneOp::FillPath {
                path,
                transform,
                paint,
                opa,
                fill_rule,
            } => {
                let d = path_to_d(path);
                let color = paint_color(paint);
                let mut attrs = format!(
                    " d=\"{}\" fill=\"{}\" fill-opacity=\"{:.3}\" fill-rule=\"{}\"",
                    d,
                    color_hex(&color),
                    *opa as f32 / 255.0,
                    fill_rule_str(*fill_rule)
                );
                if *transform != Transform::IDENTITY {
                    attrs.push_str(&transform_attr(transform));
                }
                svg.push_str(&format!("<path{}/>", attrs));
            }
            SceneOp::StrokePath {
                path,
                transform,
                paint,
                width,
                opa,
                line_cap,
                line_join,
                miter_limit,
                dash: _,
            } => {
                let d = path_to_d(path);
                let color = paint_color(paint);
                let mut attrs = format!(
                    " d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-opacity=\"{:.3}\"",
                    d,
                    color_hex(&color),
                    fixed_f(*width),
                    *opa as f32 / 255.0
                );
                attrs.push_str(&format!(" stroke-linecap=\"{}\"", line_cap_str(*line_cap)));
                attrs.push_str(&format!(" stroke-linejoin=\"{}\"", line_join_str(*line_join)));
                let ml = fixed_f(*miter_limit);
                if (ml - 4.0).abs() > 0.01 {
                    attrs.push_str(&format!(" stroke-miterlimit=\"{}\"", ml));
                }
                if *transform != Transform::IDENTITY {
                    attrs.push_str(&transform_attr(transform));
                }
                svg.push_str(&format!("<path{}/>", attrs));
            }
            SceneOp::FillRect {
                area,
                transform,
                color,
                radius,
                opa,
                ..
            } => {
                let mut attrs = format!(
                    " x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" fill-opacity=\"{:.3}\"",
                    fixed_f(area.x),
                    fixed_f(area.y),
                    fixed_f(area.w),
                    fixed_f(area.h),
                    color_hex(color),
                    *opa as f32 / 255.0
                );
                if *radius != mirx::Fixed::from_int(0) {
                    attrs.push_str(&format!(" rx=\"{}\" ry=\"{}\"", fixed_f(*radius), fixed_f(*radius)));
                }
                if *transform != Transform::IDENTITY {
                    attrs.push_str(&transform_attr(transform));
                }
                svg.push_str(&format!("<rect{}/>", attrs));
            }
            SceneOp::Border {
                area,
                transform,
                color,
                width,
                radius,
                opa,
                ..
            } => {
                let mut attrs = format!(
                    " x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-opacity=\"{:.3}\"",
                    fixed_f(area.x),
                    fixed_f(area.y),
                    fixed_f(area.w),
                    fixed_f(area.h),
                    color_hex(color),
                    fixed_f(*width),
                    *opa as f32 / 255.0
                );
                if *radius != mirx::Fixed::from_int(0) {
                    attrs.push_str(&format!(" rx=\"{}\" ry=\"{}\"", fixed_f(*radius), fixed_f(*radius)));
                }
                if *transform != Transform::IDENTITY {
                    attrs.push_str(&transform_attr(transform));
                }
                svg.push_str(&format!("<rect{}/>", attrs));
            }
            SceneOp::Line {
                p1,
                p2,
                transform,
                color,
                width,
                opa,
                ..
            } => {
                let mut attrs = format!(
                    " x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-opacity=\"{:.3}\"",
                    fixed_f(p1.x),
                    fixed_f(p1.y),
                    fixed_f(p2.x),
                    fixed_f(p2.y),
                    color_hex(color),
                    fixed_f(*width),
                    *opa as f32 / 255.0
                );
                if *transform != Transform::IDENTITY {
                    attrs.push_str(&transform_attr(transform));
                }
                svg.push_str(&format!("<line{}/>", attrs));
            }
            SceneOp::Arc { .. } | SceneOp::Label { .. } | SceneOp::Blit { .. }
            | SceneOp::PushClip { .. } | SceneOp::PopClip => {}
        }
    }
    svg.push_str("</svg>");
    svg
}

fn scene_bbox(scene: &Scene) -> (u32, u32) {
    let mut max_x = 0f32;
    let mut max_y = 0f32;
    for op in &scene.ops {
        match op {
            SceneOp::FillPath { path, .. } | SceneOp::StrokePath { path, .. } => {
                for cmd in &path.cmds {
                    if let Some(p) = cmd_endpoint(cmd) {
                        max_x = max_x.max(p.0);
                        max_y = max_y.max(p.1);
                    }
                }
            }
            SceneOp::FillRect { area, .. } | SceneOp::Border { area, .. } => {
                max_x = max_x.max(fixed_f(area.x) + fixed_f(area.w));
                max_y = max_y.max(fixed_f(area.y) + fixed_f(area.h));
            }
            SceneOp::Line { p1, p2, .. } => {
                max_x = max_x.max(fixed_f(p1.x)).max(fixed_f(p2.x));
                max_y = max_y.max(fixed_f(p1.y)).max(fixed_f(p2.y));
            }
            _ => {}
        }
    }
    (max_x.ceil().max(1.0) as u32, max_y.ceil().max(1.0) as u32)
}

fn cmd_endpoint(cmd: &PathCmd) -> Option<(f32, f32)> {
    match cmd {
        PathCmd::MoveTo(p) | PathCmd::LineTo(p) => Some((fixed_f(p.x), fixed_f(p.y))),
        PathCmd::QuadTo { end, .. } | PathCmd::CubicTo { end, .. } => {
            Some((fixed_f(end.x), fixed_f(end.y)))
        }
        PathCmd::Close => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mirx::{Fixed, Point};

    #[test]
    fn empty_scene_produces_minimal_svg() {
        let scene = Scene { ops: Vec::new() };
        let svg = scene_to_svg(&scene, 10, 10);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("viewBox=\"0 0 10 10\""));
    }

    #[test]
    fn fill_path_emits_path_element() {
        let cmds = vec![
            PathCmd::MoveTo(Point::new(Fixed::from_int(0), Fixed::from_int(0))),
            PathCmd::LineTo(Point::new(Fixed::from_int(10), Fixed::from_int(0))),
            PathCmd::Close,
        ];
        let scene = Scene {
            ops: vec![SceneOp::FillPath {
                path: Path { cmds },
                transform: Transform::IDENTITY,
                paint: Paint::Color(Color { r: 255, g: 0, b: 0, a: 255 }),
                opa: 255,
                fill_rule: FillRule::NonZero,
            }],
        };
        let svg = scene_to_svg(&scene, 20, 20);
        assert!(svg.contains("<path"));
        assert!(svg.contains("d=\"M0 0 L10 0 Z\""));
        assert!(svg.contains("fill=\"#FF0000\""));
        assert!(svg.contains("fill-rule=\"nonzero\""));
    }

    #[test]
    fn fill_rect_emits_rect_element() {
        let scene = Scene {
            ops: vec![SceneOp::FillRect {
                area: mirx::Rect::new(Fixed::from_int(5), Fixed::from_int(5), Fixed::from_int(15), Fixed::from_int(20)),
                transform: Transform::IDENTITY,
                quad: None,
                color: Color { r: 0, g: 128, b: 255, a: 255 },
                radius: Fixed::from_int(4),
                opa: 200,
            }],
        };
        let svg = scene_to_svg(&scene, 40, 40);
        assert!(svg.contains("<rect"));
        assert!(svg.contains("x=\"5\""));
        assert!(svg.contains("width=\"15\""));
        assert!(svg.contains("rx=\"4\""));
        assert!(svg.contains("fill=\"#0080FF\""));
    }

    #[test]
    fn group_emits_g_element() {
        let scene = Scene {
            ops: vec![
                SceneOp::GroupBegin {
                    transform: None,
                    opacity: Some(128),
                    clip: None,
                    mask: None,
                    filter: None,
                    disjoint_hint: false,
                },
                SceneOp::GroupEnd,
            ],
        };
        let svg = scene_to_svg(&scene, 10, 10);
        assert!(svg.contains("<g opacity=\"0.502\">"));
        assert!(svg.contains("</g>"));
    }
}
