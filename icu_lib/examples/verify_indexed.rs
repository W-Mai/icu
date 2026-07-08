use icu_lib::endecoder::lvgl::LVGL;
use icu_lib::endecoder::EnDecoder;
use icu_lib::midata::MiData;
use icu_lib::postprocess::{IndexHoverOverlay, OverlayStack};

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/img_1.bin".to_string());
    let bytes = std::fs::read(&path).unwrap();
    let ed = LVGL {};
    let midata = ed.decode(bytes);
    match &midata {
        MiData::INDEXED(indexed) => {
            println!(
                "indexed: {}x{} bpp={} palette={} indexes={}",
                indexed.width,
                indexed.height,
                indexed.bpp,
                indexed.palette.len(),
                indexed.indexes.len(),
            );
            println!("palette[0]={:?}", indexed.palette[0]);
            println!("palette[1]={:?}", indexed.palette[1]);
            let mut counts = [0u32; 256];
            for &idx in &indexed.indexes {
                counts[idx as usize] += 1;
            }
            for (i, c) in counts.iter().enumerate().take(indexed.palette.len()) {
                if *c > 0 {
                    println!("  index {} = {} pixels", i, c);
                }
            }

            let mut stack = OverlayStack::new(indexed.rgba.clone());
            stack.push(Box::new(IndexHoverOverlay::new(&indexed, 1)));
            let composited = stack.composite();
            println!("composited: {}x{}", composited.width(), composited.height());
            composited.save("/tmp/indexed_hover.png").unwrap();
            let highlighted = composited
                .pixels()
                .filter(|p| p.0[2] == 0 && p.0[0] > 100 && p.0[1] > 100)
                .count();
            println!("highlighted pixels (yellow tint): {}", highlighted);
        }
        other => println!("expected INDEXED, got {}", other.variant_name()),
    }
}
