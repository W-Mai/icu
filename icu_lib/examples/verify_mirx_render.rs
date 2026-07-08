use icu_lib::endecoder::mirui::{font_render, scene_render, Mirx};
use icu_lib::endecoder::EnDecoder;
use icu_lib::midata::MiData;
use icu_lib::EncoderParams;

fn main() {
    let bytes = std::fs::read("/tmp/test_vector.mirx").unwrap();
    let ed = Mirx;
    let midata = ed.decode(bytes);
    match &midata {
        MiData::PATH(scene_data) => {
            let (w, h) = scene_render::scene_dimensions(&scene_data.scene).unwrap_or((256, 256));
            println!("scene dimensions: {}x{}", w, h);
            let img = scene_render::render_scene(&scene_data.scene, w, h);
            println!("rendered image: {}x{}", img.width(), img.height());
            img.save("/tmp/test_vector_preview.png").unwrap();
            let non_transparent = img.pixels().filter(|p| p.0[3] > 0).count();
            println!("non-transparent pixels: {}", non_transparent);
        }
        other => println!("expected PATH, got {}", other.variant_name()),
    }

    let bytes = std::fs::read("/tmp/test_font.mirx").unwrap();
    let midata = ed.decode(bytes);
    match &midata {
        MiData::FONT(font_data) => {
            let img = font_render::render_font_atlas(&font_data.font);
            println!("atlas image: {}x{}", img.width(), img.height());
            img.save("/tmp/test_font_atlas.png").unwrap();
            let non_zero = img.pixels().filter(|p| p.0[0] > 0).count();
            println!("non-zero pixels: {}", non_zero);

            let text_img = font_render::render_font_text(&font_data.font, "A", 32, 16);
            println!("text image: {}x{}", text_img.width(), text_img.height());
            text_img.save("/tmp/test_font_text.png").unwrap();
        }
        other => println!("expected FONT, got {}", other.variant_name()),
    }
}
