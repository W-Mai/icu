use icu_lib::endecoder::mirui::scene_render;
use icu_lib::endecoder::svg::Svg;
use icu_lib::endecoder::EnDecoder;
use icu_lib::midata::MiData;

fn main() {
    let svg = std::fs::read_to_string("/tmp/test.svg").unwrap();
    let ed = Svg;
    let midata = ed.decode(svg.into_bytes());
    match &midata {
        MiData::PATH(sd) => {
            println!("ops: {}", sd.scene.ops.len());
            let (w, h) = scene_render::scene_dimensions(&sd.scene).unwrap_or((64, 64));
            println!("dimensions: {}x{}", w, h);
            let img = scene_render::render_scene(&sd.scene, w, h);
            println!("rendered: {}x{}", img.width(), img.height());
            img.save("/tmp/svg_render.png").unwrap();
            let non_empty = img.pixels().filter(|p| p.0[3] > 0).count();
            println!("non-transparent pixels: {}", non_empty);
        }
        other => println!("expected PATH, got {}", other.variant_name()),
    }
}
