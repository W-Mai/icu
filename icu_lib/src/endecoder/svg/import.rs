use mirx::{Color, FillRule, Fixed, Path, PathCmd, Point, Rect, Scene, SceneOp, Transform};

fn fixed_from_f32(v: f32) -> Fixed {
    Fixed::from_raw((v * 256.0).round() as i32)
}

fn circle_to_path(cx: Fixed, cy: Fixed, rx: Fixed, ry: Fixed) -> Path {
    let k = 0.5523f32;
    let cxf = cx.to_f32();
    let cyf = cy.to_f32();
    let rxf = rx.to_f32();
    let ryf = ry.to_f32();
    let kr = rxf * k;
    let kry = ryf * k;
    let p = |x: f32, y: f32| Point::new(fixed_from_f32(x), fixed_from_f32(y));
    Path {
        cmds: vec![
            PathCmd::MoveTo(p(cxf + rxf, cyf)),
            PathCmd::CubicTo {
                ctrl1: p(cxf + rxf, cyf + kry),
                ctrl2: p(cxf + kr, cyf + ryf),
                end: p(cxf, cyf + ryf),
            },
            PathCmd::CubicTo {
                ctrl1: p(cxf - kr, cyf + ryf),
                ctrl2: p(cxf - rxf, cyf + kry),
                end: p(cxf - rxf, cyf),
            },
            PathCmd::CubicTo {
                ctrl1: p(cxf - rxf, cyf - kry),
                ctrl2: p(cxf - kr, cyf - ryf),
                end: p(cxf, cyf - ryf),
            },
            PathCmd::CubicTo {
                ctrl1: p(cxf + kr, cyf - ryf),
                ctrl2: p(cxf + rxf, cyf - kry),
                end: p(cxf + rxf, cyf),
            },
            PathCmd::Close,
        ],
    }
}

fn parse_color(s: &str) -> Color {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        let hex = hex.trim();
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                Color { r, g, b, a: 255 }
            }
            3 => {
                let r = u8::from_str_radix(&format!("{}{}", &hex[0..1], &hex[0..1]), 16).unwrap_or(0);
                let g = u8::from_str_radix(&format!("{}{}", &hex[1..2], &hex[1..2]), 16).unwrap_or(0);
                let b = u8::from_str_radix(&format!("{}{}", &hex[2..3], &hex[2..3]), 16).unwrap_or(0);
                Color { r, g, b, a: 255 }
            }
            _ => Color { r: 0, g: 0, b: 0, a: 255 },
        }
    } else if s.starts_with("rgb(") {
        let inner = s.trim_start_matches("rgb(").trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            Color {
                r: parts[0].trim().parse().unwrap_or(0),
                g: parts[1].trim().parse().unwrap_or(0),
                b: parts[2].trim().parse().unwrap_or(0),
                a: 255,
            }
        } else {
            Color { r: 0, g: 0, b: 0, a: 255 }
        }
    } else {
        named_color(s)
    }
}

fn named_color(s: &str) -> Color {
    match s.trim().to_lowercase().as_str() {
        "black" => Color { r: 0, g: 0, b: 0, a: 255 },
        "white" => Color { r: 255, g: 255, b: 255, a: 255 },
        "red" => Color { r: 255, g: 0, b: 0, a: 255 },
        "green" => Color { r: 0, g: 128, b: 0, a: 255 },
        "blue" => Color { r: 0, g: 0, b: 255, a: 255 },
        "yellow" => Color { r: 255, g: 255, b: 0, a: 255 },
        "cyan" => Color { r: 0, g: 255, b: 255, a: 255 },
        "magenta" => Color { r: 255, g: 0, b: 255, a: 255 },
        "gray" | "grey" => Color { r: 128, g: 128, b: 128, a: 255 },
        "none" => Color { r: 0, g: 0, b: 0, a: 0 },
        _ => Color { r: 0, g: 0, b: 0, a: 255 },
    }
}

fn parse_opacity(s: &str) -> u8 {
    let f: f32 = s.trim().parse().unwrap_or(1.0);
    (f.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn parse_fixed(s: &str) -> Fixed {
    let f: f32 = s.trim().parse().unwrap_or(0.0);
    fixed_from_f32(f)
}

fn parse_transform(_s: &str) -> Transform {
    Transform::IDENTITY
}

fn parse_fill_rule(s: &str) -> FillRule {
    match s.trim() {
        "evenodd" => FillRule::EvenOdd,
        _ => FillRule::NonZero,
    }
}

fn parse_path_d(d: &str) -> Path {
    let mut cmds: Vec<PathCmd> = Vec::new();
    let mut chars = d.chars().peekable();
    let mut last_cmd: char = '\0';
    let mut last_end = Point::new(Fixed::from_int(0), Fixed::from_int(0));

    fn read_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<f32> {
        while let Some(&c) = chars.peek() {
            if c.is_ascii_whitespace() || c == ',' {
                chars.next();
            } else {
                break;
            }
        }
        let mut s = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
                s.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if s.is_empty() {
            None
        } else {
            s.parse().ok()
        }
    }

    fn read_point(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<(f32, f32)> {
        let x = read_number(chars)?;
        let y = read_number(chars)?;
        Some((x, y))
    }

    while let Some(&c) = chars.peek() {
        if c.is_ascii_whitespace() || c == ',' {
            chars.next();
            continue;
        }
        let cmd = if c.is_ascii_alphabetic() {
            chars.next();
            c
        } else {
            last_cmd.to_ascii_lowercase()
        };
        let relative = cmd.is_lowercase();
        let abs_cmd = cmd.to_ascii_uppercase();
        match abs_cmd {
            'M' => {
                if let Some((x, y)) = read_point(&mut chars) {
                    let p = if relative {
                        Point::new(fixed_from_f32(last_end.x.to_f32() + x), fixed_from_f32(last_end.y.to_f32() + y))
                    } else {
                        Point::new(fixed_from_f32(x), fixed_from_f32(y))
                    };
                    cmds.push(PathCmd::MoveTo(p));
                    last_end = p;
                    last_cmd = if cmd.is_uppercase() { 'L' } else { 'l' };
                }
            }
            'L' => {
                if let Some((x, y)) = read_point(&mut chars) {
                    let p = if relative {
                        Point::new(fixed_from_f32(last_end.x.to_f32() + x), fixed_from_f32(last_end.y.to_f32() + y))
                    } else {
                        Point::new(fixed_from_f32(x), fixed_from_f32(y))
                    };
                    cmds.push(PathCmd::LineTo(p));
                    last_end = p;
                    last_cmd = cmd;
                }
            }
            'Q' => {
                if let Some((x1, y1)) = read_point(&mut chars) {
                    if let Some((x, y)) = read_point(&mut chars) {
                        let ctrl = if relative {
                            Point::new(fixed_from_f32(last_end.x.to_f32() + x1), fixed_from_f32(last_end.y.to_f32() + y1))
                        } else {
                            Point::new(fixed_from_f32(x1), fixed_from_f32(y1))
                        };
                        let end = if relative {
                            Point::new(fixed_from_f32(last_end.x.to_f32() + x), fixed_from_f32(last_end.y.to_f32() + y))
                        } else {
                            Point::new(fixed_from_f32(x), fixed_from_f32(y))
                        };
                        cmds.push(PathCmd::QuadTo { ctrl, end });
                        last_end = end;
                        last_cmd = cmd;
                    }
                }
            }
            'C' => {
                if let Some((x1, y1)) = read_point(&mut chars) {
                    if let Some((x2, y2)) = read_point(&mut chars) {
                        if let Some((x, y)) = read_point(&mut chars) {
                            let ctrl1 = if relative {
                                Point::new(fixed_from_f32(last_end.x.to_f32() + x1), fixed_from_f32(last_end.y.to_f32() + y1))
                            } else {
                                Point::new(fixed_from_f32(x1), fixed_from_f32(y1))
                            };
                            let ctrl2 = if relative {
                                Point::new(fixed_from_f32(last_end.x.to_f32() + x2), fixed_from_f32(last_end.y.to_f32() + y2))
                            } else {
                                Point::new(fixed_from_f32(x2), fixed_from_f32(y2))
                            };
                            let end = if relative {
                                Point::new(fixed_from_f32(last_end.x.to_f32() + x), fixed_from_f32(last_end.y.to_f32() + y))
                            } else {
                                Point::new(fixed_from_f32(x), fixed_from_f32(y))
                            };
                            cmds.push(PathCmd::CubicTo { ctrl1, ctrl2, end });
                            last_end = end;
                            last_cmd = cmd;
                        }
                    }
                }
            }
            'Z' => {
                cmds.push(PathCmd::Close);
                last_cmd = cmd;
            }
            'H' => {
                if let Some(x) = read_number(&mut chars) {
                    let p = if relative {
                        Point::new(fixed_from_f32(last_end.x.to_f32() + x), last_end.y)
                    } else {
                        Point::new(fixed_from_f32(x), last_end.y)
                    };
                    cmds.push(PathCmd::LineTo(p));
                    last_end = p;
                    last_cmd = cmd;
                }
            }
            'V' => {
                if let Some(y) = read_number(&mut chars) {
                    let p = if relative {
                        Point::new(last_end.x, fixed_from_f32(last_end.y.to_f32() + y))
                    } else {
                        Point::new(last_end.x, fixed_from_f32(y))
                    };
                    cmds.push(PathCmd::LineTo(p));
                    last_end = p;
                    last_cmd = cmd;
                }
            }
            _ => {
                break;
            }
        }
    }
    Path { cmds }
}

fn extract_attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{}=\"", name);
    let idx = tag.find(&key)?;
    let rest = &tag[idx + key.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn has_attr(tag: &str, name: &str) -> bool {
    let key = format!("{}=\"", name);
    tag.contains(&key)
}

fn split_tags(svg: &str) -> Vec<(String, bool)> {
    let mut tags = Vec::new();
    let mut chars = svg.chars().peekable();
    let mut cur = String::new();
    let mut in_tag = false;
    while let Some(c) = chars.next() {
        if !in_tag && c == '<' {
            in_tag = true;
            cur.clear();
            cur.push(c);
        } else if in_tag && c == '>' {
            cur.push(c);
            let self_closing = cur.ends_with("/>");
            let body = if self_closing {
                &cur[..cur.len() - 2]
            } else {
                &cur[..cur.len() - 1]
            };
            let body = body.trim_start_matches('<').trim_end();
            tags.push((body.to_string(), self_closing));
            in_tag = false;
            cur.clear();
        } else if in_tag {
            cur.push(c);
        }
    }
    tags
}

pub fn svg_to_scene(data: &[u8]) -> Scene {
    let s = String::from_utf8_lossy(data);
    let tags = split_tags(&s);
    let mut ops: Vec<SceneOp> = Vec::new();
    let mut group_depth = 0u32;

    for (tag, self_closing) in tags {
        if tag.starts_with("?xml") || tag.starts_with("!--") {
            continue;
        }
        if tag.starts_with("svg") {
            continue;
        }
        if tag.starts_with("/g") {
            if group_depth > 0 {
                ops.push(SceneOp::GroupEnd);
                group_depth -= 1;
            }
            continue;
        }
        if tag.starts_with("g") {
            let opacity = extract_attr(&tag, "opacity").map(|s| parse_opacity(&s));
            let transform = extract_attr(&tag, "transform").map(|s| parse_transform(&s));
            ops.push(SceneOp::GroupBegin {
                transform,
                opacity,
                clip: None,
                mask: None,
                filter: None,
                disjoint_hint: false,
            });
            if !self_closing {
                group_depth += 1;
            } else {
                ops.push(SceneOp::GroupEnd);
            }
            continue;
        }
        if tag.starts_with("path") {
            let d = match extract_attr(&tag, "d") {
                Some(d) => d,
                None => continue,
            };
            let path = parse_path_d(&d);
            let fill = extract_attr(&tag, "fill").map(|s| parse_color(&s)).unwrap_or(Color { r: 0, g: 0, b: 0, a: 255 });
            let opa = extract_attr(&tag, "fill-opacity")
                .or_else(|| extract_attr(&tag, "opacity"))
                .map(|s| parse_opacity(&s))
                .unwrap_or(255);
            let fill_rule = extract_attr(&tag, "fill-rule")
                .map(|s| parse_fill_rule(&s))
                .unwrap_or(FillRule::NonZero);
            let transform = extract_attr(&tag, "transform").map(|s| parse_transform(&s));
            ops.push(SceneOp::FillPath {
                path,
                transform: transform.unwrap_or(Transform::IDENTITY),
                color: fill,
                opa,
                fill_rule,
            });
            continue;
        }
        if tag.starts_with("rect") {
            let x = extract_attr(&tag, "x").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let y = extract_attr(&tag, "y").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let w = extract_attr(&tag, "width").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let h = extract_attr(&tag, "height").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let r = extract_attr(&tag, "rx").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let fill = extract_attr(&tag, "fill").map(|s| parse_color(&s)).unwrap_or(Color { r: 0, g: 0, b: 0, a: 255 });
            let opa = extract_attr(&tag, "fill-opacity")
                .or_else(|| extract_attr(&tag, "opacity"))
                .map(|s| parse_opacity(&s))
                .unwrap_or(255);
            let transform = extract_attr(&tag, "transform").map(|s| parse_transform(&s));
            if has_attr(&tag, "stroke") {
                let stroke = extract_attr(&tag, "stroke").map(|s| parse_color(&s)).unwrap_or(Color { r: 0, g: 0, b: 0, a: 255 });
                let sw = extract_attr(&tag, "stroke-width").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(1));
                ops.push(SceneOp::Border {
                    area: Rect::new(x, y, w, h),
                    transform: transform.unwrap_or(Transform::IDENTITY),
                    quad: None,
                    color: stroke,
                    width: sw,
                    radius: r,
                    opa,
                });
            } else {
                ops.push(SceneOp::FillRect {
                    area: Rect::new(x, y, w, h),
                    transform: transform.unwrap_or(Transform::IDENTITY),
                    quad: None,
                    color: fill,
                    radius: r,
                    opa,
                });
            }
            continue;
        }
        if tag.starts_with("line") {
            let x1 = extract_attr(&tag, "x1").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let y1 = extract_attr(&tag, "y1").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let x2 = extract_attr(&tag, "x2").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let y2 = extract_attr(&tag, "y2").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let stroke = extract_attr(&tag, "stroke").map(|s| parse_color(&s)).unwrap_or(Color { r: 0, g: 0, b: 0, a: 255 });
            let sw = extract_attr(&tag, "stroke-width").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(1));
            let opa = extract_attr(&tag, "stroke-opacity")
                .or_else(|| extract_attr(&tag, "opacity"))
                .map(|s| parse_opacity(&s))
                .unwrap_or(255);
            let transform = extract_attr(&tag, "transform").map(|s| parse_transform(&s));
            ops.push(SceneOp::Line {
                p1: Point::new(x1, y1),
                p2: Point::new(x2, y2),
                transform: transform.unwrap_or(Transform::IDENTITY),
                color: stroke,
                width: sw,
                opa,
            });
            continue;
        }
        if tag.starts_with("circle") {
            let cx = extract_attr(&tag, "cx").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let cy = extract_attr(&tag, "cy").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let r = extract_attr(&tag, "r").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let fill = extract_attr(&tag, "fill").map(|s| parse_color(&s)).unwrap_or(Color { r: 0, g: 0, b: 0, a: 255 });
            let opa = extract_attr(&tag, "fill-opacity")
                .or_else(|| extract_attr(&tag, "opacity"))
                .map(|s| parse_opacity(&s))
                .unwrap_or(255);
            let transform = extract_attr(&tag, "transform").map(|s| parse_transform(&s));
            let path = circle_to_path(cx, cy, r, r);
            ops.push(SceneOp::FillPath {
                path,
                transform: transform.unwrap_or(Transform::IDENTITY),
                color: fill,
                opa,
                fill_rule: FillRule::NonZero,
            });
            continue;
        }
        if tag.starts_with("ellipse") {
            let cx = extract_attr(&tag, "cx").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let cy = extract_attr(&tag, "cy").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let rx = extract_attr(&tag, "rx").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let ry = extract_attr(&tag, "ry").map(|s| parse_fixed(&s)).unwrap_or(Fixed::from_int(0));
            let fill = extract_attr(&tag, "fill").map(|s| parse_color(&s)).unwrap_or(Color { r: 0, g: 0, b: 0, a: 255 });
            let opa = extract_attr(&tag, "fill-opacity")
                .or_else(|| extract_attr(&tag, "opacity"))
                .map(|s| parse_opacity(&s))
                .unwrap_or(255);
            let transform = extract_attr(&tag, "transform").map(|s| parse_transform(&s));
            let path = circle_to_path(cx, cy, rx, ry);
            ops.push(SceneOp::FillPath {
                path,
                transform: transform.unwrap_or(Transform::IDENTITY),
                color: fill,
                opa,
                fill_rule: FillRule::NonZero,
            });
            continue;
        }
        if tag.starts_with("polyline") || tag.starts_with("polygon") {
            let closed = tag.starts_with("polygon");
            let points_str = extract_attr(&tag, "points").unwrap_or_default();
            let fill = extract_attr(&tag, "fill").map(|s| parse_color(&s)).unwrap_or(Color { r: 0, g: 0, b: 0, a: 255 });
            let opa = extract_attr(&tag, "fill-opacity")
                .or_else(|| extract_attr(&tag, "opacity"))
                .map(|s| parse_opacity(&s))
                .unwrap_or(255);
            let transform = extract_attr(&tag, "transform").map(|s| parse_transform(&s));
            let nums: Vec<f32> = points_str
                .split(|c: char| c.is_ascii_whitespace() || c == ',')
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse().ok())
                .collect();
            let mut cmds: Vec<PathCmd> = Vec::new();
            let mut i = 0;
            while i + 1 < nums.len() {
                let p = Point::new(fixed_from_f32(nums[i]), fixed_from_f32(nums[i + 1]));
                if cmds.is_empty() {
                    cmds.push(PathCmd::MoveTo(p));
                } else {
                    cmds.push(PathCmd::LineTo(p));
                }
                i += 2;
            }
            if closed {
                cmds.push(PathCmd::Close);
            }
            ops.push(SceneOp::FillPath {
                path: Path { cmds },
                transform: transform.unwrap_or(Transform::IDENTITY),
                color: fill,
                opa,
                fill_rule: FillRule::NonZero,
            });
            continue;
        }
    }
    while group_depth > 0 {
        ops.push(SceneOp::GroupEnd);
        group_depth -= 1;
    }
    Scene { ops }
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
    fn parse_rect_to_fill_rect() {
        let svg = b"<svg><rect x=\"5\" y=\"6\" width=\"20\" height=\"30\" fill=\"blue\"/></svg>";
        let scene = svg_to_scene(svg);
        assert_eq!(scene.ops.len(), 1);
        match &scene.ops[0] {
            SceneOp::FillRect { area, color, .. } => {
                assert_eq!(area.w.to_f32(), 20.0);
                assert_eq!(area.h.to_f32(), 30.0);
                assert_eq!(color.b, 255);
            }
            _ => panic!("expected FillRect"),
        }
    }

    #[test]
    fn parse_group_with_opacity() {
        let svg = b"<svg><g opacity=\"0.5\"><rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" fill=\"black\"/></g></svg>";
        let scene = svg_to_scene(svg);
        assert_eq!(scene.ops.len(), 3);
        match &scene.ops[0] {
            SceneOp::GroupBegin { opacity, .. } => {
                assert_eq!(*opacity, Some(128));
            }
            _ => panic!("expected GroupBegin"),
        }
    }

    #[test]
    fn parse_line() {
        let svg = b"<svg><line x1=\"0\" y1=\"0\" x2=\"10\" y2=\"10\" stroke=\"black\" stroke-width=\"2\"/></svg>";
        let scene = svg_to_scene(svg);
        assert_eq!(scene.ops.len(), 1);
        match &scene.ops[0] {
            SceneOp::Line { p2, width, .. } => {
                assert_eq!(p2.x.to_f32(), 10.0);
                assert_eq!(width.to_f32(), 2.0);
            }
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn round_trip_path_through_svg() {
        use mirx::{Fixed, Point};
        let original = Scene {
            ops: vec![SceneOp::FillPath {
                path: Path {
                    cmds: vec![
                        PathCmd::MoveTo(Point::new(Fixed::from_int(0), Fixed::from_int(0))),
                        PathCmd::LineTo(Point::new(Fixed::from_int(10), Fixed::from_int(0))),
                        PathCmd::LineTo(Point::new(Fixed::from_int(10), Fixed::from_int(10))),
                        PathCmd::Close,
                    ],
                },
                transform: Transform::IDENTITY,
                color: Color { r: 255, g: 0, b: 0, a: 255 },
                opa: 255,
                fill_rule: FillRule::NonZero,
            }],
        };
        let svg = crate::endecoder::svg::export::scene_to_svg(&original, 20, 20);
        let back = svg_to_scene(svg.as_bytes());
        assert_eq!(back.ops.len(), 1);
        match &back.ops[0] {
            SceneOp::FillPath { path, color, .. } => {
                assert_eq!(path.cmds.len(), 4);
                assert_eq!(color.r, 255);
            }
            _ => panic!("expected FillPath"),
        }
    }

    #[test]
    fn parse_circle_to_fill_path() {
        let svg = b"<svg><circle cx=\"50\" cy=\"50\" r=\"20\" fill=\"#00FF00\"/></svg>";
        let scene = svg_to_scene(svg);
        assert_eq!(scene.ops.len(), 1);
        match &scene.ops[0] {
            SceneOp::FillPath { path, color, .. } => {
                assert_eq!(path.cmds.len(), 6);
                assert_eq!(color.g, 255);
            }
            _ => panic!("expected FillPath"),
        }
    }

    #[test]
    fn parse_polygon_uses_close() {
        let svg = b"<svg><polygon points=\"0,0 10,0 10,10\" fill=\"red\"/></svg>";
        let scene = svg_to_scene(svg);
        assert_eq!(scene.ops.len(), 1);
        match &scene.ops[0] {
            SceneOp::FillPath { path, .. } => {
                assert_eq!(path.cmds.len(), 4);
                assert!(matches!(path.cmds.last(), Some(PathCmd::Close)));
            }
            _ => panic!("expected FillPath"),
        }
    }

    #[test]
    fn parse_polyline_open() {
        let svg = b"<svg><polyline points=\"0,0 10,0 10,10\" fill=\"red\" stroke=\"black\" stroke-width=\"1\"/></svg>";
        let scene = svg_to_scene(svg);
        assert!(scene.ops.len() >= 1);
    }
}
