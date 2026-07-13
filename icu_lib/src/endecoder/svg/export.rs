use mirx::{
    Color, FillRule, GradientUnits, LineCap, LineJoin, Paint, Path, PathCmd, Scene, SceneOp,
    SpreadMode, Transform,
};

fn fixed_f(v: mirx::Fixed) -> f32 {
    v.to_f32()
}

fn color_hex(c: &Color) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

fn opacity_attr(opa: u8) -> String {
    if opa == 255 {
        String::new()
    } else {
        format!(" opacity=\"{:.3}\"", opa as f32 / 255.0)
    }
}

fn gradient_units_str(units: GradientUnits) -> &'static str {
    match units {
        GradientUnits::UserSpaceOnUse => "userSpaceOnUse",
        GradientUnits::ObjectBoundingBox => "objectBoundingBox",
    }
}

fn spread_method_str(spread: SpreadMode) -> &'static str {
    match spread {
        SpreadMode::Pad => "pad",
        SpreadMode::Reflect => "reflect",
        SpreadMode::Repeat => "repeat",
    }
}

struct SvgDefs {
    gradients: Vec<Paint>,
    clips: Vec<(Path, Transform, FillRule)>,
}

impl SvgDefs {
    fn collect(scene: &Scene) -> Self {
        let mut defs = Self {
            gradients: Vec::new(),
            clips: Vec::new(),
        };
        for op in &scene.ops {
            match op {
                SceneOp::FillPath { paint, .. } | SceneOp::StrokePath { paint, .. } => {
                    defs.add_gradient(paint);
                }
                SceneOp::PushClip {
                    path,
                    transform,
                    fill_rule,
                } => {
                    defs.clips.push((path.clone(), *transform, *fill_rule));
                }
                _ => {}
            }
        }
        defs
    }

    fn add_gradient(&mut self, paint: &Paint) {
        if matches!(paint, Paint::LinearGradient(_) | Paint::RadialGradient(_))
            && !self.gradients.iter().any(|existing| existing == paint)
        {
            self.gradients.push(paint.clone());
        }
    }

    fn paint_ref(&self, paint: &Paint) -> String {
        match paint {
            Paint::Color(color) => color_hex(color),
            Paint::LinearGradient(_) | Paint::RadialGradient(_) => {
                let id = self
                    .gradients
                    .iter()
                    .position(|existing| existing == paint)
                    .unwrap_or(0);
                format!("url(#grad{})", id)
            }
        }
    }

    fn to_svg(&self) -> String {
        if self.gradients.is_empty() && self.clips.is_empty() {
            return String::new();
        }
        let mut out = String::from("<defs>");
        for (id, paint) in self.gradients.iter().enumerate() {
            match paint {
                Paint::Color(_) => {}
                Paint::LinearGradient(gradient) => {
                    let mut attrs = format!(
                        " id=\"grad{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" gradientUnits=\"{}\" spreadMethod=\"{}\"",
                        id,
                        fixed_f(gradient.start.x),
                        fixed_f(gradient.start.y),
                        fixed_f(gradient.end.x),
                        fixed_f(gradient.end.y),
                        gradient_units_str(gradient.units),
                        spread_method_str(gradient.spread)
                    );
                    attrs.push_str(&transform_attr_named(
                        "gradientTransform",
                        &gradient.transform,
                    ));
                    out.push_str(&format!("<linearGradient{}>", attrs));
                    push_gradient_stops(&mut out, &gradient.stops);
                    out.push_str("</linearGradient>");
                }
                Paint::RadialGradient(gradient) => {
                    let mut attrs = format!(
                        " id=\"grad{}\" cx=\"{}\" cy=\"{}\" r=\"{}\" fx=\"{}\" fy=\"{}\" fr=\"{}\" gradientUnits=\"{}\" spreadMethod=\"{}\"",
                        id,
                        fixed_f(gradient.center.x),
                        fixed_f(gradient.center.y),
                        fixed_f(gradient.radius),
                        fixed_f(gradient.focal.x),
                        fixed_f(gradient.focal.y),
                        fixed_f(gradient.focal_radius),
                        gradient_units_str(gradient.units),
                        spread_method_str(gradient.spread)
                    );
                    attrs.push_str(&transform_attr_named(
                        "gradientTransform",
                        &gradient.transform,
                    ));
                    out.push_str(&format!("<radialGradient{}>", attrs));
                    push_gradient_stops(&mut out, &gradient.stops);
                    out.push_str("</radialGradient>");
                }
            }
        }
        for (id, (path, transform, fill_rule)) in self.clips.iter().enumerate() {
            let mut attrs = format!(
                " d=\"{}\" fill-rule=\"{}\"",
                path_to_d(path),
                fill_rule_str(*fill_rule)
            );
            if *transform != Transform::IDENTITY {
                attrs.push_str(&transform_attr(transform));
            }
            out.push_str(&format!(
                "<clipPath id=\"clip{}\"><path{}/></clipPath>",
                id, attrs
            ));
        }
        out.push_str("</defs>");
        out
    }
}

fn transform_attr_named(name: &str, tf: &Transform) -> String {
    if *tf == Transform::IDENTITY {
        return String::new();
    }
    format!(
        " {}=\"matrix({} {} {} {} {} {})\"",
        name,
        fixed_f(tf.m00),
        fixed_f(tf.m10),
        fixed_f(tf.m01),
        fixed_f(tf.m11),
        fixed_f(tf.tx),
        fixed_f(tf.ty)
    )
}

fn push_gradient_stops(out: &mut String, stops: &[mirx::GradientStop]) {
    for stop in stops {
        out.push_str(&format!(
            "<stop offset=\"{}\" stop-color=\"{}\" stop-opacity=\"{:.3}\"/>",
            fixed_f(stop.offset),
            color_hex(&stop.color),
            stop.color.a as f32 / 255.0
        ));
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
    let defs = SvgDefs::collect(scene);
    svg.push_str(&defs.to_svg());
    let mut clip_index = 0usize;
    for op in &scene.ops {
        match op {
            SceneOp::GroupBegin {
                transform, opacity, ..
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
                let mut attrs = format!(
                    " d=\"{}\" fill=\"{}\" fill-opacity=\"{:.3}\" fill-rule=\"{}\"",
                    d,
                    defs.paint_ref(paint),
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
                dash,
            } => {
                let d = path_to_d(path);
                let mut attrs = format!(
                    " d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-opacity=\"{:.3}\"",
                    d,
                    defs.paint_ref(paint),
                    fixed_f(*width),
                    *opa as f32 / 255.0
                );
                attrs.push_str(&format!(" stroke-linecap=\"{}\"", line_cap_str(*line_cap)));
                attrs.push_str(&format!(
                    " stroke-linejoin=\"{}\"",
                    line_join_str(*line_join)
                ));
                let ml = fixed_f(*miter_limit);
                if (ml - 4.0).abs() > 0.01 {
                    attrs.push_str(&format!(" stroke-miterlimit=\"{}\"", ml));
                }
                if !dash.is_empty() {
                    let dash_str: Vec<String> =
                        dash.iter().map(|d| fixed_f(*d).to_string()).collect();
                    attrs.push_str(&format!(" stroke-dasharray=\"{}\"", dash_str.join(",")));
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
                    attrs.push_str(&format!(
                        " rx=\"{}\" ry=\"{}\"",
                        fixed_f(*radius),
                        fixed_f(*radius)
                    ));
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
                    attrs.push_str(&format!(
                        " rx=\"{}\" ry=\"{}\"",
                        fixed_f(*radius),
                        fixed_f(*radius)
                    ));
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
            SceneOp::PushClip { .. } => {
                svg.push_str(&format!("<g clip-path=\"url(#clip{})\">", clip_index));
                clip_index += 1;
            }
            SceneOp::PopClip => {
                svg.push_str("</g>");
            }
            SceneOp::Arc { .. } | SceneOp::Label { .. } | SceneOp::Blit { .. } => {}
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
    use mirx::{Fixed, GradientStop, LinearGradient, Point};

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
                paint: Paint::Color(Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                }),
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
    fn fill_path_emits_linear_gradient_def() {
        let cmds = vec![
            PathCmd::MoveTo(Point::new(Fixed::from_int(0), Fixed::from_int(0))),
            PathCmd::LineTo(Point::new(Fixed::from_int(10), Fixed::from_int(0))),
            PathCmd::LineTo(Point::new(Fixed::from_int(10), Fixed::from_int(10))),
            PathCmd::Close,
        ];
        let scene = Scene {
            ops: vec![SceneOp::FillPath {
                path: Path { cmds },
                transform: Transform::IDENTITY,
                paint: Paint::LinearGradient(LinearGradient {
                    start: Point::new(Fixed::ZERO, Fixed::ZERO),
                    end: Point::new(Fixed::from_int(10), Fixed::ZERO),
                    stops: vec![
                        GradientStop {
                            offset: Fixed::ZERO,
                            color: Color {
                                r: 255,
                                g: 0,
                                b: 0,
                                a: 255,
                            },
                        },
                        GradientStop {
                            offset: Fixed::ONE,
                            color: Color {
                                r: 0,
                                g: 0,
                                b: 255,
                                a: 128,
                            },
                        },
                    ]
                    .into(),
                    spread: SpreadMode::Pad,
                    units: GradientUnits::UserSpaceOnUse,
                    transform: Transform::IDENTITY,
                }),
                opa: 255,
                fill_rule: FillRule::NonZero,
            }],
        };
        let svg = scene_to_svg(&scene, 20, 20);
        assert!(svg.contains("<defs><linearGradient id=\"grad0\""));
        assert!(svg.contains("<stop offset=\"0\" stop-color=\"#FF0000\" stop-opacity=\"1.000\"/>"));
        assert!(svg.contains("<stop offset=\"1\" stop-color=\"#0000FF\" stop-opacity=\"0.502\"/>"));
        assert!(svg.contains("fill=\"url(#grad0)\""));
    }

    #[test]
    fn push_clip_emits_clip_path_and_group() {
        let clip_path = Path {
            cmds: vec![
                PathCmd::MoveTo(Point::new(Fixed::from_int(0), Fixed::from_int(0))),
                PathCmd::LineTo(Point::new(Fixed::from_int(8), Fixed::from_int(0))),
                PathCmd::Close,
            ],
        };
        let scene = Scene {
            ops: vec![
                SceneOp::PushClip {
                    path: clip_path,
                    transform: Transform::IDENTITY,
                    fill_rule: FillRule::EvenOdd,
                },
                SceneOp::PopClip,
            ],
        };
        let svg = scene_to_svg(&scene, 20, 20);
        assert!(svg.contains(
            "<clipPath id=\"clip0\"><path d=\"M0 0 L8 0 Z\" fill-rule=\"evenodd\"/></clipPath>"
        ));
        assert!(svg.contains("<g clip-path=\"url(#clip0)\"></g>"));
    }

    #[test]
    fn fill_rect_emits_rect_element() {
        let scene = Scene {
            ops: vec![SceneOp::FillRect {
                area: mirx::Rect::new(
                    Fixed::from_int(5),
                    Fixed::from_int(5),
                    Fixed::from_int(15),
                    Fixed::from_int(20),
                ),
                transform: Transform::IDENTITY,
                quad: None,
                color: Color {
                    r: 0,
                    g: 128,
                    b: 255,
                    a: 255,
                },
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
