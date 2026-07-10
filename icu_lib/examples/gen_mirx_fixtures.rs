use icu_lib::endecoder::mirui::Mirx;
use icu_lib::endecoder::EnDecoder;
use icu_lib::midata::{MiData, SceneData};
use icu_lib::EncoderParams;
use mirx::{Color, FillRule, Fixed, Font, FontChunkHeader, FontChunkKind, AtlasHeader, GlyphMetric, Paint as MirxPaint, Path, PathCmd, Point, Scene, SceneOp, Transform};

fn main() {
    let cmds = vec![
        PathCmd::MoveTo(Point::new(Fixed::from_int(10), Fixed::from_int(10))),
        PathCmd::LineTo(Point::new(Fixed::from_int(100), Fixed::from_int(10))),
        PathCmd::LineTo(Point::new(Fixed::from_int(100), Fixed::from_int(100))),
        PathCmd::LineTo(Point::new(Fixed::from_int(10), Fixed::from_int(100))),
        PathCmd::Close,
    ];
    let scene = Scene {
        ops: vec![SceneOp::FillPath {
            path: Path { cmds },
            transform: Transform::IDENTITY,
            paint: MirxPaint::Color(Color { r: 255, g: 100, b: 50, a: 255 }),
            opa: 255,
            fill_rule: FillRule::EvenOdd,
        }],
    };
    let ed = Mirx;
    let bytes = ed.encode(&MiData::PATH(SceneData { scene }), EncoderParams::default());
    std::fs::write("/tmp/test_vector.mirx", &bytes).unwrap();
    println!("wrote /tmp/test_vector.mirx ({} bytes)", bytes.len());

    let font = Font {
        chunk_header: FontChunkHeader { kind: FontChunkKind::Sdf, format: 4, size: 24 },
        atlas: AtlasHeader {
            version: mirx::SUPPORTED_VERSION,
            bit_depth: 4,
            _pad0: 0,
            source_size: 4,
            spread: 1,
            glyph_count: 1,
            metric_offset: mirx::HEADER_LEN as u32,
            data_offset: (mirx::HEADER_LEN + mirx::METRIC_LEN) as u32,
            bytes_per_glyph: 8,
            ascender: 3,
            descender: 1,
            line_height: 4,
            _pad1: 0,
        },
        metrics: vec![GlyphMetric { codepoint: 'A' as u32, advance: 4, bearing_x: 0, bearing_y: 3 }],
        data: vec![0xAAu8; 8],
    };
    let bytes = ed.encode(
        &MiData::FONT(icu_lib::midata::FontData::Mirx(font)),
        EncoderParams::default(),
    );
    std::fs::write("/tmp/test_font.mirx", &bytes).unwrap();
    println!("wrote /tmp/test_font.mirx ({} bytes)", bytes.len());
}
