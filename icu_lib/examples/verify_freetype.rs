use icu_lib::endecoder::mirui::font_render;
use icu_lib::endecoder::EnDecoder;
use icu_lib::endecoder::font::FreeType;
use icu_lib::midata::{FontData, MiData};

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        "/System/Library/Fonts/Supplemental/Arial.ttf".to_string()
    });
    let bytes = std::fs::read(&path).unwrap();
    let ed = FreeType;
    let midata = ed.decode(bytes);
    match &midata {
        MiData::FONT(FontData::FreeType(f)) => {
            println!("family={} style={} glyphs={} parsed={}",
                f.family, f.style, f.glyph_count, f.glyphs.len());
            let img = font_render::render_freetype_glyphs(f);
            println!("glyph grid: {}x{}", img.width(), img.height());
            img.save("/tmp/freetype_glyphs.png").unwrap();
            let non_empty = img.pixels().filter(|p| p.0[3] > 0).count();
            println!("non-transparent pixels: {}", non_empty);
        }
        other => println!("expected FONT FreeType, got {}", other.variant_name()),
    }
}
