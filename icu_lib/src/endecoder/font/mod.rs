use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::{FontData, FreeTypeFontData, FreeTypeGlyph, MiData};
use image::RgbaImage;
use mirx::{Fixed, PathCmd, Point};
use serde_json::json;
use ttf_parser::{name, Face, OutlineBuilder};

pub struct FreeType;

struct OutlineCollector {
    cmds: Vec<PathCmd>,
}

impl OutlineCollector {
    fn new() -> Self {
        Self { cmds: Vec::new() }
    }

    fn map(&self, x: f32, y: f32) -> Point {
        Point::new(Fixed::from_raw((x * 256.0) as i32), Fixed::from_raw((y * 256.0) as i32))
    }
}

impl OutlineBuilder for OutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.cmds.push(PathCmd::MoveTo(self.map(x, y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.cmds.push(PathCmd::LineTo(self.map(x, y)));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.cmds.push(PathCmd::QuadTo {
            ctrl: self.map(x1, y1),
            end: self.map(x, y),
        });
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.cmds.push(PathCmd::CubicTo {
            ctrl1: self.map(x1, y1),
            ctrl2: self.map(x2, y2),
            end: self.map(x, y),
        });
    }
    fn close(&mut self) {
        self.cmds.push(PathCmd::Close);
    }
}

fn decode_name_bytes(n: &ttf_parser::name::Name) -> Option<String> {
    if n.is_unicode() {
        let mut name: Vec<u16> = Vec::new();
        for c in ttf_parser::LazyArray16::<u16>::new(n.name) {
            name.push(c);
        }
        String::from_utf16(&name).ok()
    } else if n.platform_id == ttf_parser::name::PlatformId::Macintosh && n.encoding_id == 0 {
        let mut s = String::with_capacity(n.name.len());
        for &b in n.name {
            s.push(b as char);
        }
        Some(s)
    } else {
        None
    }
}

fn extract_name(face: &Face, name_id: u16) -> String {
    let mut best: Option<String> = None;
    for n in face.names().into_iter() {
        if n.name_id != name_id {
            continue;
        }
        if let Some(s) = decode_name_bytes(&n) {
            if best.is_none() || n.is_unicode() {
                best = Some(s);
                if n.is_unicode() {
                    break;
                }
            }
        }
    }
    best.unwrap_or_default()
}

fn parse_face(face: &Face) -> FreeTypeFontData {
    let family = extract_name(face, name::name_id::FAMILY);
    let style = extract_name(face, name::name_id::SUBFAMILY);
    let units_per_em = face.units_per_em();
    let ascender = face.ascender();
    let descender = face.descender();
    let line_height = face.height();
    let glyph_count = face.number_of_glyphs() as u32;

    let mut glyphs = Vec::new();
    for codepoint in 0u32..=0x10FFFF {
        let ch = match char::from_u32(codepoint) {
            Some(c) => c,
            None => continue,
        };
        let gid = match face.glyph_index(ch) {
            Some(g) => g,
            None => continue,
        };
        let mut collector = OutlineCollector::new();
        let bbox = match face.outline_glyph(gid, &mut collector) {
            Some(b) => b,
            None => continue,
        };
        let advance = face.glyph_hor_advance(gid).unwrap_or(0);
        let side_bearing = face.glyph_hor_side_bearing(gid).unwrap_or(0);
        glyphs.push(FreeTypeGlyph {
            codepoint,
            advance,
            bearing_x: side_bearing,
            bearing_y: 0,
            bbox: (bbox.x_min, bbox.y_min, bbox.x_max, bbox.y_max),
            outline: collector.cmds,
        });
        if glyphs.len() >= 512 {
            break;
        }
    }

    FreeTypeFontData {
        family,
        style,
        units_per_em,
        ascender,
        descender,
        line_height,
        glyph_count,
        glyphs,
    }
}

impl EnDecoder for FreeType {
    fn can_decode(&self, data: &[u8]) -> bool {
        data.len() >= 4
            && matches!(
                &data[..4],
                [0x00, 0x01, 0x00, 0x00] | b"OTTO" | b"ttcf"
            )
    }

    fn encode(&self, _data: &MiData, _params: crate::EncoderParams) -> Vec<u8> {
        Vec::new()
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        let face = match Face::parse(&data, 0) {
            Ok(f) => f,
            Err(_) => return MiData::RGBA(RgbaImage::new(0, 0)),
        };
        let font_data = parse_face(&face);
        MiData::FONT(FontData::FreeType(font_data))
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let face = match Face::parse(data, 0) {
            Ok(f) => f,
            Err(_) => {
                return ImageInfo {
                    width: 0,
                    height: 0,
                    data_size: data.len() as u32,
                    format: "unknown".to_string(),
                    other_info: json!({}),
                }
            }
        };
        let family = extract_name(&face, name::name_id::FAMILY);
        let style = extract_name(&face, name::name_id::SUBFAMILY);
        ImageInfo {
            width: 0,
            height: 0,
            data_size: data.len() as u32,
            format: "ttf".to_string(),
            other_info: json!({
                "layout": "freetype",
                "family": family,
                "style": style,
                "units_per_em": face.units_per_em(),
                "ascender": face.ascender(),
                "descender": face.descender(),
                "line_height": face.height(),
                "glyph_count": face.number_of_glyphs(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static DEJA_FONTS_DIR: &str = "/System/Library/Fonts";

    fn load_test_ttf() -> Option<Vec<u8>> {
        for name in ["DejaVuSans.ttf", "Helvetica.ttc", "Arial.ttf"] {
            let path = format!("{}/{}", DEJA_FONTS_DIR, name);
            if let Ok(data) = std::fs::read(&path) {
                if FreeType.can_decode(&data) {
                    return Some(data);
                }
            }
        }
        let candidates = [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/Library/Fonts/Arial.ttf",
        ];
        for path in candidates {
            if let Ok(data) = std::fs::read(path) {
                if FreeType.can_decode(&data) {
                    return Some(data);
                }
            }
        }
        None
    }

    #[test]
    fn can_decode_ttf_magic() {
        assert!(FreeType.can_decode(&[0x00, 0x01, 0x00, 0x00, 0xff]));
        assert!(FreeType.can_decode(b"OTTOextra"));
        assert!(FreeType.can_decode(b"ttcfextra"));
        assert!(!FreeType.can_decode(b"PNG\r\n"));
        assert!(!FreeType.can_decode(b"MIRX"));
    }

    #[test]
    fn decode_ttf_returns_freetype_font() {
        let data = match load_test_ttf() {
            Some(d) => d,
            None => {
                eprintln!("skip: no test TTF found");
                return;
            }
        };
        let ed = FreeType;
        let midata = ed.decode(data.clone());
        match midata {
            MiData::FONT(FontData::FreeType(f)) => {
                assert!(!f.family.is_empty(), "family should not be empty");
                assert!(f.glyph_count > 0, "should have glyphs");
                assert!(!f.glyphs.is_empty(), "should have parsed glyphs");
                let first = &f.glyphs[0];
                assert!(!first.outline.is_empty(), "glyph should have outline");
            }
            other => panic!("expected FONT FreeType, got {}", other.variant_name()),
        }
    }

    #[test]
    fn info_reports_family_and_glyph_count() {
        let data = match load_test_ttf() {
            Some(d) => d,
            None => {
                eprintln!("skip: no test TTF found");
                return;
            }
        };
        let ed = FreeType;
        let info = ed.info(&data);
        let other = &info.other_info;
        assert_eq!(other.get("layout").and_then(|v| v.as_str()), Some("freetype"));
        let gc = other.get("glyph_count").and_then(|v| v.as_u64());
        assert!(gc.unwrap_or(0) > 0);
    }
}
